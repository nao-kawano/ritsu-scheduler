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
//! Time-based Cycle Trigger
//!

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use super::CycleTrigger;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time;

/* -------------------------------------------------------------------------- */

pub struct IntervalTrigger {
    cycle_ms: u32,
}

impl IntervalTrigger {
    pub fn new(cycle_ms: u32) -> Self {
        IntervalTrigger { cycle_ms }
    }
}

impl CycleTrigger for IntervalTrigger {
    fn on_start(&self) -> Result<(), String> {
        info!("on_start cycle={}ms", self.cycle_ms);
        Ok(())
    }

    fn on_shutdown(&self) {
        info!("on_shutdown");
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
            let remaining = match timeout.checked_sub(start.elapsed()) {
                Some(rem) => rem,
                None => return true,
            };
            thread::sleep(std::cmp::min(remaining, time::Duration::from_millis(10)));
        }
        true
    }
}
