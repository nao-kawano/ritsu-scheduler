//!
//! Entry point.
//!

extern crate chrono;
extern crate env_logger;
extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

mod client_connector;
mod config;
mod cycle;
mod manager;
mod pgraph;

use std::io::Write;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use dps_message::Message;

use client_connector::ClientConnector;
use config::{ClientConfig, SchedulerConfig, ServerConfig};
use cycle::CycleGenerator;
use manager::{EventManager, ManagerState};

/* -------------------------------------------------------------------------- */

pub enum Event {
    Abort,
    CycleStart(u64),
    ClientMsg(Message),
}

/* -------------------------------------------------------------------------- */

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

fn main() {
    // setup logger.
    unsafe { std::env::set_var("RUST_LOG", "trace") }; // for debugging.
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{:5}] {}",
                chrono::Local::now().format("%Y/%m/%d %H:%M:%S%.6f"),
                record.level(),
                record.args()
            )
        })
        .init();
    info!("Starting dps scheduler {}", env!("CARGO_PKG_VERSION"));

    // load configuration from file.
    // currently, configuration is hardcoded in the code.
    let config = load_sample_config();

    // setup channel between modules.
    let (tx, rx): (Sender<Event>, Receiver<Event>) = mpsc::channel();

    // setup manager.
    let mut event_manager = EventManager::new(config.client_configs);

    // setup client connector.
    let tx_client = tx.clone();
    let mut client_connector = ClientConnector::new(config.server_config.port);
    client_connector.start(tx_client);

    // setup cycle generator.
    let tx_cycle = tx.clone();
    let mut cycle = CycleGenerator::new(config.server_config.cycle_time);
    cycle.start(tx_cycle);

    // install Ctrl+C handler for shutdown.
    let tx_abort = tx.clone();
    ctrlc::set_handler(move || {
        warn!("Got Ctrl+C");
        tx_abort.send(Event::Abort).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    // receive event from thread.
    loop {
        // receive event.
        let event = rx.recv().unwrap();
        // process event in manager.
        let result = event_manager.process(event);
        // send response if needed.
        match result {
            Ok(responses) => client_connector.send_responses(responses),
            Err(e) => warn!("Error while processing, continue: {}", e),
        }
        // check if exit.
        if event_manager.get_state() == ManagerState::Exitted {
            break;
        }
    }

    // stop thread.
    cycle.stop();
    client_connector.stop();
}
