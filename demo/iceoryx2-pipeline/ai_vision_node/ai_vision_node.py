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

"""AI Vision node for the Ritsu + iceoryx2 zero-copy pipeline demonstration."""

import ctypes
from pathlib import Path
import sys
import time

import iceoryx2 as iox2

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
INTERVAL_SEC: float = 0.1
STARTUP_WAIT_SEC: float = 60.0

# ------------------------------------------------------------------------------
# Common Header & Status Constants
# ------------------------------------------------------------------------------
STATUS_VALID: int = 0
STATUS_INVALID: int = 1

# ------------------------------------------------------------------------------
# Common Pipeline QoS Constants
# ------------------------------------------------------------------------------
QOS_SUBSCRIBER_MAX_BUFFER_SIZE: int = 1
QOS_ENABLE_SAFE_OVERFLOW: bool = True
QOS_SUBSCRIBER_MAX_BORROWED_SAMPLES: int = 1
QOS_HISTORY_SIZE: int = 0

# ------------------------------------------------------------------------------
# Service Topic & Payload Dimension Constants
# ------------------------------------------------------------------------------
TOPIC_CAMERA_FRAME: str = "Pipeline/CameraFrame"
IMAGE_WIDTH: int = 640
IMAGE_HEIGHT: int = 480
IMAGE_DATA_LEN: int = IMAGE_WIDTH * IMAGE_HEIGHT

TOPIC_VISION_RESULT: str = "Pipeline/VisionResult"

# ------------------------------------------------------------------------------
# Scenario Timing Constants (0-indexed running_cycle)
# ------------------------------------------------------------------------------
# Zero-indexed running cycles at which inference spike and freeze are simulated.
SPIKE_RUNNING_CYCLE: int = 20
FREEZE_RUNNING_CYCLE: int = 30

NORMAL_PROC_MS: float = 40.0
FALLBACK_PROC_MS: float = 5.0
SPIKE_PROC_MS: float = 110.0
FREEZE_PROC_MS: float = 300.0


# ------------------------------------------------------------------------------
# Shared C-ABI Data Structures
# ------------------------------------------------------------------------------
class PayloadHeader(ctypes.Structure):
    """Common 24-byte header shared across Rust and Python payloads."""

    _fields_ = [
        ("cycle", ctypes.c_uint64),
        ("timestamp_us", ctypes.c_uint64),
        ("status", ctypes.c_uint32),
        ("reserved", ctypes.c_uint32),
    ]

    @staticmethod
    def type_name() -> str:
        """Return the cross-language iceoryx2 type name."""
        return "PayloadHeader"


class SensorResult(ctypes.Structure):
    """Camera frame payload (307,232 bytes) published by sensor_node."""

    _fields_ = [
        ("header", PayloadHeader),
        ("width", ctypes.c_uint32),
        ("height", ctypes.c_uint32),
        ("data", ctypes.c_uint8 * IMAGE_DATA_LEN),
    ]

    @staticmethod
    def type_name() -> str:
        """Return the cross-language iceoryx2 type name."""
        return "SensorResult"


class VisionResult(ctypes.Structure):
    """Inference output payload (40 bytes) published by ai_vision_node."""

    _fields_ = [
        ("header", PayloadHeader),
        ("detected", ctypes.c_uint32),
        ("distance_m", ctypes.c_float),
        ("angle_deg", ctypes.c_float),
        ("confidence", ctypes.c_float),
    ]

    @staticmethod
    def type_name() -> str:
        """Return the cross-language iceoryx2 type name."""
        return "VisionResult"


# ------------------------------------------------------------------------------
# Zero-Copy IPC & Inference Helpers
# ------------------------------------------------------------------------------
def drain_latest_sample(subscriber: iox2.Subscriber) -> iox2.Sample | None:
    """Drain queued samples and return the newest borrowed sample without copying."""
    # Release each older borrow before receiving the next so max_borrowed_samples=1 holds.
    latest: iox2.Sample | None = subscriber.receive()
    while latest is not None and subscriber.has_samples():
        latest.delete()
        latest = subscriber.receive()
    return latest


def run_normal_inference(
    frame: SensorResult,
    running_cycle: int,
    cycle_start_ns: int,
) -> VisionResult:
    """Inspect the zero-copy camera frame, pace to target latency, and return VisionResult."""
    # Inspect frame dimensions directly in shared memory (no buffer copy).
    frame_valid: bool = (
        int(frame.width) == IMAGE_WIDTH
        and int(frame.height) == IMAGE_HEIGHT
    )

    target_ms: float = NORMAL_PROC_MS
    if running_cycle == FREEZE_RUNNING_CYCLE:
        target_ms = FREEZE_PROC_MS
        print(
            f"[Cyc {running_cycle:02d}] [{NODE_NAME}] [WARN] "
            f"Simulating freeze ({FREEZE_PROC_MS:.0f} ms)",
            flush=True,
        )
    elif running_cycle == SPIKE_RUNNING_CYCLE:
        target_ms = SPIKE_PROC_MS
        print(
            f"[Cyc {running_cycle:02d}] [{NODE_NAME}] [WARN] "
            f"Simulating inference spike ({SPIKE_PROC_MS:.0f} ms)",
            flush=True,
        )

    elapsed_ms: float = (time.perf_counter_ns() - cycle_start_ns) / 1_000_000.0
    remaining_s: float = (target_ms - elapsed_ms) / 1000.0
    if remaining_s > 0.0:
        time.sleep(remaining_s)

    result: VisionResult = VisionResult()
    result.header.cycle = running_cycle
    result.header.timestamp_us = 0
    result.header.status = STATUS_VALID
    result.header.reserved = 0
    result.detected = 1 if frame_valid else 0
    result.distance_m = max(1.2, 4.5 - 0.08 * (running_cycle + 1))
    result.angle_deg = round((((running_cycle + 1) % 7) - 3) * 1.5, 2)
    result.confidence = 0.95
    return result


def run_fallback_inference(
    running_cycle: int,
    prev: VisionResult,
    cycle_start_ns: int,
) -> VisionResult:
    """Run lightweight fallback estimation from the last valid VisionResult."""
    elapsed_ms: float = (time.perf_counter_ns() - cycle_start_ns) / 1_000_000.0
    remaining_s: float = (FALLBACK_PROC_MS - elapsed_ms) / 1000.0
    if remaining_s > 0.0:
        time.sleep(remaining_s)

    result: VisionResult = VisionResult()
    result.header.cycle = running_cycle
    result.header.timestamp_us = 0
    result.header.status = STATUS_INVALID
    result.header.reserved = 0
    result.detected = int(prev.detected)
    result.distance_m = float(prev.distance_m)
    result.angle_deg = float(prev.angle_deg)
    result.confidence = max(0.1, float(prev.confidence) * 0.5)
    return result


def publish_vision(publisher: iox2.Publisher, payload: VisionResult) -> None:
    """Loan shared memory, stamp the publish timestamp, and send VisionResult."""
    sample_uninit: iox2.SampleMutUninit = publisher.loan_uninit()
    payload.header.timestamp_us = time.time_ns() // 1000
    sample_mut: iox2.SampleMut = sample_uninit.write_payload(payload)
    sample_mut.send()


def wait_for_enter() -> None:
    """Block until the user presses Enter so terminal logs remain visible."""
    try:
        input("\n[Process completed] Press Enter key to close...\n")
    except (EOFError, KeyboardInterrupt):
        pass


# ------------------------------------------------------------------------------
# Main Control & Zero-Copy IPC Loop
# ------------------------------------------------------------------------------
def main() -> int:
    """Initialize iceoryx2 and RtClient, then execute the vision control loop."""
    iox2.set_log_level_from_env_or(iox2.LogLevel.Error)

    node: iox2.Node = (
        iox2.NodeBuilder.new().name(iox2.NodeName.new(NODE_NAME)).create(iox2.ServiceType.Ipc)
    )

    camera_service: iox2.PortFactoryPublishSubscribe = (
        node.service_builder(iox2.ServiceName.new(TOPIC_CAMERA_FRAME))
        .publish_subscribe(SensorResult)
        .subscriber_max_buffer_size(QOS_SUBSCRIBER_MAX_BUFFER_SIZE)
        .enable_safe_overflow(QOS_ENABLE_SAFE_OVERFLOW)
        .subscriber_max_borrowed_samples(QOS_SUBSCRIBER_MAX_BORROWED_SAMPLES)
        .history_size(QOS_HISTORY_SIZE)
        .open_or_create()
    )
    camera_subscriber: iox2.Subscriber = camera_service.subscriber_builder().create()

    vision_service: iox2.PortFactoryPublishSubscribe = (
        node.service_builder(iox2.ServiceName.new(TOPIC_VISION_RESULT))
        .publish_subscribe(VisionResult)
        .subscriber_max_buffer_size(QOS_SUBSCRIBER_MAX_BUFFER_SIZE)
        .enable_safe_overflow(QOS_ENABLE_SAFE_OVERFLOW)
        .subscriber_max_borrowed_samples(QOS_SUBSCRIBER_MAX_BORROWED_SAMPLES)
        .history_size(QOS_HISTORY_SIZE)
        .open_or_create()
    )
    vision_publisher: iox2.Publisher = vision_service.publisher_builder().create()

    client: RtClient = RtClient(HOST, PORT, CLIENT_ID, INTERVAL_SEC, STARTUP_WAIT_SEC)
    if not client.join():
        print(f"[{NODE_NAME}] Failed to join server at {HOST}:{PORT}", flush=True)
        wait_for_enter()
        return 1
    print(
        f"[{NODE_NAME}] Joined server at {HOST}:{PORT} "
        f"(CID: {CLIENT_ID}, interval: {INTERVAL_SEC:.3f}s)",
        flush=True,
    )

    # Last valid inference output reused during lightweight fallback cycles.
    last_vision: VisionResult = VisionResult(
        detected=1,
        distance_m=3.5,
        angle_deg=0.0,
        confidence=0.92,
    )
    proc_count: int = 0
    try:
        while True:
            resp_type: ResponseType
            extras: dict[str, str]
            resp_type, extras = client.wait_next()

            if resp_type == ResponseType.START:
                cycle_start_ns: int = time.perf_counter_ns()
                running_cycle: int = int(extras.get("cycle", proc_count))
                proc_count += 1

                sample: iox2.Sample | None = None
                src_cycle: int | None = None
                ipc_read_us: float = 0.0
                e2e_us: int = 0
                out_payload: VisionResult = VisionResult()
                mode: str = "FALLBACK"

                # Borrow the latest camera frame only through inference, then release it immediately.
                try:
                    read_start_ns: int = time.perf_counter_ns()
                    sample = drain_latest_sample(camera_subscriber)
                    frame: SensorResult | None = None
                    if sample is not None:
                        frame = sample.payload().contents
                        src_cycle = int(frame.header.cycle)
                        e2e_us = max(0, (time.time_ns() // 1000) - int(frame.header.timestamp_us))
                    ipc_read_us = (time.perf_counter_ns() - read_start_ns) / 1000.0

                    # Dispatch normal inference vs. lightweight fallback based on freshness and status.
                    if (
                        frame is not None
                        and int(frame.header.cycle) >= running_cycle
                        and int(frame.header.status) == STATUS_VALID
                    ):
                        mode = "NORMAL"
                        out_payload = run_normal_inference(
                            frame,
                            running_cycle,
                            cycle_start_ns,
                        )
                    else:
                        mode = "FALLBACK"
                        out_payload = run_fallback_inference(
                            running_cycle,
                            last_vision,
                            cycle_start_ns,
                        )
                finally:
                    if sample is not None:
                        sample.delete()

                if out_payload.header.status == STATUS_VALID:
                    last_vision = out_payload

                write_start_ns: int = time.perf_counter_ns()
                publish_vision(vision_publisher, out_payload)
                ipc_write_us: float = (time.perf_counter_ns() - write_start_ns) / 1000.0

                proc_ms: float = (time.perf_counter_ns() - cycle_start_ns) / 1_000_000.0
                src_cyc_str: str = str(src_cycle) if src_cycle is not None else "none"
                print(
                    f"[Cyc {running_cycle:02d}] [{NODE_NAME}] {mode:<8s} "
                    f"src_cyc={src_cyc_str} status={out_payload.header.status} "
                    f"ipc_read_us={ipc_read_us:.2f} e2e_us={e2e_us} "
                    f"ipc_write_us={ipc_write_us:.2f} proc_ms={proc_ms:.2f}",
                    flush=True,
                )

                if client.notify_done() == ResponseType.ERROR:
                    print(f"[{NODE_NAME}] Received ERROR on notify_done, exiting", flush=True)
                    break

            elif resp_type in (ResponseType.SKIP, ResponseType.LATE):
                proc_count += 1
                print(
                    f"[{NODE_NAME}] Received {resp_type.value}, skipping inference for this cycle",
                    flush=True,
                )
                continue

            elif resp_type == ResponseType.ERROR:
                reason: str = extras.get("reason", "N/A")
                cid: str = extras.get("cid", "N/A")
                print(
                    f"[{NODE_NAME}] Exiting on ERROR (reason={reason}, cid={cid})",
                    flush=True,
                )
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
        client.exit()

    wait_for_enter()
    return 0


if __name__ == "__main__":
    sys.exit(main())
