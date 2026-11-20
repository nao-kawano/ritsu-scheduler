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
    assert_eq!(range_data.actual_metrics[0].time_us, store.metrics[0].time_us);
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
