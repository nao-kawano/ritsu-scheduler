//!
//! Entry point.
//!

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

mod cycle;

use cycle::CycleGenerator;

/* -------------------------------------------------------------------------- */

pub enum Event {
    Abort,
    CycleStart(u64),
}

/* -------------------------------------------------------------------------- */

fn main() {
    let (tx, rx): (Sender<Event>, Receiver<Event>) = mpsc::channel();

    // setup manager. load specs and create manager.

    // start thread.
    let tx_cycle = tx.clone();
    let mut cycle = CycleGenerator::new(1000);
    cycle.start(tx_cycle);

    // install Ctrl+C handler for shutdown.
    let tx_abort = tx.clone();
    ctrlc::set_handler(move || {
        tx_abort.send(Event::Abort).unwrap();
    })
    .expect("Error setting Ctrl-C handler");

    // receive event from thread.
    loop {
        // event processing.
        let event = rx.recv().unwrap();
        match event {
            Event::Abort => break,
            Event::CycleStart(cycle_number) => print!("CycleStart: {}\n", cycle_number),
        };
        // send response if needed.
    }

    // stop thread.
    cycle.stop();
}
