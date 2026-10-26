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

import argparse
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
DEFAULT_CID = 20
DEFAULT_PORT = 10000
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


def main() -> int:
    parser = argparse.ArgumentParser(description="AI Vision Node (Python)")
    parser.add_argument("--host", default="127.0.0.1", help="Server host")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT, help="Server port")
    parser.add_argument("--client-id", type=int, default=DEFAULT_CID, help="Client ID")
    parser.add_argument("--run-cycle-sec", type=float, default=0.2,
                        help="Cycle duration in seconds")
    parser.add_argument("--startup-wait-sec", type=float, default=60.0, help="Startup timeout")
    args = parser.parse_args()

    client = RtClient(
        args.host,
        args.port,
        args.client_id,
        args.run_cycle_sec,
        args.startup_wait_sec
    )

    if not client.join():
        print(f"[{NODE_NAME}] Failed to join server", file=sys.stderr)
        return 1
    print(f"[{NODE_NAME}] Joined server at {args.host}:{args.port} (CID: {args.client_id})")

    proc_count = 0
    try:
        while True:
            resp_type, extras = client.wait_next()
            if resp_type == ResponseType.START:
                if proc_count >= MAX_CYCLES:
                    break
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
                    print(f"[{NODE_NAME}] Received ERROR on notify_done", file=sys.stderr)
                    break
                proc_count += 1
            elif resp_type in (ResponseType.SKIP, ResponseType.LATE):
                continue
            else:
                if proc_count < MAX_CYCLES:
                    print(f"[{NODE_NAME}] Exiting due to unexpected response: {resp_type}", file=sys.stderr)
                break
    except KeyboardInterrupt:
        pass
    finally:
        client.exit()

    try:
        input("\n[Process completed] Press Enter key to close...\n")
    except (EOFError, KeyboardInterrupt):
        pass

    return 0


if __name__ == "__main__":
    sys.exit(main())
