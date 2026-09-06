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
//! In-Memory Log Store & Viewport Slice Operations.
//!

use crate::types::{
    ActualCycle, ActualExecution, ActualInstantEvent, ActualMetricPoint, LogRangeData, LogSummary,
};

use std::collections::HashMap;

#[cfg(test)]
#[path = "log_store_test.rs"]
mod log_store_test;

/* -------------------------------------------------------------------------- */

/// Maximum number of actual executions returned in a single range request (total across all CIDs).
pub const MAX_RANGE_EXECUTIONS: usize = 10_000;

/// Maximum number of instant events returned in a single range request (total across all CIDs).
pub const MAX_RANGE_INSTANT_EVENTS: usize = 10_000;

/// Maximum number of metric points returned in a single range request.
pub const MAX_RANGE_METRICS: usize = 10_000;

/// Maximum number of cycles returned in a single range request.
pub const MAX_RANGE_CYCLES: usize = 5_000;

/* -------------------------------------------------------------------------- */

/// In-memory dataset holding the full parsed log for lifetime of the session.
#[derive(Debug, Clone)]
pub struct LogStore {
    pub summary: LogSummary,
    pub executions_by_cid: HashMap<u16, Vec<ActualExecution>>,
    pub instant_events_by_cid: HashMap<u16, Vec<ActualInstantEvent>>,
    pub metrics: Vec<ActualMetricPoint>,
    pub cycles: Vec<ActualCycle>,
}

impl LogStore {
    /// Constructs a new populated LogStore from parsed components.
    pub fn new(
        summary: LogSummary,
        executions_by_cid: HashMap<u16, Vec<ActualExecution>>,
        instant_events_by_cid: HashMap<u16, Vec<ActualInstantEvent>>,
        metrics: Vec<ActualMetricPoint>,
        cycles: Vec<ActualCycle>,
    ) -> Self {
        Self {
            summary,
            executions_by_cid,
            instant_events_by_cid,
            metrics,
            cycles,
        }
    }

    /// Extracts a range-restricted slice of log data clamped by default safety caps.
    pub fn get_range(&self, start_ms: u64, end_ms: u64) -> LogRangeData {
        self.get_range_with_caps(
            start_ms,
            end_ms,
            MAX_RANGE_EXECUTIONS,
            MAX_RANGE_INSTANT_EVENTS,
            MAX_RANGE_METRICS,
            MAX_RANGE_CYCLES,
        )
    }

    /// Extracts a range-restricted slice of log data with configurable safety caps.
    pub fn get_range_with_caps(
        &self,
        start_ms: u64,
        end_ms: u64,
        max_executions: usize,
        max_events: usize,
        max_metrics: usize,
        max_cycles: usize,
    ) -> LogRangeData {
        if start_ms > end_ms {
            return LogRangeData {
                actual_executions_by_cid: Vec::new(),
                actual_instant_events_by_cid: Vec::new(),
                actual_metrics: Vec::new(),
                actual_cycles: Vec::new(),
                is_truncated: false,
            };
        }

        let mut is_truncated = false;

        // Collect distinct client IDs sorted in ascending order for deterministic output
        let mut sorted_cids: Vec<u16> = self
            .executions_by_cid
            .keys()
            .chain(self.instant_events_by_cid.keys())
            .copied()
            .collect();
        sorted_cids.sort_unstable();
        sorted_cids.dedup();

        // Extract overlapping execution bars per client
        let mut actual_executions_by_cid = Vec::with_capacity(sorted_cids.len());
        let mut total_executions = 0;

        for &cid in &sorted_cids {
            let mut sliced_execs = Vec::new();
            if let Some(execs) = self.executions_by_cid.get(&cid) {
                let upper_bound = execs.partition_point(|e| e.start_ms <= end_ms);
                for exec in &execs[..upper_bound] {
                    let exec_end = exec.start_ms + exec.duration_ms as u64;
                    if exec_end >= start_ms {
                        if total_executions >= max_executions {
                            is_truncated = true;
                            break;
                        }
                        sliced_execs.push(exec.clone());
                        total_executions += 1;
                    }
                }
            }
            actual_executions_by_cid.push((cid, sliced_execs));
        }

        // Extract instant events per client
        let mut actual_instant_events_by_cid = Vec::with_capacity(sorted_cids.len());
        let mut total_events = 0;

        for &cid in &sorted_cids {
            let mut sliced_events = Vec::new();
            if let Some(events) = self.instant_events_by_cid.get(&cid) {
                let start_idx = events.partition_point(|e| e.time_ms < start_ms);
                let end_idx = events.partition_point(|e| e.time_ms <= end_ms);
                for event in &events[start_idx..end_idx] {
                    if total_events >= max_events {
                        is_truncated = true;
                        break;
                    }
                    sliced_events.push(event.clone());
                    total_events += 1;
                }
            }
            actual_instant_events_by_cid.push((cid, sliced_events));
        }

        // Extract concurrency metric points including the immediately preceding point for step continuity
        let mut actual_metrics = Vec::new();
        if !self.metrics.is_empty() {
            let start_idx = self.metrics.partition_point(|m| m.time_ms < start_ms);
            // Only extract if the requested window overlaps with the recorded metrics timeline
            if start_idx < self.metrics.len() {
                let effective_start_idx = if start_idx > 0 { start_idx - 1 } else { 0 };
                let end_idx = self.metrics.partition_point(|m| m.time_ms <= end_ms);

                if effective_start_idx < end_idx {
                    let candidate_slice = &self.metrics[effective_start_idx..end_idx];
                    if candidate_slice.len() > max_metrics {
                        is_truncated = true;
                        actual_metrics.extend_from_slice(&candidate_slice[..max_metrics]);
                    } else {
                        actual_metrics.extend_from_slice(candidate_slice);
                    }
                }
            }
        }

        // Extract cycles with padding margins for anchor and grid rendering
        let cycle_time_ms = self
            .summary
            .config
            .server_config
            .cycle_time_ms
            .max(1) as u64;
        let margin_ms = (cycle_time_ms * 2).max(100);
        let req_cycle_start = start_ms.saturating_sub(margin_ms);
        let req_cycle_end = end_ms.saturating_add(margin_ms);

        let mut actual_cycles = Vec::new();
        if !self.cycles.is_empty() {
            let start_idx = self.cycles.partition_point(|c| c.start_ms < req_cycle_start);
            let end_idx = self.cycles.partition_point(|c| c.start_ms <= req_cycle_end);

            if start_idx < self.cycles.len() {
                let candidate_slice = &self.cycles[start_idx..end_idx.min(self.cycles.len())];
                if candidate_slice.len() > max_cycles {
                    is_truncated = true;
                    actual_cycles.extend_from_slice(&candidate_slice[..max_cycles]);
                } else {
                    actual_cycles.extend_from_slice(candidate_slice);
                }
            }
        }

        LogRangeData {
            actual_executions_by_cid,
            actual_instant_events_by_cid,
            actual_metrics,
            actual_cycles,
            is_truncated,
        }
    }
}
