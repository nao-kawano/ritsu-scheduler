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

use iceoryx2::port::subscriber::Subscriber;
use iceoryx2::prelude::{
    ipc, set_log_level_from_env_or, LogLevel, NodeBuilder, NodeName, ServiceName,
};
use iceoryx2::sample::Sample;
use rt_client::{MessageType, RtClient};
use shared_types::{
    VisionResult, QOS_ENABLE_SAFE_OVERFLOW, QOS_HISTORY_SIZE, QOS_SUBSCRIBER_MAX_BORROWED_SAMPLES,
    QOS_SUBSCRIBER_MAX_BUFFER_SIZE, STATUS_VALID, TOPIC_VISION_RESULT,
};

const NODE_NAME: &str = "actuator_node";
const HOST: &str = "127.0.0.1";
const PORT: u16 = 7878;
const CLIENT_ID: u16 = 30;
const INTERVAL_SEC: f64 = 0.1;
const STARTUP_WAIT_SEC: f64 = 60.0;

/// Target simulated actuation duration in milliseconds.
const TARGET_PROC_MS: u64 = 15;

/// Number of consecutive `SKIP` messages required to trigger an emergency stop.
const ESTOP_SKIP_THRESHOLD: u32 = 2;

/// Returns the current OS wall-clock timestamp in microseconds since the Unix epoch.
fn unix_timestamp_us() -> u64 {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX)
}

/// Blocks until the user presses Enter so terminal logs remain visible.
fn wait_for_enter() {
    println!("\n[Process completed] Press Enter key to close...");
    let _ = io::stdout().flush();
    let mut dummy = String::new();
    let _ = io::stdin().read_line(&mut dummy);
}

/// Drains all queued samples and returns the newest zero-copy `VisionResult` borrow.
fn drain_latest_vision(
    subscriber: &Subscriber<ipc::Service, VisionResult, ()>,
) -> Option<Sample<ipc::Service, VisionResult, ()>> {
    let mut latest = None;
    while subscriber.has_samples().unwrap_or(false) {
        // Release the prior borrow before the next receive (`max_borrowed_samples = 1`).
        latest = None;
        match subscriber.receive() {
            Ok(Some(sample)) => latest = Some(sample),
            Ok(None) => break,
            Err(err) => {
                println!("[{NODE_NAME}] [WARN] Failed to receive sample: {err}");
                break;
            }
        }
    }
    latest
}

/// Evaluates vision freshness and status and formats a human-readable actuation summary.
fn build_action_summary(result: Option<&VisionResult>, running_cycle: u64) -> String {
    let Some(vision) = result else {
        return "CRUISE(FALLBACK) speed=100% (reason=missing, src_cyc=none)".to_string();
    };

    let src_cyc = vision.header.cycle;
    if src_cyc < running_cycle {
        return format!("CRUISE(FALLBACK) speed=100% (reason=stale, src_cyc={src_cyc})");
    }

    if vision.header.status != STATUS_VALID {
        return format!(
            "CRUISE(FALLBACK) speed=100% (reason=status={}, src_cyc={src_cyc})",
            vision.header.status
        );
    }

    format!(
        "CRUISE speed=100% detected={} dist={:.2}m angle={:.2}deg conf={:.2} src_cyc={src_cyc}",
        vision.detected, vision.distance_m, vision.angle_deg, vision.confidence
    )
}

/// Evaluates vision freshness and status, sleeps up to `TARGET_PROC_MS`, and returns a summary.
fn drive_actuator(
    result: Option<&VisionResult>,
    running_cycle: u64,
    cycle_start: Instant,
) -> String {
    let summary = build_action_summary(result, running_cycle);

    // Maintain constant ~15 ms actuation duration across normal and fallback paths.
    if let Some(remaining) =
        Duration::from_millis(TARGET_PROC_MS).checked_sub(cycle_start.elapsed())
    {
        thread::sleep(remaining);
    }

    summary
}

fn main() {
    set_log_level_from_env_or(LogLevel::Error);

    // Initialize iceoryx2 node and VisionResult subscriber with single-latest QoS.
    let node_name = NodeName::new(NODE_NAME).expect("valid iceoryx2 node name");
    let node = NodeBuilder::new()
        .name(&node_name)
        .create::<ipc::Service>()
        .expect("create iceoryx2 node");

    let service_name = ServiceName::new(TOPIC_VISION_RESULT).expect("valid iceoryx2 service name");
    let service = node
        .service_builder(&service_name)
        .publish_subscribe::<VisionResult>()
        .subscriber_max_buffer_size(QOS_SUBSCRIBER_MAX_BUFFER_SIZE)
        .enable_safe_overflow(QOS_ENABLE_SAFE_OVERFLOW)
        .subscriber_max_borrowed_samples(QOS_SUBSCRIBER_MAX_BORROWED_SAMPLES)
        .history_size(QOS_HISTORY_SIZE)
        .open_or_create()
        .expect("open or create VisionResult service");
    let subscriber = service
        .subscriber_builder()
        .create()
        .expect("create VisionResult subscriber");

    // Join Ritsu real-time scheduler as client 30 (100 ms periodic slot).
    let mut client = RtClient::new(HOST, PORT, CLIENT_ID, INTERVAL_SEC, STARTUP_WAIT_SEC);
    if !client.join() {
        println!("[{NODE_NAME}] Failed to join server at {HOST}:{PORT}");
        wait_for_enter();
        return;
    }
    println!(
        "[{NODE_NAME}] Joined server at {HOST}:{PORT} (CID: {CLIENT_ID}, interval: {INTERVAL_SEC:.3}s)"
    );

    let mut proc_count: u64 = 0;
    let mut consecutive_skips: u32 = 0;
    loop {
        let _ = io::stdout().flush();
        let msg = client.wait_next();

        match msg.mtype {
            MessageType::Start => {
                consecutive_skips = 0;
                let cycle_start = Instant::now();
                let running_cycle = msg
                    .get_extra("cycle")
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(proc_count);
                proc_count += 1;

                let read_start = Instant::now();
                let sample = drain_latest_vision(&subscriber);
                let ipc_read_us = read_start.elapsed().as_secs_f64() * 1_000_000.0;

                let now_us = unix_timestamp_us();
                let e2e_us = sample
                    .as_deref()
                    .map(|r| now_us.saturating_sub(r.header.timestamp_us))
                    .unwrap_or(0);

                let action_summary = drive_actuator(sample.as_deref(), running_cycle, cycle_start);
                // Explicitly release the zero-copy borrow after actuation finishes.
                drop(sample);

                let proc_ms = cycle_start.elapsed().as_secs_f64() * 1000.0;
                println!(
                    "[Cyc {running_cycle:02}] [{NODE_NAME}] {action_summary} ipc_read_us={ipc_read_us:.2} e2e_us={e2e_us} proc_ms={proc_ms:.2}"
                );

                let resp = client.notify_done();
                if resp.mtype == MessageType::Error {
                    println!("[{NODE_NAME}] Received ERROR on notify_done, exiting");
                    break;
                }
            }
            MessageType::Skip => {
                proc_count += 1;
                consecutive_skips += 1;
                if consecutive_skips >= ESTOP_SKIP_THRESHOLD {
                    println!(
                        "[{NODE_NAME}] [CRITICAL] {ESTOP_SKIP_THRESHOLD} consecutive SKIPs detected! Triggering E-STOP (speed 0%)"
                    );
                    break;
                }
                println!(
                    "[{NODE_NAME}] [WARN] Preceding node delayed! Braking mode engaged (speed 50%)"
                );
                continue;
            }
            MessageType::Late => {
                proc_count += 1;
                println!("[{NODE_NAME}] Received Late, skipping actuation for this cycle");
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
