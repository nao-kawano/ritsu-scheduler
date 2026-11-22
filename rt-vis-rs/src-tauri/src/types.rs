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

impl InstantEventType {
    /// Returns true if this event represents an anomaly displayed in the anomaly timeline.
    pub fn is_anomaly(self) -> bool {
        match self {
            Self::Overrun | Self::Skip | Self::Late | Self::Retransmit | Self::Error => true,
            Self::Ready | Self::Exit => false,
        }
    }

    /// Returns true if this event breaks deterministic cycle health (excludes lifecycle Exit and
    /// Retransmit, because successful UDP retransmission recovers within the cycle).
    pub fn is_critical_anomaly(self) -> bool {
        match self {
            Self::Overrun | Self::Skip | Self::Late | Self::Error => true,
            Self::Ready | Self::Exit | Self::Retransmit => false,
        }
    }
}

/* -------------------------------------------------------------------------- */
// Statistics & Analysis IPC Data Structures
/* -------------------------------------------------------------------------- */

/// Time-weighted duration and ratio spent at a discrete concurrency level.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConcurrencyLevelStat {
    pub level: u32,
    pub duration_us: u64,
    pub ratio: f64,
}

/// System-wide concurrency and utilization statistics over a time window.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConcurrencyStats {
    pub total_duration_us: u64,
    pub active_duration_us: u64,
    pub active_ratio: f64,
    pub active_avg_concurrency: f64,
    pub peak_concurrency: u32,
    /// Earliest timestamp where `peak_concurrency` occurs; `None` when `peak_concurrency == 0` or window duration is 0.
    pub peak_time_us: Option<u64>,
    pub distribution: Vec<ConcurrencyLevelStat>,
}

/// System-wide health and anomaly event summary across all clients.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SystemStats {
    pub total_cycles: u64,
    pub clean_cycles: u64,
    pub clean_cycle_ratio: f64,
    /// Total `Overrun` instant events; may differ by 1 per client from `ClientStats.overrun_count` if truncated mid-overrun.
    pub overrun_count: u64,
    pub skip_count: u64,
    pub late_count: u64,
    pub retransmit_count: u64,
    pub error_count: u64,
}

/// Execution duration distribution metrics in microseconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DurationStats {
    pub sample_count: u64,
    pub min_us: u64,
    pub avg_us: f64,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub max_us: u64,
    pub std_dev_us: f64,
    pub wcet_cycle: u64,
    pub wcet_time_us: u64,
}

/// Single half-open bin `[start_us, end_us)` inside a histogram.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HistogramBin {
    pub start_us: i64,
    pub end_us: i64,
    pub count: u64,
}

/// Histogram with uniform nice-number step size and explicit underflow/overflow buckets.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Histogram {
    pub step_us: u64,
    pub origin_us: i64,
    pub bins: Vec<HistogramBin>,
    pub underflow_count: u64,
    pub overflow_count: u64,
    pub total_count: u64,
}

/// Expected-duration budget coverage, exceedances, margins, and histogram.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ClientBudgetStats {
    pub expected_duration_us: u64,
    pub coverage_ratio: f64,
    pub exceed_count: u64,
    pub avg_exceed_us: f64,
    pub max_exceed_us: u64,
    pub margin_p99_us: i64,
    pub margin_wcet_us: i64,
    pub duration_histogram: Histogram,
}

/// Execution, event, duration, and budget statistics for a single client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ClientStats {
    pub cid: u16,
    pub normal_count: u64,
    /// Completed execution bars ending in `ExecutionStatus::Overrun`.
    /// Note: Incomplete bars at log end are excluded to keep the completion-rate denominator consistent,
    /// so this may be 1 less than `SystemStats.overrun_count` if a log terminates mid-overrun.
    pub overrun_count: u64,
    pub incomplete_count: u64,
    pub skip_count: u64,
    pub late_count: u64,
    pub retransmit_count: u64,
    pub error_count: u64,
    pub exit_count: u64,
    /// Completed execution duration statistics; `None` when completed execution count is 0.
    pub duration: Option<DurationStats>,
    /// Expected-duration budget alignment statistics; `None` when completed execution count is 0.
    pub budget: Option<ClientBudgetStats>,
}

/// Single extreme jitter cycle entry for outlier inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct JitterOutlier {
    pub cycle: u64,
    pub time_us: u64,
    pub jitter_us: i64,
}

/// Cycle-to-cycle start jitter statistics (excluding Cycle 0).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct JitterStats {
    pub sample_count: u64,
    pub min_us: i64,
    pub max_us: i64,
    pub avg_us: f64,
    pub min_cycle: u64,
    pub min_time_us: u64,
    pub max_cycle: u64,
    pub max_time_us: u64,
    pub abs_avg_us: f64,
    pub abs_p95_us: u64,
    pub abs_p99_us: u64,
    pub abs_std_dev_us: f64,
    pub histogram: Histogram,
    pub outliers: Vec<JitterOutlier>,
}

/// Full statistics bundle aggregated across the loaded execution log.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LogStats {
    pub system: SystemStats,
    /// Per-client statistics ordered by ascending client ID.
    pub clients: Vec<ClientStats>,
    /// Cycle-to-cycle start jitter statistics; `None` when fewer than 2 cycles exist (`cycles.len() <= 1`).
    pub jitter: Option<JitterStats>,
    pub concurrency: ConcurrencyStats,
}

/// Flattened anomaly event entry for cross-client chronological event inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AnomalyEventItem {
    pub log_line_no: u64,
    pub instance_id: u64,
    pub cid: u16,
    pub cycle: u64,
    pub time_us: u64,
    pub event_type: InstantEventType,
}

/// Paginated response returned by `get_anomaly_events` IPC command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AnomalyEventsPage {
    pub total_count: usize,
    pub items: Vec<AnomalyEventItem>,
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
    pub anchor_cycle: u64,
    pub anchor_offset_us: u64,
    pub start_us: u64,
    pub duration_us: u64,
    pub depends_instance_ids: Vec<u32>,
    pub status: ExecutionStatus,
}

/// Planned concurrency metric step event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PlannedMetricPoint {
    pub time_us: u64,
    pub running_count: u32,
}

/// Result returned by simulate_plan IPC command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SimulationResult {
    pub executions: Vec<PlannedExecution>,
    pub metrics: Vec<PlannedMetricPoint>,
    /// Steady-state concurrency statistics; `None` when client list is empty or static validation fails.
    pub concurrency_stats: Option<ConcurrencyStats>,
    pub config_errors: HashMap<u16, Vec<String>>,
}

impl SimulationResult {
    pub fn new(
        executions: Vec<PlannedExecution>,
        metrics: Vec<PlannedMetricPoint>,
        concurrency_stats: Option<ConcurrencyStats>,
        config_errors: HashMap<u16, Vec<String>>,
    ) -> Self {
        Self {
            executions,
            metrics,
            concurrency_stats,
            config_errors,
        }
    }

    /// Creates a result with no executions or errors.
    pub fn empty() -> Self {
        Self::new(Vec::new(), Vec::new(), None, HashMap::new())
    }

    /// Creates a result representing static configuration errors.
    pub fn error(config_errors: HashMap<u16, Vec<String>>) -> Self {
        Self::new(Vec::new(), Vec::new(), None, config_errors)
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
    pub cycle: u64,
    pub time_us: u64,
    pub event_type: InstantEventType,
}

/// Actual execution bar data for a client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActualExecution {
    pub log_line_no_start: u64,
    pub log_line_no_end: Option<u64>,
    pub instance_id: u64,
    pub cycle: u64,
    pub start_us: u64,
    pub duration_us: u64,
    pub status: ExecutionStatus,
}

/// Actual metric data point for concurrency level over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActualMetricPoint {
    pub time_us: u64,
    pub running_count: u32,
}

/// Actual cycle record for server timer and jitter tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActualCycle {
    pub cycle: u64,
    pub start_us: u64,
    pub start_jitter_us: i64,
}

/// Summary metadata returned by load_log IPC upon initial loading.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LogSummary {
    pub config: SchedulerConfig,
    pub max_concurrency: u32,
    pub total_duration_us: u64,
    pub total_cycles: u64,
    pub min_start_jitter_us: i64,
    pub max_start_jitter_us: i64,
    pub stats: LogStats,
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
