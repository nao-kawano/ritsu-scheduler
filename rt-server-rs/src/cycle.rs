//!
//! Cycle Generator engine and Trigger trait.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
const LOG_TAG: &str = "CycleGen";

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;

use crate::event::Event;

pub mod interval;

/* -------------------------------------------------------------------------- */

/// Trait for cycle triggers.
pub trait CycleTrigger: Send + Sync {
    /// Called when the cycle generator starts.
    fn on_start(&self) -> Result<(), String>;
    /// Called when the cycle generator stops.
    fn on_shutdown(&self);
    /// Waits for the next cycle. Returns true to continue, false to stop.
    fn wait_next_cycle(&self, stop_flag: &Arc<AtomicBool>) -> bool;
}

/* -------------------------------------------------------------------------- */

/// Common engine for cycle generation.
pub struct CycleGenerator {
    trigger: Arc<dyn CycleTrigger>,
    stop_flag: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl CycleGenerator {
    /// Constructor.
    pub fn new(trigger: Box<dyn CycleTrigger>) -> Self {
        CycleGenerator {
            trigger: Arc::from(trigger),
            stop_flag: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        }
    }

    /// Starts the cycle generator thread.
    pub fn start(&mut self, tx_channel: Sender<Event>) -> Result<(), String> {
        info!("{}: starting", LOG_TAG);

        let trigger = Arc::clone(&self.trigger);
        let stop_flag = Arc::clone(&self.stop_flag);

        self.thread_handle = Some(thread::spawn(move || {
            let mut cycle_count: u64 = 0;
            debug!("{}: cycle thread started.", LOG_TAG);
            // Initialize trigger.
            if let Err(e) = trigger.on_start() {
                error!("{}: failed to start trigger: {}", LOG_TAG, e);
                return;
            }
            //
            loop {
                // Check stop request before sending event.
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }

                // Send event.
                if let Err(e) = tx_channel.send(Event::CycleStart(cycle_count)) {
                    error!("{}: failed to send event: {:?}", LOG_TAG, e);
                    break;
                }
                cycle_count += 1;

                // Wait for next cycle.
                if !trigger.wait_next_cycle(&stop_flag) {
                    break;
                }
            }
            // Cleanup.
            trigger.on_shutdown();
            debug!("{}: cycle thread stopped.", LOG_TAG);
        }));

        Ok(())
    }

    /// Stops the cycle generator thread.
    pub fn stop(&mut self) {
        if let Some(h) = self.thread_handle.take() {
            info!("{}: stop requested", LOG_TAG);
            self.stop_flag.store(true, Ordering::Relaxed);
            h.join().unwrap();
            info!("{}: stopped", LOG_TAG);
            self.stop_flag.store(false, Ordering::Relaxed);
        } else {
            warn!("{}: not started", LOG_TAG);
        }
    }
}
