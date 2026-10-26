// Copyright 2026 Naoyuki Kawano
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// =============================================================================
//!
//! Entry point.
//!

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use rt_config::SchedulerConfig;
#[cfg(feature = "perf-log")]
use rt_message::MessageType;

mod clients;
mod cycle;
mod event;
mod logging;
mod manager;

use clients::{ClientConnector, udp::UdpTransport};
use cycle::{CycleGenerator, interval::IntervalTrigger};
use event::Event;
use logging::LogConfig;
use manager::{EventManager, ManagerState};

use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
#[cfg(feature = "perf-log")]
use std::time;

/* -------------------------------------------------------------------------- */

/// Command line arguments for Ritsu server.
#[derive(Parser, Debug)]
#[command(version, about = "Ritsu server")]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value_t = String::from("./config.toml"))]
    config: String,

    /// Console log level (off, error, warn, info, debug, trace)
    #[arg(long, default_value_t = String::new())]
    log_level: String,

    /// Directory for log files (enables file logging)
    #[arg(long, default_value_t = String::new())]
    log_file_dir: String,

    /// File log level (off, error, warn, info, debug, trace)
    #[arg(long, default_value_t = String::new())]
    log_file_level: String,

    /// Disable file logging
    #[arg(long, default_value_t = false)]
    no_log_file: bool,
}

/* -------------------------------------------------------------------------- */

/// Resolves application-specific CLI arguments and configuration file into a generic LogConfig.
fn resolve_log_config(args: &Args, config: &SchedulerConfig) -> LogConfig {
    let server_cfg = &config.server_config;

    // Resolve console log level (CLI > config file > default "info")
    let console_level_str = if !args.log_level.is_empty() {
        &args.log_level
    } else if !server_cfg.log_level.is_empty() {
        &server_cfg.log_level
    } else {
        "info"
    };
    let console_level = logging::parse_level(console_level_str).unwrap_or_else(|e| {
        eprintln!("Invalid console log level: {}", e);
        std::process::exit(1);
    });

    // Resolve file logging directory and target path
    let file_dir = if args.no_log_file {
        None
    } else if !args.log_file_dir.is_empty() {
        // CLI --log-file-dir is resolved relative to current working directory
        Some(PathBuf::from(&args.log_file_dir))
    } else if !server_cfg.log_file_dir.is_empty() {
        let p = Path::new(&server_cfg.log_file_dir);
        if p.is_absolute() {
            Some(p.to_path_buf())
        } else {
            // Config file log_file_dir is resolved relative to configuration file location
            let config_dir = Path::new(&args.config).parent().unwrap_or(Path::new(""));
            Some(config_dir.join(p))
        }
    } else {
        None
    };

    // Resolve file log level
    let file_level = if file_dir.is_some() {
        let file_level_str = if !args.log_file_level.is_empty() {
            &args.log_file_level
        } else if !server_cfg.log_file_level.is_empty() {
            &server_cfg.log_file_level
        } else {
            "debug"
        };
        logging::parse_level(file_level_str).unwrap_or_else(|e| {
            eprintln!("Invalid file log level: {}", e);
            std::process::exit(1);
        })
    } else {
        log::LevelFilter::Off
    };

    LogConfig {
        console_level,
        file_dir,
        file_basename: String::from("rt-server"),
        file_level,
        max_backups: 5,
    }
}

/* -------------------------------------------------------------------------- */

/// Reads and parses the configuration file into SchedulerConfig.
/// Returns both the parsed configuration and the raw file content.
fn read_config(path: &str) -> (SchedulerConfig, String) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read config file {}: {}", path, e);
            std::process::exit(1);
        }
    };

    let config: SchedulerConfig = match toml::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to parse config file {}: {}", path, e);
            std::process::exit(1);
        }
    };

    (config, content)
}

/* -------------------------------------------------------------------------- */

/// Helper for performance measurement.
/// This will be zero-cost when the "perf-log" feature is disabled.
#[cfg(feature = "perf-log")]
struct PerfMetrics {
    mtype: MessageType,
    cid: u16,
    // measurement points.
    time_recv: Option<time::Instant>,
    time_start_proc: Option<time::Instant>,
    time_start_send: Option<time::Instant>,
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
                mtype: MessageType::Ready, // dummy
                cid: 0,                    // dummy
                time_recv: None,
                time_start_proc: None,
                time_start_send: None,
            }
        }
    }
    fn mark_proc_start(&mut self) {
        if self.time_recv.is_some() {
            self.time_start_proc = Some(time::Instant::now());
        }
    }
    fn mark_send_start(&mut self) {
        if self.time_recv.is_some() {
            self.time_start_send = Some(time::Instant::now());
        }
    }
    fn finish(self) {
        if let Some(time_recv) = self.time_recv {
            let time_finish = time::Instant::now();
            let time_start_proc = self.time_start_proc.unwrap_or(time::Instant::now());
            let time_start_send = self.time_start_send.unwrap_or(time::Instant::now());
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
    // Parse command line arguments first to handle --help/--version options or syntax errors
    // without printing any log initialization messages.
    let args = Args::parse();

    // Read and parse configuration file before logger setup.
    let (config, raw_config) = read_config(&args.config);

    // Resolve logging configuration and initialize asynchronous logger.
    let log_cfg = resolve_log_config(&args, &config);
    let _logger_handle = logging::init(&log_cfg).unwrap_or_else(|e| {
        eprintln!("Failed to initialize logger: {}", e);
        std::process::exit(1);
    });

    info!("# Starting Ritsu server v{}", env!("CARGO_PKG_VERSION"));
    info!("- Config file: {}", args.config);
    info!("- Log level: {}", log_cfg.console_level_display());
    if let Some(ref dir) = log_cfg.file_dir {
        let log_path = dir.join(format!("{}.log", log_cfg.file_basename));
        info!("- Log file: {}", log_path.display());
        info!("- Log file level: {}", log_cfg.file_level_display());
    } else {
        info!("- Log file: disabled");
    }
    info!("----------------------------------------");

    // Output raw configuration with <CONFIG> prefix for each line after logger initialization.
    for line in raw_config.lines() {
        info!("<CONFIG> {}", line);
    }

    // Validate configuration.
    if let Err(errors) = config.validate() {
        for (cid, errs) in errors {
            for err in errs {
                error!("[ClientConfig CID:{:03}] {}", cid, err);
            }
        }
        error!(
            "Configuration validation failed for file {}. Please check your configuration.",
            args.config
        );
        std::process::exit(1);
    }

    // derive execution rules.
    let rules = config.get_client_rules();

    // Collect and format expected client IDs before client_configs is moved.
    let mut expected_cids: Vec<u16> = config
        .client_configs
        .iter()
        .map(|c| c.client_id)
        .collect();
    expected_cids.sort_unstable();
    let cids_str = expected_cids
        .iter()
        .map(|cid| format!("{:03}", cid))
        .collect::<Vec<_>>()
        .join(", ");

    // setup channel between modules.
    let (tx, rx): (mpsc::Sender<Event>, mpsc::Receiver<Event>) = mpsc::channel();

    // setup manager.
    let mut event_manager = EventManager::new(
        config.client_configs,
        rules,
        config.server_config.stats_interval_cycle,
    );

    info!("Waiting for clients: CID [{}]...", cids_str);

    // setup client connector.
    let tx_client = tx.clone();
    let transport = Box::new(UdpTransport::new(config.server_config.port));
    let mut client_connector = ClientConnector::new(transport);
    client_connector
        .start(tx_client)
        .expect("Failed to start client connector");

    // setup cycle generator.
    let tx_cycle = tx.clone();
    let trigger = Box::new(IntervalTrigger::new(config.server_config.cycle_time_ms));
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
