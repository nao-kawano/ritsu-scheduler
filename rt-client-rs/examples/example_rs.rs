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

use clap::Parser;
use rt_client::RtClient;
use rt_message::MessageType;
use std::thread;
use std::time::Duration;

#[cfg(target_os = "windows")]
use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};

/// Command line arguments for example_rs.
#[derive(Parser, Debug)]
#[command(version, about = "Ritsu Rust client example")]
struct Args {
    /// Server hostname or IP address
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Server port number
    #[arg(long, default_value_t = 7878)]
    port: u16,

    /// Client identifier
    #[arg(long, default_value_t = 1)]
    client_id: u16,

    /// Cycle duration in seconds
    #[arg(long, default_value_t = 2.0)]
    run_cycle_sec: f64,

    /// Timeout for startup phase in seconds
    #[arg(long, default_value_t = 60.0)]
    startup_wait_sec: f64,

    /// Simulated workload processing time in seconds
    #[arg(long, default_value_t = 0.4)]
    proc_time_sec: f64,

    /// Maximum number of process iterations before exit
    #[arg(long, default_value_t = 5)]
    proc_count: i32,
}

/// Helper function to output timestamped log messages (HH:MM:SS.mmm).
fn log_msg(message: &str) {
    let now = chrono::Local::now();
    println!("{} {}", now.format("%H:%M:%S%.3f"), message);
}

fn main() {
    let args = Args::parse();

    let mut client = RtClient::new(
        &args.host,
        args.port,
        args.client_id,
        args.run_cycle_sec,
        args.startup_wait_sec,
    );

    if !client.join() {
        log_msg("failed to join, exit");
        std::process::exit(-1);
    }

    log_msg("client joined");

    let mut proc_count = 0;
    loop {
        let msg = client.wait_next();
        let cycle_val = msg.get_extra("cycle").map(|s| s.as_str()).unwrap_or("N/A");
        log_msg(&format!(
            "wait_next done: {}, cycle={}, count={}",
            msg.mtype.as_str(),
            cycle_val,
            proc_count
        ));

        if msg.mtype == MessageType::Start {
            log_msg(&format!(
                "got START, do some process with {:.3} sec ...",
                args.proc_time_sec
            ));

            #[cfg(target_os = "windows")]
            unsafe {
                let _ = timeBeginPeriod(1);
            }

            thread::sleep(Duration::from_secs_f64(args.proc_time_sec));

            #[cfg(target_os = "windows")]
            unsafe {
                let _ = timeEndPeriod(1);
            }

            let msg_done = client.notify_done();
            if msg_done.mtype == MessageType::Error {
                log_msg("failed to notify done or got ERROR, exit");
                break;
            }
        } else if msg.mtype == MessageType::Skip {
            log_msg("got SKIP, retry");
        } else if msg.mtype == MessageType::Late {
            log_msg("got LATE, retry");
        } else {
            let reason_val = msg.get_extra("reason").map(|s| s.as_str()).unwrap_or("N/A");
            log_msg(&format!(
                "got ERROR, going to exit (reason={}, cid={})",
                reason_val, msg.cid
            ));
            break;
        }

        proc_count += 1;
        if proc_count >= args.proc_count {
            log_msg("completed, going to exit");
            break;
        }
    }

    client.exit();
}
