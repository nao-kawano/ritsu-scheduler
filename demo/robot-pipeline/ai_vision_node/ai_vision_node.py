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
# =============================================================================

from pathlib import Path
import random
import sys
import time

# Add path to rt-client-py directory
script_dir = Path(__file__).resolve().parent
repo_root = script_dir.parents[2]
sys.path.insert(0, str(repo_root / "rt-client-py"))

from rtclient import RtClient, ResponseType

NODE_NAME = "ai_vision_node"
HOST = "127.0.0.1"
PORT = 7878
CLIENT_ID = 20
INTERVAL_SEC = 0.2
STARTUP_WAIT_SEC = 60.0
MAX_CYCLES = 25
MIN_DURATION_MS = 55.0
MAX_DURATION_MS = 85.0


def get_scenario_message(cycle: int) -> str:
    """Return scenario message based on the execution phase."""
    if 1 <= cycle <= 7:
        return "Status: Clear (no obstacles in ROI)"
    elif 8 <= cycle <= 14:
        return "Detected: Person at (x=2.1m, y=0.1m, conf=0.92)"
    elif 15 <= cycle <= 20:
        return "Tracking: Person passing at (x=1.1m, y=-0.8m, conf=0.85)"
    else:
        return "Status: Clear (obstacle cleared)"


def wait_for_enter() -> None:
    """Block until the user presses Enter so terminal logs remain visible."""
    try:
        input("\n[Process completed] Press Enter key to close...\n")
    except (EOFError, KeyboardInterrupt):
        pass


def main() -> int:
    client = RtClient(
        HOST,
        PORT,
        CLIENT_ID,
        INTERVAL_SEC,
        STARTUP_WAIT_SEC,
    )

    if not client.join():
        print(f"[{NODE_NAME}] Failed to join server", file=sys.stderr)
        wait_for_enter()
        return 1
    print(f"[{NODE_NAME}] Joined server at {HOST}:{PORT} (CID: {CLIENT_ID})")

    proc_count = 0
    try:
        while True:
            resp_type, extras = client.wait_next()
            if resp_type == ResponseType.START:
                cycle = int(extras.get("cycle", proc_count)) + 1

                target_duration_ms = random.uniform(MIN_DURATION_MS, MAX_DURATION_MS)
                start_time = time.perf_counter()
                time.sleep(target_duration_ms / 1000.0)
                elapsed_ms = (time.perf_counter() - start_time) * 1000.0

                scenario_msg = get_scenario_message(cycle)
                print(
                    f"[Cycle {cycle:02d}/{MAX_CYCLES}] [{NODE_NAME:<14s}] {scenario_msg} (elapsed: {elapsed_ms:.1f}ms)", flush=True)

                resp_type = client.notify_done()
                if resp_type == ResponseType.ERROR:
                    print(f"[{NODE_NAME}] Received ERROR on notify_done, exiting", file=sys.stderr)
                    break
                proc_count += 1
            elif resp_type in (ResponseType.SKIP, ResponseType.LATE):
                proc_count += 1
                print(
                    f"[{NODE_NAME}] Received {resp_type.value}, skipping execution for this cycle",
                    flush=True,
                )
                continue
            elif resp_type == ResponseType.ERROR:
                reason = extras.get("reason", "N/A")
                cid = extras.get("cid", "N/A")
                print(f"[{NODE_NAME}] Exiting on ERROR (reason={reason}, cid={cid})", flush=True)
                break
            else:
                print(
                    f"[{NODE_NAME}] Exiting due to unexpected message type: {resp_type.value}",
                    file=sys.stderr,
                )
                break
    except KeyboardInterrupt:
        pass
    finally:
        client.exit()

    wait_for_enter()

    return 0


if __name__ == "__main__":
    sys.exit(main())
