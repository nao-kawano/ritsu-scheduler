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

use super::*;
use crate::log_parser::parse_log;
use crate::types::{ExecutionStatus, LogStats};
use rt_config::{SchedulerConfig, ServerConfig};

/* -------------------------------------------------------------------------- */
/* Test Fixtures (Embedded Real Server Logs)                                  */
/* -------------------------------------------------------------------------- */

const LOG_NORMAL: &str = include_str!("../fixtures/logs/server_normal.log");

/* -------------------------------------------------------------------------- */
/* Test Cases                                                                 */
/* -------------------------------------------------------------------------- */

/// Validates extracting the entire time range returns all parsed data across all 5 fields in deterministic CID order.
#[test]
fn test_get_range_full_span() {
    let store = parse_log(LOG_NORMAL.as_bytes()).expect("Failed to parse normal fixture log");
    let total_duration_us = store.summary.total_duration_us;

    let range_data = store.get_range(0, total_duration_us + 100_000);

    // Field 1: Verify truncation flag
    assert!(!range_data.is_truncated);

    // Field 2: Verify actual executions (deterministic CID order and exact per-CID counts)
    let cids_in_execs: Vec<u16> = range_data
        .actual_executions_by_cid
        .iter()
        .map(|(cid, _)| *cid)
        .collect();
    assert_eq!(cids_in_execs, vec![10, 11, 20]);

    let execs_map: HashMap<u16, usize> = range_data
        .actual_executions_by_cid
        .iter()
        .map(|(cid, execs)| (*cid, execs.len()))
        .collect();
    assert_eq!(execs_map.get(&10), Some(&5));
    assert_eq!(execs_map.get(&11), Some(&5));
    assert_eq!(execs_map.get(&20), Some(&4));

    // Field 3: Verify instant events (deterministic CID order and exact per-CID counts)
    let cids_in_events: Vec<u16> = range_data
        .actual_instant_events_by_cid
        .iter()
        .map(|(cid, _)| *cid)
        .collect();
    assert_eq!(cids_in_events, vec![10, 11, 20]);

    let events_map: HashMap<u16, usize> = range_data
        .actual_instant_events_by_cid
        .iter()
        .map(|(cid, events)| (*cid, events.len()))
        .collect();
    assert_eq!(events_map.get(&10), Some(&5));
    assert_eq!(events_map.get(&11), Some(&5));
    assert_eq!(events_map.get(&20), Some(&5));

    // Field 4: Verify concurrency metrics parity and ordering
    assert_eq!(range_data.actual_metrics.len(), store.metrics.len());
    assert_eq!(
        range_data.actual_metrics[0].time_us,
        store.metrics[0].time_us
    );
    assert_eq!(
        range_data.actual_metrics.last().unwrap().time_us,
        store.metrics.last().unwrap().time_us
    );
    for window in range_data.actual_metrics.windows(2) {
        assert!(window[0].time_us <= window[1].time_us);
    }

    // Field 5: Verify total cycles parity and ordering
    assert_eq!(range_data.actual_cycles.len(), store.cycles.len());
    assert_eq!(range_data.actual_cycles[0].cycle, 0);
    assert_eq!(
        range_data.actual_cycles.last().unwrap().cycle,
        store.cycles.last().unwrap().cycle
    );
    for window in range_data.actual_cycles.windows(2) {
        assert!(window[0].start_us <= window[1].start_us);
    }
}

/// Validates extracting a partial time window verifies all 5 fields against strict time and margin boundaries.
#[test]
fn test_get_range_partial_slice() {
    let store = parse_log(LOG_NORMAL.as_bytes()).expect("Failed to parse normal fixture log");

    // Target a window around Cycle 1 and Cycle 2 (roughly 50ms to 149ms)
    let start_us = 50_000;
    let end_us = 149_000;
    let range_data = store.get_range(start_us, end_us);

    // Field 1: Verify truncation flag
    assert!(!range_data.is_truncated);

    // Field 2: Verify all returned executions strictly overlap with [start_us, end_us]
    for (cid, execs) in &range_data.actual_executions_by_cid {
        for exec in execs {
            let exec_end = exec.start_us + exec.duration_us;
            assert!(
                exec.start_us <= end_us && exec_end >= start_us,
                "Execution for CID {} at [{}, {}] does not overlap with requested window [{}, {}]",
                cid,
                exec.start_us,
                exec_end,
                start_us,
                end_us
            );
        }
    }

    // Field 3: Verify all returned instant events strictly fall within [start_us, end_us]
    for (cid, events) in &range_data.actual_instant_events_by_cid {
        for event in events {
            assert!(
                event.time_us >= start_us && event.time_us <= end_us,
                "Instant event for CID {} at {}us falls outside requested window [{}, {}]",
                cid,
                event.time_us,
                start_us,
                end_us
            );
        }
    }

    // Field 4: Verify metrics continuity (includes preceding point if available) and sorting
    assert!(!range_data.actual_metrics.is_empty());
    assert!(range_data.actual_metrics[0].time_us <= start_us);
    if range_data.actual_metrics.len() > 1 {
        for m in &range_data.actual_metrics[1..] {
            assert!(m.time_us >= start_us && m.time_us <= end_us);
        }
    }
    for window in range_data.actual_metrics.windows(2) {
        assert!(window[0].time_us <= window[1].time_us);
    }

    // Field 5: Verify cycle margin inclusion (cycle_time_us * 2 = 100_000us margin around [50_000, 149_000])
    // Expected bounds: max(0, 50_000 - 100_000) = 0us to 149_000 + 100_000 = 249_000us
    assert!(!range_data.actual_cycles.is_empty());
    for cycle in &range_data.actual_cycles {
        assert!(
            cycle.start_us <= end_us + 100_000,
            "Cycle {} start {}us exceeded upper margin boundary",
            cycle.cycle,
            cycle.start_us
        );
    }
}

/// Validates query with inverted arguments returns an empty result across all fields safely.
#[test]
fn test_get_range_inverted_bounds() {
    let store = parse_log(LOG_NORMAL.as_bytes()).expect("Failed to parse normal fixture log");

    let range_data = store.get_range(200_000, 100_000);

    assert!(!range_data.is_truncated);
    assert!(range_data.actual_executions_by_cid.is_empty());
    assert!(range_data.actual_instant_events_by_cid.is_empty());
    assert!(range_data.actual_metrics.is_empty());
    assert!(range_data.actual_cycles.is_empty());
}

/// Validates query out of log bounds returns empty collections across all 5 fields.
#[test]
fn test_get_range_out_of_bounds() {
    let store = parse_log(LOG_NORMAL.as_bytes()).expect("Failed to parse normal fixture log");

    // Request a window far beyond the end of the log
    let range_data = store.get_range(50_000_000, 60_000_000);

    // Field 1: Truncation flag
    assert!(!range_data.is_truncated);

    // Field 2: Executions
    let total_execs: usize = range_data
        .actual_executions_by_cid
        .iter()
        .map(|(_, execs)| execs.len())
        .sum();
    assert_eq!(total_execs, 0);

    // Field 3: Instant events
    let total_events: usize = range_data
        .actual_instant_events_by_cid
        .iter()
        .map(|(_, events)| events.len())
        .sum();
    assert_eq!(total_events, 0);

    // Field 4: Metrics
    assert_eq!(range_data.actual_metrics.len(), 0);

    // Field 5: Cycles
    assert_eq!(range_data.actual_cycles.len(), 0);
}

/// Validates safety cap limits trigger truncation flag and clamp returned element counts across categories.
#[test]
fn test_get_range_safety_cap_truncation() {
    let store = parse_log(LOG_NORMAL.as_bytes()).expect("Failed to parse normal fixture log");
    let total_duration_us = store.summary.total_duration_us;

    // Test execution cap truncation
    let cap_execs = 5;
    let range_exec_capped = store.get_range_with_caps(
        0,
        total_duration_us + 100_000,
        cap_execs,
        MAX_RANGE_INSTANT_EVENTS,
        MAX_RANGE_METRICS,
        MAX_RANGE_CYCLES,
    );
    assert!(range_exec_capped.is_truncated);
    let total_returned_execs: usize = range_exec_capped
        .actual_executions_by_cid
        .iter()
        .map(|(_, execs)| execs.len())
        .sum();
    assert_eq!(total_returned_execs, cap_execs);

    // Test instant event cap truncation
    let cap_events = 7;
    let range_event_capped = store.get_range_with_caps(
        0,
        total_duration_us + 100_000,
        MAX_RANGE_EXECUTIONS,
        cap_events,
        MAX_RANGE_METRICS,
        MAX_RANGE_CYCLES,
    );
    assert!(range_event_capped.is_truncated);
    let total_returned_events: usize = range_event_capped
        .actual_instant_events_by_cid
        .iter()
        .map(|(_, events)| events.len())
        .sum();
    assert_eq!(total_returned_events, cap_events);

    // Test metric cap truncation
    let cap_metrics = 3;
    let range_metric_capped = store.get_range_with_caps(
        0,
        total_duration_us + 100_000,
        MAX_RANGE_EXECUTIONS,
        MAX_RANGE_INSTANT_EVENTS,
        cap_metrics,
        MAX_RANGE_CYCLES,
    );
    assert!(range_metric_capped.is_truncated);
    assert_eq!(range_metric_capped.actual_metrics.len(), cap_metrics);

    // Test cycle cap truncation
    let cap_cycles = 4;
    let range_cycle_capped = store.get_range_with_caps(
        0,
        total_duration_us + 100_000,
        MAX_RANGE_EXECUTIONS,
        MAX_RANGE_INSTANT_EVENTS,
        MAX_RANGE_METRICS,
        cap_cycles,
    );
    assert!(range_cycle_capped.is_truncated);
    assert_eq!(range_cycle_capped.actual_cycles.len(), cap_cycles);
}

/* -------------------------------------------------------------------------- */
/* Synthetic Boundary Helpers & Tests                                         */
/* -------------------------------------------------------------------------- */

/// Builds a minimal synthetic execution bar for boundary testing.
fn make_exec(instance_id: u64, start_us: u64, duration_us: u64) -> ActualExecution {
    ActualExecution {
        log_line_no_start: instance_id,
        log_line_no_end: Some(instance_id + 1),
        instance_id,
        cycle: 0,
        start_us,
        duration_us,
        status: ExecutionStatus::Normal,
    }
}

/// Builds a minimal LogStore populated with synthetic executions and metrics.
fn build_synthetic_store(
    executions_by_cid: HashMap<u16, Vec<ActualExecution>>,
    metrics: Vec<ActualMetricPoint>,
) -> LogStore {
    let summary = LogSummary {
        config: SchedulerConfig {
            server_config: ServerConfig {
                cycle_time_ms: 50,
                ..Default::default()
            },
            client_configs: Vec::new(),
        },
        max_concurrency: 1,
        total_duration_us: 0,
        total_cycles: 0,
        min_start_jitter_us: 0,
        max_start_jitter_us: 0,
        is_truncated: false,
        stats: LogStats::default(),
    };

    LogStore::new(
        summary,
        Vec::new(),
        executions_by_cid,
        HashMap::new(),
        Vec::new(),
        metrics,
    )
}

/// Validates binary-search overlap boundaries for executions across inclusive, gap, and zero-duration cases.
#[test]
fn test_get_range_execution_overlap_boundaries() {
    let mut executions_by_cid = HashMap::new();
    executions_by_cid.insert(
        10,
        vec![
            make_exec(1, 100, 100),  // [100, 200]
            make_exec(2, 300, 0),    // [300, 300] zero-duration
            make_exec(3, 400, 500),  // [400, 900] long overrun
            make_exec(4, 1000, 100), // [1000, 1100]
        ],
    );
    let store = build_synthetic_store(executions_by_cid, Vec::new());

    let ids_in = |start_us: u64, end_us: u64| -> Vec<u64> {
        let range = store.get_range(start_us, end_us);
        assert_eq!(range.actual_executions_by_cid.len(), 1);
        range.actual_executions_by_cid[0]
            .1
            .iter()
            .map(|e| e.instance_id)
            .collect()
    };

    assert_eq!(ids_in(150, 250), vec![1]);
    assert_eq!(ids_in(200, 250), vec![1]);
    assert_eq!(ids_in(201, 250), Vec::<u64>::new());
    assert_eq!(ids_in(50, 100), vec![1]);
    assert_eq!(ids_in(50, 99), Vec::<u64>::new());
    assert_eq!(ids_in(300, 300), vec![2]);
    assert_eq!(ids_in(299, 299), Vec::<u64>::new());
    assert_eq!(ids_in(301, 301), Vec::<u64>::new());
    assert_eq!(ids_in(500, 600), vec![3]);
    assert_eq!(ids_in(901, 999), Vec::<u64>::new());
    assert_eq!(ids_in(200, 400), vec![1, 2, 3]);
}

/// Validates metrics continuity inclusion and out-of-bounds symmetry guard around recorded timeline edges.
#[test]
fn test_get_range_metrics_continuity_boundaries() {
    let metrics = vec![
        ActualMetricPoint {
            time_us: 100,
            running_count: 1,
        },
        ActualMetricPoint {
            time_us: 300,
            running_count: 0,
        },
        ActualMetricPoint {
            time_us: 500,
            running_count: 1,
        },
    ];
    let store = build_synthetic_store(HashMap::new(), metrics);

    let pts_in = |start_us: u64, end_us: u64| -> Vec<(u64, u32)> {
        store
            .get_range(start_us, end_us)
            .actual_metrics
            .into_iter()
            .map(|m| (m.time_us, m.running_count))
            .collect()
    };

    assert_eq!(pts_in(0, 99), Vec::<(u64, u32)>::new());
    assert_eq!(pts_in(150, 250), vec![(100, 1)]);
    assert_eq!(pts_in(300, 350), vec![(100, 1), (300, 0)]);
    assert_eq!(pts_in(501, 600), Vec::<(u64, u32)>::new());
}

/// Validates exact cap matching across multiple CIDs does not mark truncation until an extra item overflows.
#[test]
fn test_get_range_exact_cap_boundary_across_cids() {
    let mut executions_by_cid = HashMap::new();
    executions_by_cid.insert(10, vec![make_exec(1, 0, 40), make_exec(2, 50, 40)]);
    executions_by_cid.insert(20, vec![make_exec(3, 200, 50)]);
    let store = build_synthetic_store(executions_by_cid, Vec::new());

    // Remaining executions reaches 0 after CID 10, but CID 20 has 0 candidates in [0, 100], so no truncation occurs
    let exact_fit = store.get_range_with_caps(0, 100, 2, 100, 100, 100);
    assert!(!exact_fit.is_truncated);
    assert_eq!(exact_fit.actual_executions_by_cid[0].0, 10);
    assert_eq!(exact_fit.actual_executions_by_cid[0].1.len(), 2);
    assert_eq!(exact_fit.actual_executions_by_cid[1].0, 20);
    assert_eq!(exact_fit.actual_executions_by_cid[1].1.len(), 0);

    // Expanding to [0, 300] introduces a candidate in CID 20 that overflows remaining_executions == 0
    let overflow_second_cid = store.get_range_with_caps(0, 300, 2, 100, 100, 100);
    assert!(overflow_second_cid.is_truncated);
    assert_eq!(overflow_second_cid.actual_executions_by_cid[0].0, 10);
    assert_eq!(overflow_second_cid.actual_executions_by_cid[0].1.len(), 2);
    assert_eq!(overflow_second_cid.actual_executions_by_cid[1].0, 20);
    assert_eq!(overflow_second_cid.actual_executions_by_cid[1].1.len(), 0);

    // Increasing cap to 3 accommodates all executions across both CIDs without truncation
    let full_fit = store.get_range_with_caps(0, 300, 3, 100, 100, 100);
    assert!(!full_fit.is_truncated);
    assert_eq!(full_fit.actual_executions_by_cid[0].0, 10);
    assert_eq!(full_fit.actual_executions_by_cid[0].1.len(), 2);
    assert_eq!(full_fit.actual_executions_by_cid[1].0, 20);
    assert_eq!(full_fit.actual_executions_by_cid[1].1.len(), 1);
}
