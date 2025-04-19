//!
//! Entry point.
//!

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

mod generator;

use generator::PeriodicTriggerGenerator;

/* -------------------------------------------------------------------------- */

pub enum Event {
    Abort,
    FrameStart(u64),
}

/* -------------------------------------------------------------------------- */

fn main() {
    let (tx, rx): (Sender<Event>, Receiver<Event>) = mpsc::channel();

    // setup manager. load specs and create manager.

    // start thread.
    let tx_cycle = tx.clone();
    let mut trigger = PeriodicTriggerGenerator::new(1000);
    trigger.start(tx_cycle);

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
            Event::FrameStart(frame_number) => print!("FrameStart: {}\n", frame_number),
        };
        // send response if needed.
    }

    // stop thread.
    trigger.stop();
}
