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

use crate::types::{ExecutionStatus, InstantEventType};

/* -------------------------------------------------------------------------- */
/* Test Fixtures (Embedded Real Server Logs)                                  */
/* -------------------------------------------------------------------------- */

const LOG_NORMAL: &str = include_str!("../fixtures/logs/server_normal.log");
const LOG_LEAD_OVERRUN: &str =
    include_str!("../fixtures/logs/server_lead_overrun_cascaded_skip.log");
const LOG_FOLLOW_OVERRUN: &str =
    include_str!("../fixtures/logs/server_follow_overrun_autonomous_skip.log");

/* -------------------------------------------------------------------------- */
/* Minimal Synthetic Fixtures for Edge Cases                                  */
/* -------------------------------------------------------------------------- */

/// Minimal synthetic log reproducing UDP retransmissions during startup and runtime.
const SAMPLE_LOG_RETRANSMIT: &str = r#"2026/09/06 00:00:00.000000 [INFO ] main - <CONFIG> [server_config]
2026/09/06 00:00:00.001000 [INFO ] main - <CONFIG> port = 7878
2026/09/06 00:00:00.002000 [INFO ] main - <CONFIG> cycle_time_ms = 50
2026/09/06 00:00:00.003000 [INFO ] main - <CONFIG> stats_interval_cycle = 0
2026/09/06 00:00:00.004000 [INFO ] main - <CONFIG> [[client_configs]]
2026/09/06 00:00:00.005000 [INFO ] main - <CONFIG> client_id = 1
2026/09/06 00:00:00.006000 [INFO ] main - <CONFIG> display_name = "Camera"
2026/09/06 00:00:00.007000 [INFO ] main - <CONFIG> cycle = 1
2026/09/06 00:00:00.008000 [INFO ] main - <CONFIG> cycle_offset = 0
2026/09/06 00:00:00.009000 [INFO ] main - <CONFIG> depends = []
2026/09/06 00:00:00.010000 [INFO ] main - <CONFIG> expected_duration_ms = 10
2026/09/06 00:00:00.050000 [DEBUG] process_starting - <STAT> CYC:-00000000001 CID:001 MID:1 JOIN
2026/09/06 00:00:00.051000 [WARN ] process_starting - <STAT> CYC:-00000000001 CID:001 MID:1 JOIN (Retransmit)
2026/09/06 00:00:00.100000 [DEBUG] process_running - <STAT> CYC:000000000000 START
2026/09/06 00:00:00.101000 [DEBUG] process_running - <STAT> CYC:000000000000 CID:001 MID:2 Ready -> Running (Cycle)
2026/09/06 00:00:00.102000 [WARN ] process_running - <STAT> CYC:000000000000 CID:001 MID:2 Running -> Running (Retransmit)
2026/09/06 00:00:00.111000 [DEBUG] process_running - <STAT> CYC:000000000000 CID:001 MID:2 Running -> Idle
"#;

/* -------------------------------------------------------------------------- */
/* Test Cases                                                                 */
/* -------------------------------------------------------------------------- */

/// Helper to parse log from an in-memory string for unit testing.
fn parse_log_str(content: &str) -> Result<LogStore, String> {
    parse_log(content.as_bytes())
}

/// Validates complete parsing of a normal multi-client execution using real server log.
#[test]
fn test_parse_real_log_normal() {
    let store = parse_log_str(LOG_NORMAL).expect("Failed to parse real normal log");

    // Verify configuration and summary metadata
    assert_eq!(store.summary.config.server_config.port, 7901);
    assert_eq!(store.summary.config.server_config.cycle_time_ms, 50);
    assert_eq!(store.summary.config.client_configs.len(), 3);
    assert_eq!(store.summary.total_cycles, 9); // CYC: 0 to 8
    assert!(!store.summary.is_truncated);
    assert!(store.summary.total_duration_ms > 400);

    // Verify cycle sequence and timing
    let cycles = &store.cycles;
    assert_eq!(cycles.len(), 9);
    assert_eq!(cycles[0].cycle, 0);
    assert_eq!(cycles[0].start_ms, 0);
    assert_eq!(cycles[0].start_jitter_ms, 0.0);

    assert_eq!(cycles[1].cycle, 1);
    assert_eq!(cycles[1].start_ms, 50);

    // Verify task executions by client ID
    // CID 10 (cycle: 2, offset: 0) runs in even cycles (0, 2, 4, 6, 8)
    let execs_cid10 = store.executions_by_cid.get(&10).expect("CID 10 not found");
    assert_eq!(execs_cid10.len(), 5);
    for exec in execs_cid10 {
        assert_eq!(exec.status, ExecutionStatus::Normal);
        assert!(exec.duration_ms >= 10);
        assert!(exec.log_line_no_start > 0);
    }

    // CID 11 (cycle: 2, offset: 0, depends: [10]) runs after CID 10 in even cycles
    let execs_cid11 = store.executions_by_cid.get(&11).expect("CID 11 not found");
    assert_eq!(execs_cid11.len(), 5);
    for exec in execs_cid11 {
        assert_eq!(exec.status, ExecutionStatus::Normal);
        assert!(exec.duration_ms >= 10);
        assert!(exec.log_line_no_start > 0);
    }

    // CID 20 (cycle: 2, offset: 1) runs in odd cycles (1, 3, 5, 7)
    let execs_cid20 = store.executions_by_cid.get(&20).expect("CID 20 not found");
    assert_eq!(execs_cid20.len(), 4);
    for exec in execs_cid20 {
        assert_eq!(exec.status, ExecutionStatus::Normal);
        assert!(exec.duration_ms >= 10);
        assert!(exec.log_line_no_start > 0);
    }

    // Verify instant point events (each client has 4 Idle->Ready transitions and 1 EXIT transition)
    for cid in [10, 11, 20] {
        let evts = store
            .instant_events_by_cid
            .get(&cid)
            .unwrap_or_else(|| panic!("Events for CID {} not found", cid));
        assert_eq!(evts.len(), 5);
        assert_eq!(
            evts.iter()
                .filter(|e| e.event_type == InstantEventType::Ready)
                .count(),
            4
        );
        assert_eq!(
            evts.iter()
                .filter(|e| e.event_type == InstantEventType::Exit)
                .count(),
            1
        );
    }

    // Verify concurrency metrics
    assert!(!store.metrics.is_empty());
    assert!(store.summary.max_concurrency >= 1);
}

/// Validates handling of leading task overrun and cascaded skip using real server log.
#[test]
fn test_parse_real_log_overrun_and_cascaded_skip() {
    let store = parse_log_str(LOG_LEAD_OVERRUN).expect("Failed to parse lead overrun real log");

    // Verify overrun detection on leading task CID 10 (3 executions in cycles 0, 4, 8; all overran)
    let execs_cid10 = store.executions_by_cid.get(&10).expect("CID 10 not found");
    assert_eq!(execs_cid10.len(), 3);
    for exec in execs_cid10 {
        assert_eq!(exec.status, ExecutionStatus::Overrun);
        assert!(exec.duration_ms > 100); // 50ms cycle_time * 2 = 100ms exceeded
    }

    let evts_cid10 = store
        .instant_events_by_cid
        .get(&10)
        .expect("Events for CID 10 not found");
    let overrun_count = evts_cid10
        .iter()
        .filter(|e| e.event_type == InstantEventType::Overrun)
        .count();
    let late_count = evts_cid10
        .iter()
        .filter(|e| e.event_type == InstantEventType::Late)
        .count();
    assert_eq!(overrun_count, 3); // Overrun detected in cycles 2, 6, and 10
    assert_eq!(late_count, 3); // Late completion recorded in cycles 2, 6, and 10

    // Verify cascaded skip on dependent task CID 11 across cycles
    let evts_cid11 = store
        .instant_events_by_cid
        .get(&11)
        .expect("Events for CID 11 not found");
    let skip_count = evts_cid11
        .iter()
        .filter(|e| e.event_type == InstantEventType::Skip)
        .count();
    assert_eq!(skip_count, 3); // Skipped in cycle 2, 6, and 10 due to CID 10 overrun

    // Dependent task CID 11 was skipped in all cycles, so no execution bars exist at all
    let execs_cid11 = store.executions_by_cid.get(&11).expect("CID 11 not found");
    assert!(execs_cid11.is_empty());
}

/// Validates handling of following task overrun and autonomous skip using real server log.
#[test]
fn test_parse_real_log_follow_overrun_and_lead_skip() {
    let store = parse_log_str(LOG_FOLLOW_OVERRUN).expect("Failed to parse follow overrun real log");

    // Verify overrun detection on following task CID 11
    let execs_cid11 = store.executions_by_cid.get(&11).expect("CID 11 not found");
    assert_eq!(execs_cid11.len(), 3); // Started in cycles 0, 4, 8
    let overrun_count = execs_cid11
        .iter()
        .filter(|e| e.status == ExecutionStatus::Overrun)
        .count();
    assert_eq!(overrun_count, 2); // Overran in cycles 0 and 4 (cycle 8 terminated at shutdown)

    let evts_cid11 = store
        .instant_events_by_cid
        .get(&11)
        .expect("Events for CID 11 not found");
    let ov_evts = evts_cid11
        .iter()
        .filter(|e| e.event_type == InstantEventType::Overrun)
        .count();
    let late_evts = evts_cid11
        .iter()
        .filter(|e| e.event_type == InstantEventType::Late)
        .count();
    assert_eq!(ov_evts, 2); // Overrun events in cycles 2 and 6
    assert_eq!(late_evts, 2); // Late recovery events in cycles 2 and 6

    // Verify autonomous skip on next cycle's leading task CID 10 (blocked in cycles 2 and 6)
    let evts_cid10 = store
        .instant_events_by_cid
        .get(&10)
        .expect("Events for CID 10 not found");
    let skip_count = evts_cid10
        .iter()
        .filter(|e| e.event_type == InstantEventType::Skip)
        .count();
    assert_eq!(skip_count, 2); // Skipped in cycle 2 and 6

    // CID 10 executed in cycles 0, 4, 8 and was skipped in cycles 2, 6
    let execs_cid10 = store.executions_by_cid.get(&10).expect("CID 10 not found");
    assert_eq!(execs_cid10.len(), 3);
    for exec in execs_cid10 {
        assert_ne!(exec.cycle, 2);
        assert_ne!(exec.cycle, 6);
    }
}

/// Validates suppression of duplicate executions upon UDP retransmission.
#[test]
fn test_parse_retransmission_guard() {
    let store =
        parse_log_str(SAMPLE_LOG_RETRANSMIT).expect("Failed to parse retransmit synthetic log");

    let events = store
        .instant_events_by_cid
        .get(&1)
        .expect("Events for CID 1 not found");

    // Startup JOIN and JOIN (Retransmit) prior to Cycle 0 START must be ignored.
    // In-cycle Retransmit event must be captured.
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, InstantEventType::Retransmit);
    assert_eq!(events[0].time_ms, 2);

    // Retransmit log must not generate duplicate execution bars.
    let execs = store
        .executions_by_cid
        .get(&1)
        .expect("Executions for CID 1 not found");
    assert_eq!(execs.len(), 1);
    assert_eq!(execs[0].status, ExecutionStatus::Normal);
}

/// Validates error reporting when configuration header is missing.
#[test]
fn test_parse_error_missing_config() {
    let log_without_config =
        "2026/09/06 00:01:24.752311 [DEBUG] process_running - <STAT> CYC:000000000000 START\n";
    let result = parse_log_str(log_without_config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No <CONFIG> lines found"));
}

/// Validates graceful closing of in-flight execution when log ends abruptly.
#[test]
fn test_parse_truncated_log_safety() {
    // Slice real log up to line 50:
    // Line 49 is CYC:0 START, Line 50 is CID:010 Ready -> Running (task start).
    // Truncating at line 50 leaves CID 10 in-flight (before Line 51: Running -> Idle).
    let truncated_log: String = LOG_NORMAL.lines().take(50).collect::<Vec<_>>().join("\n");
    let store = parse_log_str(&truncated_log).expect("Should safely handle truncated log");

    let execs_cid10 = store.executions_by_cid.get(&10).expect("CID 10 not found");
    assert_eq!(execs_cid10.len(), 1);
    assert_eq!(execs_cid10[0].log_line_no_end, None);
    assert_eq!(execs_cid10[0].status, ExecutionStatus::Normal);
}
