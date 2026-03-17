//!
//! Entry point.
//!

extern crate chrono;
extern crate env_logger;
extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

mod clients;
mod config;
mod cycle;
mod event;
mod manager;

use std::fs;
use std::io::Write;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use clients::{ClientConnector, udp::UdpTransport};
use config::{ClientConfig, SchedulerConfig, ServerConfig};
use cycle::CycleGenerator;
use event::Event;
use manager::{EventManager, ManagerState};

/* -------------------------------------------------------------------------- */

#[allow(dead_code)]
fn load_sample_config() -> SchedulerConfig {
    let server_config = ServerConfig {
        port: 7878,
        cycle_time_ms: 50,
        stats_interval_cycle: 0,
    };
    #[rustfmt::skip]
    let client_configs = vec![
        ClientConfig::new(0, 2, 0, vec![]).unwrap(),
        ClientConfig::new(1, 2, 0, vec![]).unwrap(),
        ClientConfig::new(10, 2, 0, vec![0]).unwrap(),
        ClientConfig::new(11, 2, 0, vec![0, 1]).unwrap(),
        ClientConfig::new(20, 2, 1, vec![10, 11]).unwrap(),
        ClientConfig::new(2, 2, 1, vec![]).unwrap(),
    ];
    SchedulerConfig {
        server_config,
        client_configs,
    }
}

fn load_config(path: &str) -> SchedulerConfig {
    let r = fs::read_to_string(path);
    let Ok(content) = r else {
        panic!("failed to read config file {}, {}", path, r.unwrap_err());
    };

    let r: Result<SchedulerConfig, toml::de::Error> = toml::from_str(&content);
    let Ok(config) = r else {
        panic!("failed to parse config file {}, {}", path, r.unwrap_err());
    };

    return config;
}

/* -------------------------------------------------------------------------- */

/// Helper for performance measurement.
/// This will be zero-cost when the "perf-log" feature is disabled.
#[cfg(feature = "perf-log")]
struct PerfMetrics {
    mtype: rt_message::MessageType,
    cid: u16,
    // measurement points.
    time_recv: Option<std::time::Instant>,
    time_start_proc: Option<std::time::Instant>,
    time_start_send: Option<std::time::Instant>,
}

#[cfg(feature = "perf-log")]
impl PerfMetrics {
    fn new(event: &Event) -> Self {
        if let Event::ClientMsg(msg, timestamp) = event {
            Self {
                mtype: msg.mtype,
                cid: msg.cid,
                time_recv: Some(*timestamp),
                time_start_proc: None,
                time_start_send: None,
            }
        } else {
            Self {
                mtype: rt_message::MessageType::Ready, // dummy
                cid: 0,                                // dummy
                time_recv: None,
                time_start_proc: None,
                time_start_send: None,
            }
        }
    }
    fn mark_proc_start(&mut self) {
        if self.time_recv.is_some() {
            self.time_start_proc = Some(std::time::Instant::now());
        }
    }
    fn mark_send_start(&mut self) {
        if self.time_recv.is_some() {
            self.time_start_send = Some(std::time::Instant::now());
        }
    }
    fn finish(self) {
        if let Some(time_recv) = self.time_recv {
            let time_finish = std::time::Instant::now();
            let time_start_proc = self.time_start_proc.unwrap_or(std::time::Instant::now());
            let time_start_send = self.time_start_send.unwrap_or(std::time::Instant::now());
            let elapsed_q_wait = time_start_proc.saturating_duration_since(time_recv);
            let elapsed_proc = time_start_send.saturating_duration_since(time_start_proc);
            let elapsed_send = time_finish.saturating_duration_since(time_start_send);
            let elapsed_total = time_finish.saturating_duration_since(time_recv);
            info!(
                "Perf Detail: CID:{:03} {:<5} | Total: {:>5}us (QWait: {:>5}us, Proc: {:>5}us, Send: {:>5}us)",
                self.cid,
                format!("{:?}", self.mtype),
                elapsed_total.as_micros(),
                elapsed_q_wait.as_micros(),
                elapsed_proc.as_micros(),
                elapsed_send.as_micros()
            );
        }
    }
}

#[cfg(not(feature = "perf-log"))]
struct PerfMetrics;

#[cfg(not(feature = "perf-log"))]
impl PerfMetrics {
    #[inline(always)]
    fn new(_event: &Event) -> Self {
        Self
    }
    #[inline(always)]
    fn mark_proc_start(&mut self) {
        // do nothing.
    }
    #[inline(always)]
    fn mark_send_start(&mut self) {
        // do nothing.
    }
    #[inline(always)]
    fn finish(self) {
        // do nothing.
    }
}

/* -------------------------------------------------------------------------- */

fn main() {
    // setup logger.
    let env = env_logger::Env::default().default_filter_or("info");
    env_logger::Builder::from_env(env)
        .format(|buf, record| {
            let target = record.target();
            let module = target.split("::").last().unwrap_or(target);
            writeln!(
                buf,
                "{} [{:5}] {} - {}",
                chrono::Local::now().format("%Y/%m/%d %H:%M:%S%.6f"),
                record.level(),
                module,
                record.args()
            )
        })
        .init();
    info!("# Starting Ritsu server v{}", env!("CARGO_PKG_VERSION"));
    info!("----------------------------------------");

    // load configuration from file.
    let config = load_config("./config.toml");

    // setup channel between modules.
    let (tx, rx): (Sender<Event>, Receiver<Event>) = mpsc::channel();

    // setup manager.
    let mut event_manager = EventManager::new(
        config.client_configs,
        config.server_config.stats_interval_cycle,
    );

    // setup client connector.
    let tx_client = tx.clone();
    let transport = Box::new(UdpTransport::new(config.server_config.port));
    let mut client_connector = ClientConnector::new(transport);
    client_connector
        .start(tx_client)
        .expect("Failed to start client connector");

    // setup cycle generator.
    let tx_cycle = tx.clone();
    let trigger = Box::new(cycle::interval::IntervalTrigger::new(
        config.server_config.cycle_time_ms,
    ));
    let mut cycle = CycleGenerator::new(trigger);
    cycle
        .start(tx_cycle)
        .expect("Failed to start cycle generator");

    // install Ctrl+C handler for shutdown.
    let tx_abort = tx.clone();
    ctrlc::set_handler(move || {
        warn!("abort requested");
        tx_abort.send(Event::Abort).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    // receive event from thread.
    while let Ok(event) = rx.recv() {
        // create metrics for performance measurement.
        let mut perf = PerfMetrics::new(&event);

        // process event in manager.
        perf.mark_proc_start();
        let result = event_manager.process(event);

        // send response if needed.
        perf.mark_send_start();
        match result {
            Ok(responses) => {
                if responses.len() > 0 {
                    client_connector.send_responses(responses)
                }
            }
            Err(e) => warn!("processing error: {}", e),
        }

        // log performance.
        perf.finish();

        // check if exit.
        if event_manager.get_state() == ManagerState::Exited {
            break;
        }
    }

    // stop thread.
    cycle.stop();
    client_connector.stop();

    info!("----------------------------------------");
    info!("# Ritsu server stopped, bye!");
}
