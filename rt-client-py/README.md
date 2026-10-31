# Ritsu Client for Python

## 1. Overview

`rt-client-py` is a lightweight Python client library for the Ritsu process scheduler.

- **Zero External Dependencies**: Implemented in pure Python using only standard libraries.
- **Standard Packaging**: Easily installable into your Python environment via `pip` or direct Git URLs.
- **Deterministic & Synchronous**: Orchestrates process execution through explicit `wait_next()` and `notify_done()` cycles.
- **Cross-Platform**: Works identically across Linux, Windows, and macOS.

## 2. Installation / Setup

You can install the client directly from the repository.

### Standard Installation

```bash
# From the repository root
pip install ./rt-client-py
```

### Editable (Development) Installation

If you are developing or modifying the client library itself:

```bash
pip install -e ./rt-client-py
```

### Direct Git Installation

You can also install the client directly from GitHub:

```bash
pip install "git+https://github.com/nao-kawano/ritsu-scheduler.git#subdirectory=rt-client-py"
```

## 3. Quick Start

Here is a minimal example demonstrating how to connect to the scheduler server, participate in the deterministic scheduling loop, and exit cleanly:

```python
import sys
import time
from rtclient import RtClient, ResponseType, LogLevel

def main():
    # Initialize client config (host, port, client_id, run_cycle_sec, startup_wait_sec)
    client = RtClient(
        host="127.0.0.1",
        port=7878,
        client_id=1,
        run_cycle_sec=2.0,
        startup_wait_sec=60.0,
        log_level=LogLevel.WARN,
    )

    # Join the scheduler server
    if not client.join():
        print("Failed to join server", file=sys.stderr)
        return -1

    print("Successfully joined the server!")

    # Main execution loop
    max_cycles = 5
    for cycle in range(max_cycles):
        # Wait for the next scheduling event
        resp_type, extras = client.wait_next()

        if resp_type == ResponseType.START:
            print(f"[{cycle}] Got START, executing task...")
            # Simulate or execute actual task logic here
            time.sleep(0.4)
            # Notify task completion to the scheduler
            client.notify_done()
        elif resp_type == ResponseType.SKIP:
            print(f"[{cycle}] Got SKIP (previous deadline overrun), waiting for next cycle...")
        elif resp_type == ResponseType.LATE:
            print(f"[{cycle}] Got LATE, waiting for next cycle...")
        else:
            reason = extras.get("reason", "unknown")
            print(f"[{cycle}] Received error or unexpected response: {resp_type.value} ({reason})")
            break

    # Exit cleanly and close socket resources
    client.exit()
    print("Exited cleanly.")
    return 0

if __name__ == "__main__":
    main()
```

## 4. Running the Standalone Test Client

`rtclient.py` includes a standalone runner for quick sanity checks and connection testing without writing extra code.

You can execute it directly to test communication with a running Ritsu server:

```bash
python -m rtclient --host 127.0.0.1 --port 7878 --client-id 1 --run-cycle-sec 2.0 --proc-time-sec 0.4 --proc-count 5
```

### CLI Arguments

- `--host`: Server IP address (default: `127.0.0.1`).
- `--port`: Server UDP port (default: `7878`).
- `--client-id`: Unique Client ID assigned in scheduler configuration (default: `1`).
- `--run-cycle-sec`: Execution cycle time in seconds (default: `2.0`).
- `--startup-wait-sec`: Maximum wait timeout for startup barrier in seconds (default: `60.0`).
- `--proc-time-sec`: Simulated task execution duration in seconds (default: `0.4`).
- `--proc-count`: Maximum number of execution cycles before exiting (default: `5`).
- `--log-level`: Internal log verbosity: `off`, `error`, `warn`, `debug` (default: `warn`).

## 5. Notes & Configuration

### Internal Logging and Log Level Control

The client routes internal diagnostic and protocol logs to standard error (`sys.stderr`) with millisecond-precision timestamps (`HH:MM:SS.mmm [LEVEL]`), leaving standard output clean for domain logic.

Available log levels (`LogLevel`):
- `LogLevel.OFF` (`0`): Completely silent.
- `LogLevel.ERROR` (`1`): Critical error logs only (e.g., socket failures, protocol errors).
- `LogLevel.WARN` (`2`): Warnings, packet retransmissions, and client ID mismatches (**Default**).
- `LogLevel.DEBUG` (`3`): Full communication packet trace (`>> send`, `<< recv`).

Example:
```python
client = RtClient("127.0.0.1", 7878, client_id=1, log_level=LogLevel.DEBUG)
```

### Retry and Timeout Customization

Communication timeouts and retry counts can be adjusted via the `Config` instance:

```python
client = RtClient("127.0.0.1", 7878, client_id=1)
# Adjust retries for high-jitter environments if necessary
client.config.RETRY_COUNT_READY = 5
client.config.RETRY_SEC_READY = 0.050
```
