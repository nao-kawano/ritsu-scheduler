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

use iceoryx2::prelude::ZeroCopySend;

// -----------------------------------------------------------------------------
// Section 1: Common Header & Status (shared across all services)
// -----------------------------------------------------------------------------

/// Status flag indicating that the payload data is valid for downstream use.
pub const STATUS_VALID: u32 = 0;

/// Status flag indicating that the payload data is invalid or degraded.
pub const STATUS_INVALID: u32 = 1;

/// Shared 24-byte C-compatible header prepended to all zero-copy payloads.
#[repr(C)]
#[derive(Debug, Clone, Copy, ZeroCopySend)]
#[type_name("PayloadHeader")]
pub struct PayloadHeader {
    /// Zero-indexed `running_cycle` provided by Ritsu `START` extras.
    pub cycle: u64,
    /// Wall-clock timestamp in microseconds since Unix epoch, stamped immediately before `send()`.
    pub timestamp_us: u64,
    /// Pipeline status flag (`STATUS_VALID` or `STATUS_INVALID`).
    pub status: u32,
    /// Reserved padding field fixed to `0` to preserve 8-byte alignment.
    pub reserved: u32,
}

// -----------------------------------------------------------------------------
// Section 2: Common Pipeline QoS Policy (shared "latest-1" policy)
// -----------------------------------------------------------------------------

/// Maximum number of unread samples buffered per subscriber.
pub const QOS_SUBSCRIBER_MAX_BUFFER_SIZE: usize = 1;

/// Overwrite the oldest unread sample when the subscriber buffer is full.
pub const QOS_ENABLE_SAFE_OVERFLOW: bool = true;

/// Maximum number of samples a subscriber may borrow concurrently.
pub const QOS_SUBSCRIBER_MAX_BORROWED_SAMPLES: usize = 1;

/// Historical sample count retained for late-joining subscribers.
pub const QOS_HISTORY_SIZE: usize = 0;

// -----------------------------------------------------------------------------
// Section 3: Service Pipeline/CameraFrame (sensor_node -> ai_vision_node)
// -----------------------------------------------------------------------------

/// Service topic carrying raw camera frames from `sensor_node` to `ai_vision_node`.
pub const TOPIC_CAMERA_FRAME: &str = "Pipeline/CameraFrame";

/// Fixed camera frame width in pixels.
pub const IMAGE_WIDTH: u32 = 640;

/// Fixed camera frame height in pixels.
pub const IMAGE_HEIGHT: u32 = 480;

/// Total byte length of a 640x480 8-bit grayscale camera frame.
pub const IMAGE_DATA_LEN: usize = (IMAGE_WIDTH as usize) * (IMAGE_HEIGHT as usize);

/// Zero-copy camera frame payload (307,232 bytes) published on `Pipeline/CameraFrame`.
#[repr(C)]
#[derive(Debug, Clone, Copy, ZeroCopySend)]
#[type_name("SensorResult")]
pub struct SensorResult {
    /// Common 24-byte pipeline header.
    pub header: PayloadHeader,
    /// Frame width in pixels (`IMAGE_WIDTH`).
    pub width: u32,
    /// Frame height in pixels (`IMAGE_HEIGHT`).
    pub height: u32,
    /// Raw 8-bit grayscale pixel buffer (`640 * 480 = 307,200` bytes).
    pub data: [u8; IMAGE_DATA_LEN],
}

// -----------------------------------------------------------------------------
// Section 4: Service Pipeline/VisionResult (ai_vision_node -> actuator_node)
// -----------------------------------------------------------------------------

/// Service topic carrying inference outputs from `ai_vision_node` to `actuator_node`.
pub const TOPIC_VISION_RESULT: &str = "Pipeline/VisionResult";

/// Zero-copy inference output payload (40 bytes) published on `Pipeline/VisionResult`.
#[repr(C)]
#[derive(Debug, Clone, Copy, ZeroCopySend)]
#[type_name("VisionResult")]
pub struct VisionResult {
    /// Common 24-byte pipeline header.
    pub header: PayloadHeader,
    /// Obstacle detection flag (`0`: none, `1`: detected).
    pub detected: u32,
    /// Estimated distance to the detected object in meters.
    pub distance_m: f32,
    /// Relative bearing angle to the detected object in degrees.
    pub angle_deg: f32,
    /// Normalized inference confidence score in `[0.0, 1.0]`.
    pub confidence: f32,
}
