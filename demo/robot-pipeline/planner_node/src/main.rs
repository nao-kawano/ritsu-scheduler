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

use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rt_client::{MessageType, RtClient};

// -----------------------------------------------------------------------------
// Node Identity & Scheduler Constants
// -----------------------------------------------------------------------------
const NODE_NAME: &str = "planner_node";
const HOST: &str = "127.0.0.1";
const PORT: u16 = 7878;
const CLIENT_ID: u16 = 40;
const INTERVAL_SEC: f64 = 0.2;
const STARTUP_WAIT_SEC: f64 = 60.0;

/// Simulated motion planning latency range in milliseconds.
const MIN_DURATION_MS: f64 = 35.0;
const MAX_DURATION_MS: f64 = 45.0;

// -----------------------------------------------------------------------------
// RNG, Scenario & Terminal Helpers
// -----------------------------------------------------------------------------

/// Simple fast Xorshift64 PRNG to avoid external crate dependencies.
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    /// Creates a new PRNG instance with a non-zero fallback seed.
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0xdeadbeef } else { seed },
        }
    }

    /// Generates the next pseudo-random 64-bit integer.
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Generates a pseudo-random floating-point number within `[min, max]`.
    fn range_f64(&mut self, min: f64, max: f64) -> f64 {
        let frac = (self.next_u64() as f64) / (u64::MAX as f64);
        min + frac * (max - min)
    }
}

/// Returns the planning status message for the given 0-indexed cycle.
fn get_scenario_message(running_cycle: usize) -> &'static str {
    if (0..=6).contains(&running_cycle) {
        "Cmd: target v=0.50m/s, w=0.00rad/s"
    } else if (7..=13).contains(&running_cycle) {
        "Obstacle detected! Shift left: v=0.35m/s, w=+0.25rad/s"
    } else if (14..=19).contains(&running_cycle) {
        "Returning to original path: v=0.40m/s, w=-0.15rad/s"
    } else {
        "Cmd: target v=0.50m/s, w=0.00rad/s"
    }
}

/// Blocks until the user presses Enter so terminal logs remain visible.
fn wait_for_enter() {
    println!("\n[Process completed] Press Enter key to close...");
    let _ = io::stdout().flush();
    let mut dummy = String::new();
    let _ = io::stdin().read_line(&mut dummy);
}

// -----------------------------------------------------------------------------
// Main Control Loop
// -----------------------------------------------------------------------------
fn main() {
    // Seed PRNG from current wall-clock time to vary simulated latency across runs.
    let seed = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64)
        ^ (CLIENT_ID as u64);
    let mut rng = SimpleRng::new(seed);

    // Join Ritsu scheduler as client 40 (join barrier after ai_vision_node and odometry_node).
    let mut client = RtClient::new(HOST, PORT, CLIENT_ID, INTERVAL_SEC, STARTUP_WAIT_SEC);
    if !client.join() {
        println!("[{NODE_NAME}] Failed to join server at {HOST}:{PORT}");
        wait_for_enter();
        return;
    }
    println!(
        "[{NODE_NAME}] Joined server at {HOST}:{PORT} (CID: {CLIENT_ID}, interval: {INTERVAL_SEC:.3}s)"
    );

    let mut proc_count = 0;
    loop {
        let _ = io::stdout().flush();
        let msg = client.wait_next();

        match msg.mtype {
            MessageType::Start => {
                let running_cycle = msg
                    .get_extra("cycle")
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(proc_count);
                proc_count += 1;

                let target_ms = rng.range_f64(MIN_DURATION_MS, MAX_DURATION_MS);
                let start_time = Instant::now();
                thread::sleep(Duration::from_secs_f64(target_ms / 1000.0));
                let proc_ms = start_time.elapsed().as_secs_f64() * 1000.0;

                let scenario_msg = get_scenario_message(running_cycle);
                println!(
                    "[Cyc {running_cycle:02}] [{NODE_NAME}] {scenario_msg} proc_ms={proc_ms:.2}"
                );

                let resp = client.notify_done();
                if resp.mtype == MessageType::Error {
                    println!("[{NODE_NAME}] Received ERROR on notify_done, exiting");
                    break;
                }
            }

            MessageType::Skip | MessageType::Late => {
                proc_count += 1;
                let msg_label = if msg.mtype == MessageType::Skip {
                    "SKIP"
                } else {
                    "LATE"
                };
                println!("[{NODE_NAME}] Received {msg_label}, skipping execution for this cycle");
                continue;
            }

            MessageType::Error => {
                let reason = msg.get_extra("reason").map(String::as_str).unwrap_or("N/A");
                let cid = msg.get_extra("cid").map(String::as_str).unwrap_or("N/A");
                println!("[{NODE_NAME}] Exiting on ERROR (reason={reason}, cid={cid})");
                break;
            }

            other => {
                println!("[{NODE_NAME}] Exiting due to unexpected message type: {other:?}");
                break;
            }
        }
    }
    let _ = io::stdout().flush();

    client.exit();
    wait_for_enter();
}
