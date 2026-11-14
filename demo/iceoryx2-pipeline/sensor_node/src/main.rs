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
use std::ptr;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use iceoryx2::port::publisher::Publisher;
use iceoryx2::prelude::{
    ipc, set_log_level_from_env_or, LogLevel, NodeBuilder, NodeName, ServiceName,
};
use rt_client::{MessageType, RtClient};
use shared_types::{
    PayloadHeader, SensorResult, IMAGE_DATA_LEN, IMAGE_HEIGHT, IMAGE_WIDTH,
    QOS_ENABLE_SAFE_OVERFLOW, QOS_HISTORY_SIZE, QOS_SUBSCRIBER_MAX_BORROWED_SAMPLES,
    QOS_SUBSCRIBER_MAX_BUFFER_SIZE, STATUS_INVALID, STATUS_VALID, TOPIC_CAMERA_FRAME,
};

const NODE_NAME: &str = "sensor_node";
const HOST: &str = "127.0.0.1";
const PORT: u16 = 7878;
const CLIENT_ID: u16 = 10;
const INTERVAL_SEC: f64 = 0.1;
const STARTUP_WAIT_SEC: f64 = 60.0;

/// Zero-indexed running cycle at which the sensor outputs an invalid frame.
const INVALID_RUNNING_CYCLE: u64 = 10;

/// Target simulated sensor capture latency in milliseconds.
const TARGET_CAPTURE_MS: u64 = 10;

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

/// Populates `image_buf` with a deterministic grayscale test pattern and paces capture up to
/// `TARGET_CAPTURE_MS`.
fn capture_frame(image_buf: &mut [u8], running_cycle: u64, status: u32, cycle_start: Instant) {
    let fill_value = if status == STATUS_INVALID {
        0xFFu8
    } else {
        ((running_cycle * 17) & 0xFF) as u8
    };
    image_buf.fill(fill_value);
    image_buf[0] = (running_cycle & 0xFF) as u8;
    image_buf[IMAGE_DATA_LEN - 1] = status as u8;

    // Simulate remaining hardware capture latency before publishing.
    if let Some(remaining) =
        Duration::from_millis(TARGET_CAPTURE_MS).checked_sub(cycle_start.elapsed())
    {
        thread::sleep(remaining);
    }
}

/// Loans an uninitialized sample, writes the `SensorResult` fields directly into shared memory,
/// and publishes it.
fn publish_frame(
    publisher: &Publisher<ipc::Service, SensorResult, ()>,
    running_cycle: u64,
    status: u32,
    image_buf: &[u8],
) -> Result<(), String> {
    if image_buf.len() < IMAGE_DATA_LEN {
        return Err(format!(
            "Frame buffer too small: expected at least {IMAGE_DATA_LEN} bytes, got {}",
            image_buf.len()
        ));
    }

    let header = PayloadHeader {
        cycle: running_cycle,
        timestamp_us: unix_timestamp_us(),
        status,
        reserved: 0,
    };

    let mut sample = publisher
        .loan_uninit()
        .map_err(|err| format!("Failed to loan uninitialized sample: {err}"))?;

    // Write each field directly into loaned shared memory to avoid placing a
    // 307,232-byte SensorResult on the stack.
    let payload_ptr = sample.payload_mut().as_mut_ptr();
    // SAFETY: `payload_ptr` points to valid, aligned, uninitialized shared memory
    // loaned for `SensorResult`, `image_buf` is verified to hold at least `IMAGE_DATA_LEN`
    // bytes, and all four fields (`header`, `width`, `height`, `data`) are fully
    // initialized before `assume_init()` is called.
    let initialized = unsafe {
        ptr::addr_of_mut!((*payload_ptr).header).write(header);
        ptr::addr_of_mut!((*payload_ptr).width).write(IMAGE_WIDTH);
        ptr::addr_of_mut!((*payload_ptr).height).write(IMAGE_HEIGHT);
        ptr::copy_nonoverlapping(
            image_buf.as_ptr(),
            ptr::addr_of_mut!((*payload_ptr).data).cast::<u8>(),
            IMAGE_DATA_LEN,
        );
        sample.assume_init()
    };

    initialized
        .send()
        .map_err(|err| format!("Failed to send sample: {err}"))?;

    Ok(())
}

fn main() {
    set_log_level_from_env_or(LogLevel::Error);

    // Initialize iceoryx2 node and CameraFrame publisher with single-latest QoS.
    let node_name = NodeName::new(NODE_NAME).expect("valid iceoryx2 node name");
    let node = NodeBuilder::new()
        .name(&node_name)
        .create::<ipc::Service>()
        .expect("create iceoryx2 node");

    let service_name = ServiceName::new(TOPIC_CAMERA_FRAME).expect("valid iceoryx2 service name");
    let service = node
        .service_builder(&service_name)
        .publish_subscribe::<SensorResult>()
        .subscriber_max_buffer_size(QOS_SUBSCRIBER_MAX_BUFFER_SIZE)
        .enable_safe_overflow(QOS_ENABLE_SAFE_OVERFLOW)
        .subscriber_max_borrowed_samples(QOS_SUBSCRIBER_MAX_BORROWED_SAMPLES)
        .history_size(QOS_HISTORY_SIZE)
        .open_or_create()
        .expect("open or create CameraFrame service");
    let publisher = service
        .publisher_builder()
        .create()
        .expect("create CameraFrame publisher");

    // Allocate the 307,200-byte frame buffer on the heap to avoid large stack allocations.
    let mut image_buf = vec![0u8; IMAGE_DATA_LEN];

    // Join Ritsu real-time scheduler as client 10 (100 ms periodic slot).
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
    loop {
        let _ = io::stdout().flush();
        let msg = client.wait_next();

        match msg.mtype {
            MessageType::Start => {
                let cycle_start = Instant::now();
                let running_cycle = msg
                    .get_extra("cycle")
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(proc_count);
                proc_count += 1;

                let status = if running_cycle == INVALID_RUNNING_CYCLE {
                    STATUS_INVALID
                } else {
                    STATUS_VALID
                };

                capture_frame(&mut image_buf, running_cycle, status, cycle_start);

                let write_start = Instant::now();
                if let Err(err) = publish_frame(&publisher, running_cycle, status, &image_buf) {
                    println!("[{NODE_NAME}] {err}, exiting");
                    break;
                }
                let ipc_write_us = write_start.elapsed().as_secs_f64() * 1_000_000.0;
                let proc_ms = cycle_start.elapsed().as_secs_f64() * 1000.0;

                println!(
                    "[Cyc {running_cycle:02}] [{NODE_NAME}] status={status} ipc_write_us={ipc_write_us:.2} proc_ms={proc_ms:.2}"
                );

                let resp = client.notify_done();
                if resp.mtype == MessageType::Error {
                    println!("[{NODE_NAME}] Received ERROR on notify_done, exiting");
                    break;
                }
            }
            MessageType::Skip | MessageType::Late => {
                proc_count += 1;
                println!(
                    "[{NODE_NAME}] Received {:?}, skipping publish for this cycle",
                    msg.mtype
                );
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
