# Ritsu Client for Rust

## 1. Overview

`rt-client` is a lightweight Rust client library for the Ritsu process scheduler.

- **Minimal Dependencies**: Lightweight footprint, building only on standard UDP sockets and minimal dependencies (`log`, `rt-message`, and `windows` for multimedia timers on Windows).
- **Safe & Ergonomic Rust API**: Idiomatic Rust types (`RtClient`, `MessageType`, `Message`) without unsafe blocks in user-facing code.
- **Deterministic & Synchronous**: Orchestrates process execution through explicit `wait_next()` and `notify_done()` cycles.
- **Cross-Platform**: Works identically across Linux and Windows (with automatic 1ms timer precision handling).

## 2. Installation / Setup

You can add `rt-client` to your Cargo project (`Cargo.toml`) via a local path or directly via Git.

### Local Path Dependency

If you have cloned the `ritsu-scheduler` repository locally:

```toml
[dependencies]
rt-client = { path = "path/to/ritsu-scheduler/rt-client-rs" }
```

### Direct Git Dependency

You can also depend on the repository directly from GitHub:

```toml
[dependencies]
rt-client = { git = "https://github.com/nao-kawano/ritsu-scheduler.git" }
```

Cargo automatically discovers and resolves the workspace package `rt-client` and its internal message format dependency `rt-message`.

## 3. Quick Start

Here is a minimal example demonstrating how to connect to the scheduler server, participate in the deterministic scheduling loop, and exit cleanly:

```rust
use rt_client::{MessageType, RtClient};
use std::thread;
use std::time::Duration;

fn main() {
    // Initialize client config (host, port, client_id, run_cycle_sec, startup_wait_sec)
    let mut client = RtClient::new("127.0.0.1", 7878, 1, 2.0, 60.0);

    // Join the scheduler server
    if !client.join() {
        eprintln!("Failed to join server");
        std::process::exit(1);
    }

    println!("Successfully joined the server!");

    // Main execution loop
    let max_cycles = 5;
    for cycle in 0..max_cycles {
        // Wait for the next scheduling event
        let msg = client.wait_next();

        match msg.mtype {
            MessageType::Start => {
                println!("[{cycle}] Got START, executing task...");
                // Simulate or execute actual task logic here
                thread::sleep(Duration::from_millis(400));
                // Notify task completion to the scheduler
                let done_resp = client.notify_done();
                if done_resp.mtype == MessageType::Error {
                    eprintln!("[{cycle}] Failed to notify done or received ERROR");
                    break;
                }
            }
            MessageType::Skip => {
                println!("[{cycle}] Got SKIP (previous deadline overrun), waiting for next cycle...");
            }
            MessageType::Late => {
                println!("[{cycle}] Got LATE, waiting for next cycle...");
            }
            _ => {
                let reason = msg
                    .get_extra("reason")
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");
                eprintln!(
                    "[{cycle}] Received error or unexpected response: {:?} (reason: {reason})",
                    msg.mtype
                );
                break;
            }
        }
    }

    // Exit cleanly and close socket resources
    client.exit();
    println!("Exited cleanly.");
}
```

## 4. Running the Example

`rt-client-rs` includes a standalone example runner (`examples/example_rs.rs`) for quick connection testing and verification without writing extra code.

### From the Repository Root

```bash
# Display help and available options
cargo run --package rt-client --example example_rs -- --help

# Run the example client
cargo run --package rt-client --example example_rs -- --host 127.0.0.1 --port 7878 --client-id 1 --run-cycle-sec 2.0 --proc-time-sec 0.4 --proc-count 5
```

### From the `rt-client-rs` Directory

```bash
cd rt-client-rs
cargo run --example example_rs -- --host 127.0.0.1 --port 7878 --client-id 1
```

### CLI Arguments

- `--host`: Server IP address (default: `127.0.0.1`).
- `--port`: Server UDP port (default: `7878`).
- `--client-id`: Unique Client ID assigned in scheduler configuration (default: `1`).
- `--run-cycle-sec`: Execution cycle time in seconds (default: `2.0`).
- `--startup-wait-sec`: Maximum wait timeout for startup barrier in seconds (default: `60.0`).
- `--proc-time-sec`: Simulated task execution duration in seconds (default: `0.4`).
- `--proc-count`: Maximum number of execution cycles before exiting (default: `5`).

## 5. Notes & Configuration

### High-Precision Windows Timer

Windows platforms default to a 15.6ms timer resolution. To guarantee 1ms precision for retry and timeout handling, the library dynamically invokes `timeBeginPeriod(1)` and `timeEndPeriod(1)` on Windows targets.

### Internal Logging

The library uses the standard [`log`](https://crates.io/crates/log) facade. Protocol events and diagnostics are emitted via standard logging macros (`error!`, `warn!`, `trace!`, and `debug!`).

To view internal client logs, initialize any `log`-compatible logger (e.g., `env_logger`, `flexi_logger`) in your binary:

```rust
// Example using env_logger
env_logger::init();
```

### Retry and Timeout Customization

Communication timeouts and retry counts can be customized directly on the client's `config` field (`RtClientConfig`):

```rust
let mut client = RtClient::new("127.0.0.1", 7878, 1, 2.0, 60.0);

// Adjust retries and timeouts for high-jitter environments if necessary
client.config.retry_sec_ready = 0.050;
client.config.retry_count_ready = 5;
```
