//!
//! Generator generates frame trigger
//!

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::{thread, time};

use crate::Event;

/* -------------------------------------------------------------------------- */

pub struct PeriodicTriggerGenerator {
    cycle_ms: u16,
    stop_flag: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl PeriodicTriggerGenerator {
    pub fn new(cycle_ms: u16) -> Self {
        PeriodicTriggerGenerator {
            cycle_ms,
            stop_flag: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        }
    }

    pub fn start(&mut self, tx_channel: Sender<Event>) {
        // setup thread data and launch thread.
        let cycle_ms = self.cycle_ms as u64;
        let stop_flag = Arc::clone(&(self.stop_flag));
        self.thread_handle = Some(thread::spawn(move || {
            let mut frame_counter: u64 = 0;
            println!("Generator: start cycle={}ms", cycle_ms);
            loop {
                // check stop request.
                if stop_flag.load(Ordering::Relaxed) == true {
                    println!("Generator: stop request detected, exitting");
                    break;
                }
                // send event.
                _ = tx_channel.send(Event::FrameStart(frame_counter));
                frame_counter += 1;
                // wait next.
                thread::sleep(time::Duration::from_millis(cycle_ms));
            }
        }));
    }

    pub fn stop(&mut self) {
        if let Some(h) = self.thread_handle.take() {
            println!("Generator: stop requested");
            self.stop_flag.store(true, Ordering::Relaxed);
            h.join().unwrap();
            println!("Generator: stopped");
            self.stop_flag.store(false, Ordering::Relaxed);
        } else {
            println!("Generator: not started");
        }
    }
}
