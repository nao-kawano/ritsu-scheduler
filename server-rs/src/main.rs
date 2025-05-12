//!
//! Entry point.
//!

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

mod config;
mod cycle;

use config::{ClientConfig, SchedulerConfig, ServerConfig, TriggerType};
use cycle::CycleGenerator;

/* -------------------------------------------------------------------------- */

pub enum Event {
    Abort,
    CycleStart(u64),
}

/* -------------------------------------------------------------------------- */

fn load_sample_config() -> SchedulerConfig {
    let server_config = ServerConfig {
        port: 7878,
        cycle_time: 1000,
    };
    let client_configs = vec![
        ClientConfig {
            client_id: 0,
            trigger_type: TriggerType::Cycle(1),
            cycle_offset: 0,
        },
        ClientConfig {
            client_id: 1,
            trigger_type: TriggerType::Depends { clients: vec![0] },
            cycle_offset: 0,
        },
        ClientConfig {
            client_id: 2,
            trigger_type: TriggerType::Depends { clients: vec![0] },
            cycle_offset: 0,
        },
        ClientConfig {
            client_id: 3,
            trigger_type: TriggerType::Depends {
                clients: vec![1, 2],
            },
            cycle_offset: 0,
        },
        ClientConfig {
            client_id: 10,
            trigger_type: TriggerType::Cycle(2),
            cycle_offset: 1,
        },
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

    // setup client connector.

    // setup cycle generator.
    let tx_cycle = tx.clone();
    let mut cycle = CycleGenerator::new(config.server_config.cycle_time);
    cycle.start(tx_cycle);

    // install Ctrl+C handler for shutdown.
    let tx_abort = tx.clone();
    ctrlc::set_handler(move || {
        tx_abort.send(Event::Abort).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    // receive event from thread.
    loop {
        // receive event.
        let event = rx.recv().unwrap();
        match event {
            Event::Abort => break,
            Event::CycleStart(cycle_number) => print!("CycleStart: {}\n", cycle_number),
        };
        // process event in manager.

        // send response if needed.
    }

    // stop thread.
    cycle.stop();
}
