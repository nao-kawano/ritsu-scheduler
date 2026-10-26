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

use rt_client::{MessageType, RtClient};
use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

const NODE_NAME: &str = "planner_node";
const DEFAULT_CID: u16 = 40;
const DEFAULT_PORT: u16 = 10000;
const MAX_CYCLES: usize = 25;
const MIN_DURATION_MS: f64 = 35.0;
const MAX_DURATION_MS: f64 = 45.0;

/// Simple fast Xorshift64 PRNG to avoid external crate dependencies.
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0xdeadbeef } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn range_f64(&mut self, min: f64, max: f64) -> f64 {
        let frac = (self.next_u64() as f64) / (u64::MAX as f64);
        min + frac * (max - min)
    }
}

fn get_scenario_message(cycle: usize) -> &'static str {
    if (1..=7).contains(&cycle) {
        "Cmd: target v=0.50m/s, w=0.00rad/s"
    } else if (8..=14).contains(&cycle) {
        "Obstacle detected! Shift left: v=0.35m/s, w=+0.25rad/s"
    } else if (15..=20).contains(&cycle) {
        "Returning to original path: v=0.40m/s, w=-0.15rad/s"
    } else {
        "Cmd: target v=0.50m/s, w=0.00rad/s"
    }
}

fn main() {
    let mut host = "127.0.0.1".to_string();
    let mut port = DEFAULT_PORT;
    let mut client_id = DEFAULT_CID;
    let run_cycle_sec = 0.2;
    let startup_wait_sec = 60.0;

    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--host" && i + 1 < args.len() {
            host = args[i + 1].clone();
            i += 2;
        } else if args[i] == "--port" && i + 1 < args.len() {
            port = args[i + 1].parse().unwrap_or(DEFAULT_PORT);
            i += 2;
        } else if args[i] == "--client-id" && i + 1 < args.len() {
            client_id = args[i + 1].parse().unwrap_or(DEFAULT_CID);
            i += 2;
        } else {
            i += 1;
        }
    }

    let seed = Instant::now().elapsed().as_nanos() as u64 ^ (client_id as u64);
    let mut rng = SimpleRng::new(seed);

    let mut client = RtClient::new(&host, port, client_id, run_cycle_sec, startup_wait_sec);

    if !client.join() {
        eprintln!("[{NODE_NAME}] Failed to join server");
        return;
    }

    println!("[{NODE_NAME}] Joined server at {host}:{port} (CID: {client_id})");

    let mut proc_count = 0;

    loop {
        let msg = client.wait_next();
        if msg.mtype == MessageType::Start {
            // Gracefully stop on the next START after completing MAX_CYCLES.
            if proc_count >= MAX_CYCLES {
                break;
            }

            let cycle = msg
                .get_extra("cycle")
                .and_then(|s| s.parse::<usize>().ok())
                .map(|c| c + 1)
                .unwrap_or(proc_count + 1);

            let target_ms = rng.range_f64(MIN_DURATION_MS, MAX_DURATION_MS);
            let start_time = Instant::now();
            thread::sleep(Duration::from_secs_f64(target_ms / 1000.0));
            let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;

            let scenario_msg = get_scenario_message(cycle);
            println!(
                "[Cycle {:02}/{}] [{:<14}] {} (elapsed: {:.1}ms)",
                cycle, MAX_CYCLES, NODE_NAME, scenario_msg, elapsed_ms
            );
            let _ = io::stdout().flush();

            let resp = client.notify_done();
            if resp.mtype == MessageType::Error {
                eprintln!("[{NODE_NAME}] Received ERROR on notify_done");
                break;
            }

            proc_count += 1;
        } else if msg.mtype == MessageType::Skip || msg.mtype == MessageType::Late {
            continue;
        } else {
            if proc_count < MAX_CYCLES {
                eprintln!("[{NODE_NAME}] Exiting due to message type: {:?}", msg.mtype);
            }
            break;
        }
    }

    client.exit();

    println!("\n[Process completed] Press Enter key to close...");
    let _ = io::stdout().flush();
    let mut dummy = String::new();
    let _ = io::stdin().read_line(&mut dummy);
}
