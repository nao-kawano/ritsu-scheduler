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
        cycle_time: 1000,
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

fn main() {
    // setup logger.
    unsafe { std::env::set_var("RUST_LOG", "trace") }; // for debugging.
    env_logger::Builder::from_default_env()
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
    info!("Starting Ritsu server {}", env!("CARGO_PKG_VERSION"));

    // load configuration from file.
    let config = load_config("./config.toml");

    // setup channel between modules.
    let (tx, rx): (Sender<Event>, Receiver<Event>) = mpsc::channel();

    // setup manager.
    let mut event_manager = EventManager::new(config.client_configs);

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
        config.server_config.cycle_time,
    ));
    let mut cycle = CycleGenerator::new(trigger);
    cycle
        .start(tx_cycle)
        .expect("Failed to start cycle generator");

    // install Ctrl+C handler for shutdown.
    let tx_abort = tx.clone();
    ctrlc::set_handler(move || {
        warn!("Got Ctrl+C");
        tx_abort.send(Event::Abort).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    // receive event from thread.
    while let Ok(event) = rx.recv() {
        // process event in manager.
        let result = event_manager.process(event);
        // send response if needed.
        match result {
            Ok(responses) => {
                if responses.len() > 0 {
                    client_connector.send_responses(responses)
                }
            }
            Err(e) => warn!("Error while processing, continue: {}", e),
        }
        // check if exit.
        if event_manager.get_state() == ManagerState::Exited {
            break;
        }
    }

    // stop thread.
    cycle.stop();
    client_connector.stop();
}
