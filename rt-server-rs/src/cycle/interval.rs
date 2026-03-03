//!
//! Time-based Cycle Trigger
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
const LOG_TAG: &str = "IntervalTrigger";

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{thread, time};

use super::CycleTrigger;

pub struct IntervalTrigger {
    cycle_ms: u16,
}

impl IntervalTrigger {
    pub fn new(cycle_ms: u16) -> Self {
        IntervalTrigger { cycle_ms }
    }
}

impl CycleTrigger for IntervalTrigger {
    fn on_start(&self) -> Result<(), String> {
        info!("{}: on_start cycle={}ms", LOG_TAG, self.cycle_ms);
        Ok(())
    }

    fn on_shutdown(&self) {
        info!("{}: on_shutdown", LOG_TAG);
    }

    fn wait_next_cycle(&self, stop_flag: &Arc<AtomicBool>) -> bool {
        let start = time::Instant::now();
        let timeout = time::Duration::from_millis(self.cycle_ms as u64);

        while start.elapsed() < timeout {
            // Check stop request.
            if stop_flag.load(Ordering::Relaxed) {
                return false;
            }
            // Sleep for a short duration to remain responsive to stop requests.
            let remaining = timeout
                .checked_sub(start.elapsed())
                .unwrap_or(time::Duration::ZERO);
            thread::sleep(std::cmp::min(remaining, time::Duration::from_millis(10)));
        }
        true
    }
}
