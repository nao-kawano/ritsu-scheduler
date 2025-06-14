//!
//! Time-based Cycle Generator
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::{thread, time};

use crate::Event;

/* -------------------------------------------------------------------------- */

pub struct CycleGenerator {
    cycle_ms: u16,
    stop_flag: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl CycleGenerator {
    pub fn new(cycle_ms: u16) -> Self {
        CycleGenerator {
            cycle_ms,
            stop_flag: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        }
    }

    pub fn start(&mut self, tx_channel: Sender<Event>) {
        // setup thread data and launch thread.
        let cycle_ms = self.cycle_ms as u64;
        let stop_flag = Arc::clone(&(self.stop_flag));
        info!("CycleGen: start cycle={}ms", cycle_ms);
        self.thread_handle = Some(thread::spawn(move || {
            let mut cycle_count: u64 = 0;
            debug!("CycleGen: cycle thread started.");
            loop {
                // check stop request.
                if stop_flag.load(Ordering::Relaxed) == true {
                    info!("CycleGen: stop request detected, exitting");
                    break;
                }
                // send event.
                _ = tx_channel.send(Event::CycleStart(cycle_count));
                cycle_count += 1;
                // wait next.
                thread::sleep(time::Duration::from_millis(cycle_ms));
            }
            debug!("CycleGen: cycle thread stopped.");
        }));
    }

    pub fn stop(&mut self) {
        if let Some(h) = self.thread_handle.take() {
            info!("CycleGen: stop requested");
            self.stop_flag.store(true, Ordering::Relaxed);
            h.join().unwrap();
            info!("CycleGen: stopped");
            self.stop_flag.store(false, Ordering::Relaxed);
        } else {
            warn!("CycleGen: not started");
        }
    }
}
