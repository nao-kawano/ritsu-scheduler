//!
//! Entry point.
//!

mod client_connector;
mod config;
mod cycle;
mod manager;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use dps_message::Message;

use client_connector::ClientConnector;
use config::{ClientConfig, SchedulerConfig, ServerConfig, TriggerType};
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
    let client_configs = vec![
        ClientConfig::new(0, TriggerType::Cycle(1), 0).unwrap(),
        ClientConfig::new(1, TriggerType::Depends { clients: vec![0] }, 0).unwrap(),
        ClientConfig::new(2, TriggerType::Depends { clients: vec![0] }, 0).unwrap(),
        ClientConfig::new(
            3,
            TriggerType::Depends {
                clients: vec![1, 2],
            },
            0,
        )
        .unwrap(),
        ClientConfig::new(5, TriggerType::Cycle(2), 1).unwrap(),
    ];
    SchedulerConfig {
        server_config,
        client_configs,
    }
}

fn main() {
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
        print!("Got Ctrl+C\n");
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
        if let Ok(result) = result {
            client_connector.send_responses(result);
        } else {
            print!("Error while processing, continue\n");
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
