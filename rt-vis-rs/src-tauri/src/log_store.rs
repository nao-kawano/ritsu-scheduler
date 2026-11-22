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
    ActualCycle, ActualExecution, ActualInstantEvent, ActualMetricPoint, AnomalyEventItem,
    AnomalyEventsPage, ClientBudgetStats, InstantEventType, LogRangeData, LogSummary,
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

/// Maximum number of anomaly events returned in a single page request.
pub const MAX_ANOMALY_EVENTS_PAGE_LIMIT: usize = 1_000;

/* -------------------------------------------------------------------------- */

/// In-memory dataset holding the full parsed log for lifetime of the session.
///
/// Range queries via [`LogStore::get_range_with_caps`] rely on the following invariants
/// established during log parsing:
/// - `executions_by_cid`: sorted by `start_us` and non-overlapping per client (guaranteed by
///   Ritsu's single-active-execution-per-CID state machine under monotonic timestamps), so
///   `start_us.saturating_add(duration_us)` is monotonically non-decreasing.
/// - `instant_events_by_cid`, `metrics`, and `cycles`: sorted in ascending timestamp order.
/// - `anomaly_events`: sorted chronologically by `(time_us, log_line_no, instance_id)` across all clients.
#[derive(Debug, Clone)]
pub struct LogStore {
    pub summary: LogSummary,
    pub cycles: Vec<ActualCycle>,
    pub executions_by_cid: HashMap<u16, Vec<ActualExecution>>,
    pub instant_events_by_cid: HashMap<u16, Vec<ActualInstantEvent>>,
    pub anomaly_events: Vec<AnomalyEventItem>,
    pub metrics: Vec<ActualMetricPoint>,
}

impl LogStore {
    /// Constructs a new populated LogStore from parsed components.
    pub fn new(
        summary: LogSummary,
        cycles: Vec<ActualCycle>,
        executions_by_cid: HashMap<u16, Vec<ActualExecution>>,
        instant_events_by_cid: HashMap<u16, Vec<ActualInstantEvent>>,
        anomaly_events: Vec<AnomalyEventItem>,
        metrics: Vec<ActualMetricPoint>,
    ) -> Self {
        Self {
            summary,
            cycles,
            executions_by_cid,
            instant_events_by_cid,
            anomaly_events,
            metrics,
        }
    }

    /// Extracts a range-restricted slice of log data clamped by default safety caps.
    pub fn get_range(&self, start_us: u64, end_us: u64) -> LogRangeData {
        self.get_range_with_caps(
            start_us,
            end_us,
            MAX_RANGE_EXECUTIONS,
            MAX_RANGE_INSTANT_EVENTS,
            MAX_RANGE_METRICS,
            MAX_RANGE_CYCLES,
        )
    }

    /// Extracts a range-restricted slice of log data with configurable safety caps.
    pub fn get_range_with_caps(
        &self,
        start_us: u64,
        end_us: u64,
        max_executions: usize,
        max_events: usize,
        max_metrics: usize,
        max_cycles: usize,
    ) -> LogRangeData {
        if start_us > end_us {
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
        let mut remaining_executions = max_executions;
        for &cid in &sorted_cids {
            let candidate = self
                .executions_by_cid
                .get(&cid)
                .map(|execs| {
                    slice_partitioned_range(
                        execs,
                        |e| e.start_us.saturating_add(e.duration_us) < start_us,
                        |e| e.start_us <= end_us,
                    )
                })
                .unwrap_or(&[]);
            let (capped, truncated) = slice_capped(candidate, remaining_executions);
            remaining_executions -= capped.len();
            is_truncated |= truncated;
            actual_executions_by_cid.push((cid, capped.to_vec()));
        }

        // Extract instant events per client
        let mut actual_instant_events_by_cid = Vec::with_capacity(sorted_cids.len());
        let mut remaining_events = max_events;
        for &cid in &sorted_cids {
            let candidate = self
                .instant_events_by_cid
                .get(&cid)
                .map(|events| {
                    slice_partitioned_range(
                        events,
                        |e| e.time_us < start_us,
                        |e| e.time_us <= end_us,
                    )
                })
                .unwrap_or(&[]);
            let (capped, truncated) = slice_capped(candidate, remaining_events);
            remaining_events -= capped.len();
            is_truncated |= truncated;
            actual_instant_events_by_cid.push((cid, capped.to_vec()));
        }

        // Extract concurrency metric points including the immediately preceding point for step continuity
        let metrics_candidate = slice_metrics_with_continuity(&self.metrics, start_us, end_us);
        let (capped_metrics, truncated) = slice_capped(metrics_candidate, max_metrics);
        is_truncated |= truncated;
        let actual_metrics = capped_metrics.to_vec();

        // Extract cycles with padding margins for anchor and grid rendering
        let cycle_time_us = (self.summary.config.server_config.cycle_time_ms.max(1) as u64) * 1_000;
        let margin_us = (cycle_time_us * 2).max(100_000);
        let req_cycle_start = start_us.saturating_sub(margin_us);
        let req_cycle_end = end_us.saturating_add(margin_us);
        let cycles_candidate = slice_partitioned_range(
            &self.cycles,
            |c| c.start_us < req_cycle_start,
            |c| c.start_us <= req_cycle_end,
        );
        let (capped_cycles, truncated) = slice_capped(cycles_candidate, max_cycles);
        is_truncated |= truncated;
        let actual_cycles = capped_cycles.to_vec();

        LogRangeData {
            actual_executions_by_cid,
            actual_instant_events_by_cid,
            actual_metrics,
            actual_cycles,
            is_truncated,
        }
    }

    /// Returns a filtered, paginated slice of cross-client anomaly events without mutating the store.
    pub fn get_anomaly_events(
        &self,
        cid: Option<u16>,
        event_types: &[InstantEventType],
        offset: usize,
        limit: usize,
    ) -> AnomalyEventsPage {
        if event_types.is_empty() {
            return AnomalyEventsPage {
                total_count: 0,
                items: Vec::new(),
            };
        }

        let clamped_limit = limit.min(MAX_ANOMALY_EVENTS_PAGE_LIMIT);
        let mut total_count: usize = 0;
        let mut items = Vec::with_capacity(clamped_limit.min(self.anomaly_events.len()));

        for item in &self.anomaly_events {
            if let Some(target_cid) = cid
                && item.cid != target_cid
            {
                continue;
            }
            if !event_types.contains(&item.event_type) {
                continue;
            }
            if total_count >= offset && items.len() < clamped_limit {
                items.push(item.clone());
            }
            total_count += 1;
        }

        AnomalyEventsPage { total_count, items }
    }

    /// Recomputes budget alignment statistics for a client against a candidate `expected_duration_ms`.
    ///
    /// Returns `None` if `cid` is absent or has no completed execution bars.
    pub fn recompute_client_budget(
        &self,
        cid: u16,
        expected_duration_ms: u32,
    ) -> Option<ClientBudgetStats> {
        let client_stats = self
            .summary
            .stats
            .clients
            .iter()
            .find(|client| client.cid == cid)?;
        let duration_stats = client_stats.duration.as_ref()?;
        let executions = self.executions_by_cid.get(&cid)?;

        Some(crate::stats::compute_client_budget(
            executions,
            duration_stats,
            expected_duration_ms,
        ))
    }
}

/* -------------------------------------------------------------------------- */

/// Extracts a contiguous subslice matching a sorted lower/upper predicate pair.
///
/// Both predicates must partition `items` (all `true` elements preceding all `false` elements).
fn slice_partitioned_range<T>(
    items: &[T],
    before_start: impl FnMut(&T) -> bool,
    within_end: impl FnMut(&T) -> bool,
) -> &[T] {
    let start_idx = items.partition_point(before_start);
    let end_idx = start_idx + items[start_idx..].partition_point(within_end);
    &items[start_idx..end_idx]
}

/// Extracts concurrency metric points in `[start_us, end_us]`, including the immediately
/// preceding point for step-chart continuity when the window overlaps the recorded timeline.
fn slice_metrics_with_continuity(
    metrics: &[ActualMetricPoint],
    start_us: u64,
    end_us: u64,
) -> &[ActualMetricPoint] {
    if start_us > end_us {
        return &[];
    }

    let start_idx = metrics.partition_point(|m| m.time_us < start_us);
    // Only extract if the requested window overlaps with the recorded metrics timeline
    if start_idx >= metrics.len() {
        return &[];
    }

    let effective_start_idx = start_idx.saturating_sub(1);
    let end_idx = start_idx + metrics[start_idx..].partition_point(|m| m.time_us <= end_us);
    &metrics[effective_start_idx..end_idx]
}

/// Caps `slice` to at most `max_len` elements and reports whether truncation occurred.
fn slice_capped<T>(slice: &[T], max_len: usize) -> (&[T], bool) {
    if slice.len() > max_len {
        (&slice[..max_len], true)
    } else {
        (slice, false)
    }
}
