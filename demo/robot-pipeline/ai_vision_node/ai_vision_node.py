#!/usr/bin/env python3
# Copyright 2026 Naoyuki Kawano
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
# ==============================================================================

"""AI Vision node for the Ritsu multi-language robot pipeline demonstration."""

from pathlib import Path
import random
import sys
import time

# Add path to rt-client-py directory in the repository root.
_REPO_ROOT: Path = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(_REPO_ROOT / "rt-client-py"))

from rtclient import ResponseType, RtClient  # noqa: E402

# ------------------------------------------------------------------------------
# Ritsu Scheduler Constants
# ------------------------------------------------------------------------------
NODE_NAME: str = "ai_vision_node"
HOST: str = "127.0.0.1"
PORT: int = 7878
CLIENT_ID: int = 20
INTERVAL_SEC: float = 0.2
STARTUP_WAIT_SEC: float = 60.0

# Simulated inference latency range in milliseconds.
MIN_DURATION_MS: float = 55.0
MAX_DURATION_MS: float = 85.0


# ------------------------------------------------------------------------------
# Scenario & Terminal Helpers
# ------------------------------------------------------------------------------
def get_scenario_message(running_cycle: int) -> str:
    """Return the vision scenario message for the given 0-indexed cycle."""
    if 0 <= running_cycle <= 6:
        return "Status: Clear (no obstacles in ROI)"
    elif 7 <= running_cycle <= 13:
        return "Detected: Person at (x=2.1m, y=0.1m, conf=0.92)"
    elif 14 <= running_cycle <= 19:
        return "Tracking: Person passing at (x=1.1m, y=-0.8m, conf=0.85)"
    else:
        return "Status: Clear (obstacle cleared)"


def wait_for_enter() -> None:
    """Block until the user presses Enter so terminal logs remain visible."""
    try:
        input("\n[Process completed] Press Enter key to close...\n")
    except (EOFError, KeyboardInterrupt):
        pass


# ------------------------------------------------------------------------------
# Main Control Loop
# ------------------------------------------------------------------------------
def main() -> int:
    """Initialize RtClient and execute the AI vision control loop."""
    client: RtClient = RtClient(
        HOST,
        PORT,
        CLIENT_ID,
        INTERVAL_SEC,
        STARTUP_WAIT_SEC,
    )

    if not client.join():
        print(f"[{NODE_NAME}] Failed to join server at {HOST}:{PORT}", flush=True)
        wait_for_enter()
        return 1
    print(
        f"[{NODE_NAME}] Joined server at {HOST}:{PORT} "
        f"(CID: {CLIENT_ID}, interval: {INTERVAL_SEC:.3f}s)",
        flush=True,
    )

    proc_count: int = 0
    try:
        while True:
            sys.stdout.flush()
            resp_type: ResponseType
            extras: dict[str, str]
            resp_type, extras = client.wait_next()

            if resp_type == ResponseType.START:
                running_cycle: int = int(extras.get("cycle", proc_count))
                proc_count += 1

                target_duration_ms: float = random.uniform(MIN_DURATION_MS, MAX_DURATION_MS)
                start_time: float = time.perf_counter()
                time.sleep(target_duration_ms / 1000.0)
                proc_ms: float = (time.perf_counter() - start_time) * 1000.0

                scenario_msg: str = get_scenario_message(running_cycle)
                print(
                    f"[Cyc {running_cycle:02d}] [{NODE_NAME}] {scenario_msg} proc_ms={proc_ms:.2f}",
                    flush=True,
                )

                if client.notify_done() == ResponseType.ERROR:
                    print(f"[{NODE_NAME}] Received ERROR on notify_done, exiting", flush=True)
                    break

            elif resp_type in (ResponseType.SKIP, ResponseType.LATE):
                proc_count += 1
                print(
                    f"[{NODE_NAME}] Received {resp_type.value}, skipping execution for this cycle",
                    flush=True,
                )
                continue

            elif resp_type == ResponseType.ERROR:
                reason: str = extras.get("reason", "N/A")
                cid: str = extras.get("cid", "N/A")
                print(f"[{NODE_NAME}] Exiting on ERROR (reason={reason}, cid={cid})", flush=True)
                break

            else:
                print(
                    f"[{NODE_NAME}] Exiting due to unexpected message type: {resp_type.value}",
                    flush=True,
                )
                break

    except KeyboardInterrupt:
        print(f"[{NODE_NAME}] Interrupted, exiting", flush=True)
    finally:
        sys.stdout.flush()
        client.exit()

    wait_for_enter()
    return 0


if __name__ == "__main__":
    sys.exit(main())
