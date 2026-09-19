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
//!
//! Common Data Types & IPC Shared Models for Ritsu Vis.
//!

use rt_config::SchedulerConfig;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/* -------------------------------------------------------------------------- */
// Common Domain Enums
/* -------------------------------------------------------------------------- */

/// Execution outcome status for both planned simulation and actual executions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Normal,
    Overrun,
    Skip,
}

/// Point-in-time instant event types recorded during execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstantEventType {
    Ready,
    Overrun,
    Skip,
    Late,
    Error,
    Exit,
    Retransmit,
}

/* -------------------------------------------------------------------------- */
// Plan & Simulation IPC Data Structures (Create Mode & Plan Overlay)
/* -------------------------------------------------------------------------- */

/// Planned execution bar data computed by simulation engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PlannedExecution {
    pub instance_id: u32,
    pub cid: u16,
    pub anchor_cycle: i64,
    pub anchor_offset_ms: u32,
    pub start_ms: u32,
    pub duration_ms: u32,
    pub depends_instance_ids: Vec<u32>,
    pub status: ExecutionStatus,
}

/// Planned concurrency metric step event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PlannedMetricPoint {
    pub time_ms: u32,
    pub running_count: u32,
}

/// Result returned by simulate_plan IPC command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SimulationResult {
    pub executions: Vec<PlannedExecution>,
    pub metrics: Vec<PlannedMetricPoint>,
    pub config_errors: HashMap<u16, Vec<String>>,
}

impl SimulationResult {
    pub fn new(
        executions: Vec<PlannedExecution>,
        metrics: Vec<PlannedMetricPoint>,
        config_errors: HashMap<u16, Vec<String>>,
    ) -> Self {
        Self {
            executions,
            metrics,
            config_errors,
        }
    }

    /// Creates a result with no executions or errors.
    pub fn empty() -> Self {
        Self::new(Vec::new(), Vec::new(), HashMap::new())
    }

    /// Creates a result representing static configuration errors.
    pub fn error(config_errors: HashMap<u16, Vec<String>>) -> Self {
        Self::new(Vec::new(), Vec::new(), config_errors)
    }
}

/* -------------------------------------------------------------------------- */
// Actual Log & Analyze IPC Data Structures (Analyze Mode)
/* -------------------------------------------------------------------------- */

/// Point-in-time event recorded during actual execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActualInstantEvent {
    pub log_line_no: u64,
    pub instance_id: u64,
    pub time_ms: u64,
    pub event_type: InstantEventType,
}

/// Actual execution bar data for a client process.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActualExecution {
    pub log_line_no_start: u64,
    pub log_line_no_end: Option<u64>,
    pub instance_id: u64,
    pub cycle: u64,
    pub start_ms: u64,
    pub duration_ms: u32,
    pub status: ExecutionStatus,
}

/// Actual metric data point for concurrency level over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActualMetricPoint {
    pub time_ms: u64,
    pub running_count: u32,
}

/// Actual cycle record for server timer and jitter tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActualCycle {
    pub cycle: u64,
    pub start_ms: u64,
    pub start_jitter_ms: f64,
}

/// Summary metadata returned by load_log IPC upon initial loading.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LogSummary {
    pub config: SchedulerConfig,
    pub max_concurrency: u32,
    pub total_duration_ms: u64,
    pub total_cycles: u64,
    pub min_start_jitter_ms: f64,
    pub max_start_jitter_ms: f64,
    pub is_truncated: bool,
}

/// Range-restricted actual log data returned by get_log_range IPC for viewport rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LogRangeData {
    pub actual_executions_by_cid: Vec<(u16, Vec<ActualExecution>)>,
    pub actual_instant_events_by_cid: Vec<(u16, Vec<ActualInstantEvent>)>,
    pub actual_metrics: Vec<ActualMetricPoint>,
    pub actual_cycles: Vec<ActualCycle>,
    pub is_truncated: bool,
}
