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
use crate::log_store::{LogStore, MAX_ANOMALY_EVENTS_PAGE_LIMIT};
use crate::types::LogSummary;
use rt_config::{ClientConfig, SchedulerConfig, ServerConfig};

/* -------------------------------------------------------------------------- */
/* Test Fixtures & Synthetic Builders                                         */
/* -------------------------------------------------------------------------- */

const LOG_NORMAL: &str = include_str!("../fixtures/logs/server_normal.log");
const LOG_LEAD_OVERRUN: &str =
    include_str!("../fixtures/logs/server_lead_overrun_cascaded_skip.log");
const LOG_FOLLOW_OVERRUN: &str =
    include_str!("../fixtures/logs/server_follow_overrun_autonomous_skip.log");

fn make_config(clients: Vec<(u16, u32)>) -> SchedulerConfig {
    let client_configs = clients
        .into_iter()
        .map(|(cid, expected_ms)| {
            ClientConfig::builder(cid, 1)
                .expected_duration_ms(expected_ms)
                .build()
                .unwrap()
        })
        .collect();
    SchedulerConfig {
        server_config: ServerConfig {
            port: 7878,
            cycle_time_ms: 50,
            ..Default::default()
        },
        client_configs,
    }
}

fn make_completed_exec(
    id: u64,
    cycle: u64,
    start_us: u64,
    duration_us: u64,
    status: ExecutionStatus,
) -> ActualExecution {
    ActualExecution {
        log_line_no_start: id * 2,
        log_line_no_end: Some(id * 2 + 1),
        instance_id: id,
        cycle,
        start_us,
        duration_us,
        status,
    }
}

fn make_incomplete_exec(
    id: u64,
    cycle: u64,
    start_us: u64,
    duration_us: u64,
    status: ExecutionStatus,
) -> ActualExecution {
    ActualExecution {
        log_line_no_start: id * 2,
        log_line_no_end: None,
        instance_id: id,
        cycle,
        start_us,
        duration_us,
        status,
    }
}

fn make_event(
    line_no: u64,
    id: u64,
    cycle: u64,
    time_us: u64,
    event_type: InstantEventType,
) -> ActualInstantEvent {
    ActualInstantEvent {
        log_line_no: line_no,
        instance_id: id,
        cycle,
        time_us,
        event_type,
    }
}

fn make_cycle(cycle: u64, start_us: u64, start_jitter_us: i64) -> ActualCycle {
    ActualCycle {
        cycle,
        start_us,
        start_jitter_us,
    }
}

/// Extracts anomaly events from a per-client event map for synthetic unit tests.
fn make_anomaly_events(
    instant_events_by_cid: &HashMap<u16, Vec<ActualInstantEvent>>,
) -> Vec<AnomalyEventItem> {
    let mut items = Vec::new();
    for (&cid, events) in instant_events_by_cid {
        for event in events {
            if event.event_type.is_anomaly() {
                items.push(AnomalyEventItem {
                    log_line_no: event.log_line_no,
                    instance_id: event.instance_id,
                    cid,
                    cycle: event.cycle,
                    time_us: event.time_us,
                    event_type: event.event_type,
                });
            }
        }
    }
    items.sort_by_key(|item| (item.time_us, item.log_line_no, item.instance_id));
    items
}

/// Extracts `DurationStats` from completed executions for synthetic unit tests.
fn extract_duration_stats(executions: &[ActualExecution]) -> Option<DurationStats> {
    summarize_client_executions(executions)
        .1
        .map(|(dur, _)| dur)
}

/* -------------------------------------------------------------------------- */
/* Synthetic Unit & Boundary Tests                                            */
/* -------------------------------------------------------------------------- */

#[test]
fn test_stats_empty_log() {
    let config = make_config(vec![(10, 20)]);
    let stats = compute_log_stats(&config, &[], &HashMap::new(), &HashMap::new(), &[], &[], 0);

    assert_eq!(stats.system.total_cycles, 0);
    assert_eq!(stats.system.clean_cycles, 0);
    assert_eq!(stats.system.clean_cycle_ratio, 0.0);
    assert_eq!(stats.system.overrun_count, 0);

    assert!(stats.jitter.is_none());
    assert_eq!(stats.concurrency.total_duration_us, 0);
    assert_eq!(stats.concurrency.peak_concurrency, 0);
    assert!(stats.concurrency.peak_time_us.is_none());

    assert_eq!(stats.clients.len(), 1);
    assert_eq!(stats.clients[0].cid, 10);
    assert_eq!(stats.clients[0].normal_count, 0);
    assert_eq!(stats.clients[0].incomplete_count, 0);
    assert!(stats.clients[0].duration.is_none());
    assert!(stats.clients[0].budget.is_none());
}

#[test]
fn test_stats_zero_exec_and_incomplete_bar_exclusion() {
    // CID 10: only incomplete overrun bar (truncated mid-overrun at log end) + Overrun event.
    // CID 20: 1 completed Normal bar + 1 incomplete Normal bar.
    // CID 30: configured, 0 executions, 1 Skip event.
    let config = make_config(vec![(30, 10), (10, 10), (20, 10)]);

    let mut execs = HashMap::new();
    execs.insert(
        10,
        vec![make_incomplete_exec(
            1,
            0,
            100,
            60_000,
            ExecutionStatus::Overrun,
        )],
    );
    execs.insert(
        20,
        vec![
            make_completed_exec(2, 0, 200, 5_000, ExecutionStatus::Normal),
            make_incomplete_exec(3, 1, 50_200, 3_000, ExecutionStatus::Normal),
        ],
    );

    let mut events = HashMap::new();
    events.insert(
        10,
        vec![make_event(10, 1, 1, 50_000, InstantEventType::Overrun)],
    );
    events.insert(
        30,
        vec![make_event(11, 2, 1, 50_100, InstantEventType::Skip)],
    );

    let cycles = vec![make_cycle(0, 0, 0), make_cycle(1, 50_000, 0)];
    let anomalies = make_anomaly_events(&events);
    let stats = compute_log_stats(&config, &cycles, &execs, &events, &anomalies, &[], 100_000);

    // Clients are ordered by ascending CID.
    let cids: Vec<u16> = stats.clients.iter().map(|c| c.cid).collect();
    assert_eq!(cids, vec![10, 20, 30]);

    // CID 10: incomplete Overrun bar is excluded from completed overrun_count & duration/budget,
    // while SystemStats.overrun_count preserves the raw Overrun instant event count.
    let c10 = &stats.clients[0];
    assert_eq!(c10.normal_count, 0);
    assert_eq!(c10.overrun_count, 0);
    assert_eq!(c10.incomplete_count, 1);
    assert!(c10.duration.is_none());
    assert!(c10.budget.is_none());
    assert_eq!(stats.system.overrun_count, 1);

    // CID 20: only the 5_000us completed bar is counted in duration/budget (N=1 -> StdDev == 0.0).
    let c20 = &stats.clients[1];
    assert_eq!(c20.normal_count, 1);
    assert_eq!(c20.incomplete_count, 1);
    let dur20 = c20.duration.as_ref().unwrap();
    assert_eq!(dur20.sample_count, 1);
    assert_eq!(dur20.min_us, 5_000);
    assert_eq!(dur20.max_us, 5_000);
    assert_eq!(dur20.std_dev_us, 0.0);

    // CID 30: zero executions, Skip counted.
    let c30 = &stats.clients[2];
    assert_eq!(c30.skip_count, 1);
    assert!(c30.duration.is_none());
    assert!(c30.budget.is_none());
}

#[test]
fn test_stats_duration_percentiles_stddev_and_budget() {
    let config = make_config(vec![(1, 15)]);

    // 100 samples spanning 1_000..=100_000 us, placing the 100_000 us WCET sample at cycle 50.
    let mut execs_vec = Vec::new();
    for i in 1..=98u64 {
        let dur = i * 1_000;
        let status = if dur > 50_000 {
            ExecutionStatus::Overrun
        } else {
            ExecutionStatus::Normal
        };
        execs_vec.push(make_completed_exec(i, i, i * 50_000, dur, status));
    }
    execs_vec.push(make_completed_exec(
        99,
        50,
        50 * 50_000,
        100_000,
        ExecutionStatus::Overrun,
    ));
    execs_vec.push(make_completed_exec(
        100,
        99,
        99 * 50_000,
        99_000,
        ExecutionStatus::Overrun,
    ));

    let mut execs = HashMap::new();
    execs.insert(1, execs_vec);

    let stats = compute_log_stats(&config, &[], &execs, &HashMap::new(), &[], &[], 0);
    let c1 = &stats.clients[0];
    let dur = c1.duration.as_ref().unwrap();

    assert_eq!(dur.sample_count, 100);
    assert_eq!(dur.min_us, 1_000);
    assert_eq!(dur.max_us, 100_000);
    assert_eq!(dur.p50_us, 50_000);
    assert_eq!(dur.p95_us, 95_000);
    assert_eq!(dur.p99_us, 99_000);
    assert!((dur.avg_us - 50_500.0).abs() < 1e-6);
    assert_eq!(dur.wcet_cycle, 50);
    assert_eq!(dur.wcet_time_us, 50 * 50_000);

    // Population StdDev of 1..=100 * 1000: sqrt((100^2 - 1)/12) * 1000
    let expected_std = ((10_000.0 - 1.0) / 12.0_f64).sqrt() * 1_000.0;
    assert!((dur.std_dev_us - expected_std).abs() < 1e-6);

    // Budget vs expected = 15_000us: 15 covered (1..=15), 85 exceeded (16..=100 -> exceed 1..=85 * 1000)
    let budget = c1.budget.as_ref().unwrap();
    assert_eq!(budget.expected_duration_us, 15_000);
    assert!((budget.coverage_ratio - 0.15).abs() < 1e-9);
    assert_eq!(budget.exceed_count, 85);
    assert_eq!(budget.max_exceed_us, 85_000);
    assert!((budget.avg_exceed_us - 43_000.0).abs() < 1e-6);
    assert_eq!(budget.margin_p99_us, 15_000 - 99_000);
    assert_eq!(budget.margin_wcet_us, 15_000 - 100_000);

    // Earliest WCET tie check (cycle 3 must win over cycle 7 when durations are equal).
    let tie_execs = vec![
        make_completed_exec(1, 3, 300, 5_000, ExecutionStatus::Normal),
        make_completed_exec(2, 7, 700, 5_000, ExecutionStatus::Normal),
    ];
    let tie_dur = extract_duration_stats(&tie_execs).unwrap();
    assert_eq!(tie_dur.wcet_cycle, 3);
    assert_eq!(tie_dur.wcet_time_us, 300);
}

#[test]
fn test_stats_duration_histogram_overflow_and_all_zero() {
    // 99 samples at 10_000us -> P99 = 10_000us; 1 outlier at 500_000us must land in overflow_count.
    let mut execs = Vec::new();
    for i in 1..=99u64 {
        execs.push(make_completed_exec(
            i,
            i,
            i * 1_000,
            10_000,
            ExecutionStatus::Normal,
        ));
    }
    execs.push(make_completed_exec(
        100,
        100,
        100_000,
        500_000,
        ExecutionStatus::Overrun,
    ));

    let dur_stats = extract_duration_stats(&execs).unwrap();
    assert_eq!(dur_stats.p99_us, 10_000);
    let budget = compute_client_budget(&execs, &dur_stats, 10);
    let hist = &budget.duration_histogram;

    assert_eq!(hist.origin_us, 0);
    assert_eq!(hist.underflow_count, 0);
    assert_eq!(hist.overflow_count, 1);
    assert_eq!(hist.total_count, 100);
    let bin_sum: u64 = hist.bins.iter().map(|b| b.count).sum();
    assert_eq!(bin_sum, 99);
    // Verify bins are contiguous starting at 0 with uniform step_us.
    assert_eq!(hist.bins[0].start_us, 0);
    for w in hist.bins.windows(2) {
        assert_eq!(w[0].end_us, w[1].start_us);
        assert_eq!((w[0].end_us - w[0].start_us) as u64, hist.step_us);
    }

    // All-zero duration and expected_duration_ms == 0 fallback -> 1 bin [0, 1_000).
    let zero_execs = vec![make_completed_exec(1, 0, 0, 0, ExecutionStatus::Normal)];
    let zero_dur = extract_duration_stats(&zero_execs).unwrap();
    let zero_budget = compute_client_budget(&zero_execs, &zero_dur, 0);
    let zero_hist = &zero_budget.duration_histogram;
    assert_eq!(zero_hist.step_us, 1_000);
    assert_eq!(zero_hist.bins.len(), 1);
    assert_eq!(zero_hist.bins[0].start_us, 0);
    assert_eq!(zero_hist.bins[0].end_us, 1_000);
    assert_eq!(zero_hist.bins[0].count, 1);
    assert_eq!(zero_hist.overflow_count, 0);
}

#[test]
fn test_stats_clean_cycle_ratio_rules() {
    let config = make_config(vec![(1, 10), (2, 10)]);
    let cycles: Vec<ActualCycle> = (0..6).map(|c| make_cycle(c, c * 50_000, 0)).collect();

    let mut events = HashMap::new();
    // Cycle 0: Retransmit + Exit -> MUST remain clean
    // Cycle 1: Overrun + Late in same cycle -> counts as 1 dirty cycle
    // Cycle 2: Skip -> dirty
    // Cycle 3: Error -> dirty
    // Cycle 4: Late -> dirty
    // Cycle 5: Ready only -> clean
    // Cycle 999 (absent from `cycles`): Overrun -> tallied in overrun_count without distorting clean_cycles
    events.insert(
        1,
        vec![
            make_event(1, 1, 0, 100, InstantEventType::Retransmit),
            make_event(2, 2, 0, 200, InstantEventType::Exit),
            make_event(3, 3, 1, 50_000, InstantEventType::Overrun),
            make_event(4, 4, 1, 55_000, InstantEventType::Late),
            make_event(5, 5, 3, 150_000, InstantEventType::Error),
            make_event(6, 6, 5, 250_000, InstantEventType::Ready),
            make_event(9, 9, 999, 300_000, InstantEventType::Overrun),
        ],
    );
    events.insert(
        2,
        vec![
            make_event(7, 7, 2, 100_000, InstantEventType::Skip),
            make_event(8, 8, 4, 200_000, InstantEventType::Late),
        ],
    );

    let anomalies = make_anomaly_events(&events);
    let stats = compute_log_stats(
        &config,
        &cycles,
        &HashMap::new(),
        &events,
        &anomalies,
        &[],
        300_000,
    );
    assert_eq!(stats.system.total_cycles, 6);
    assert_eq!(stats.system.clean_cycles, 2); // Cycles 0 and 5
    assert!((stats.system.clean_cycle_ratio - (2.0 / 6.0)).abs() < 1e-9);
    assert_eq!(stats.system.overrun_count, 2);
    assert_eq!(stats.system.skip_count, 1);
    assert_eq!(stats.system.late_count, 2);
    assert_eq!(stats.system.retransmit_count, 1);
    assert_eq!(stats.system.error_count, 1);
}

#[test]
fn test_stats_jitter_basics_and_top50_outliers() {
    // 1 cycle only -> None (Cycle 0 excluded).
    assert!(compute_jitter_stats(&[make_cycle(0, 0, 0)]).is_none());

    // 61 cycles: Cycle 0 has large bogus value that must be ignored;
    // Cycles 1..=60 have alternating signs with magnitudes 10..=600 us.
    let mut cycles = vec![make_cycle(0, 0, 999_999)];
    for i in 1..=60u64 {
        let sign = if i % 2 == 0 { 1 } else { -1 };
        cycles.push(make_cycle(i, i * 50_000, sign * (i as i64) * 10));
    }

    let j = compute_jitter_stats(&cycles).expect("jitter stats should be Some");
    assert_eq!(j.sample_count, 60);
    assert_eq!(j.min_us, -590);
    assert_eq!(j.min_cycle, 59);
    assert_eq!(j.min_time_us, 59 * 50_000);
    assert_eq!(j.max_us, 600);
    assert_eq!(j.max_cycle, 60);
    assert_eq!(j.max_time_us, 60 * 50_000);
    // Sum of (-10 + 20 - 30 + ... + 600) = 30 * 10 = 300 => avg = 5.0
    assert!((j.avg_us - 5.0).abs() < 1e-9);
    // Abs values 10..=600 step 10 => avg = 305.0, P95 = 570, P99 = 600
    assert!((j.abs_avg_us - 305.0).abs() < 1e-9);
    assert_eq!(j.abs_p95_us, 570);
    assert_eq!(j.abs_p99_us, 600);

    // Top 50 outliers: cycles 60 down to 11 in descending |jitter_us| order.
    assert_eq!(j.outliers.len(), 50);
    assert_eq!(j.outliers[0].cycle, 60);
    assert_eq!(j.outliers[0].jitter_us, 600);
    assert_eq!(j.outliers[1].cycle, 59);
    assert_eq!(j.outliers[1].jitter_us, -590);
    assert_eq!(j.outliers[49].cycle, 11);
    assert_eq!(j.outliers[49].jitter_us, -110);

    // Verify tie-breaking at the 50th-rank boundary: equal |jitter_us| prefers smaller cycle number.
    let mut tie_cycles = vec![make_cycle(0, 0, 0)];
    for i in 1..=49u64 {
        tie_cycles.push(make_cycle(i, i * 50_000, 500));
    }
    tie_cycles.push(make_cycle(60, 60 * 50_000, 100));
    tie_cycles.push(make_cycle(55, 55 * 50_000, -100));
    tie_cycles.push(make_cycle(70, 70 * 50_000, -100));

    let tie_jitter = compute_jitter_stats(&tie_cycles).expect("tie jitter stats should be Some");
    assert_eq!(tie_jitter.outliers.len(), 50);
    assert_eq!(tie_jitter.outliers[49].cycle, 55);
    assert_eq!(tie_jitter.outliers[49].jitter_us, -100);
}

#[test]
fn test_stats_jitter_histogram_dynamic_asymmetric() {
    // Positive-only jitter -> origin_us == 0 (0 negative bins).
    let pos_cycles = vec![
        make_cycle(0, 0, 0),
        make_cycle(1, 50_000, 100),
        make_cycle(2, 100_000, 200),
        make_cycle(3, 150_000, 300),
    ];
    let pos_hist = compute_jitter_stats(&pos_cycles).unwrap().histogram;
    assert_eq!(pos_hist.origin_us, 0);
    assert_eq!(pos_hist.underflow_count, 0);
    assert_eq!(pos_hist.total_count, 3);
    assert!(pos_hist.bins.iter().all(|b| b.start_us >= 0));

    // Negative-only jitter -> upper_bound == 0 (0 positive bins).
    let neg_cycles = vec![
        make_cycle(0, 0, 0),
        make_cycle(1, 50_000, -100),
        make_cycle(2, 100_000, -250),
    ];
    let neg_hist = compute_jitter_stats(&neg_cycles).unwrap().histogram;
    assert!(neg_hist.origin_us < 0);
    assert_eq!(neg_hist.bins.last().unwrap().end_us, 0);
    assert_eq!(neg_hist.overflow_count, 0);
    assert_eq!(neg_hist.total_count, 2);

    // Mixed jitter with extreme underflow and overflow outliers, ensuring 0 is a bin boundary.
    // Use 101 samples per sign so P99 picks 1_000 and the +-100_000 outlier exceeds 1.2 * P99.
    let mut mixed_cycles = vec![make_cycle(0, 0, 0)];
    for i in 1..=200u64 {
        let val = if i % 2 == 0 { 1_000 } else { -1_000 };
        mixed_cycles.push(make_cycle(i, i * 50_000, val));
    }
    mixed_cycles.push(make_cycle(201, 10_050_000, 100_000));
    mixed_cycles.push(make_cycle(202, 10_100_000, -100_000));
    let mixed_hist = compute_jitter_stats(&mixed_cycles).unwrap().histogram;
    assert!(mixed_hist.bins.iter().any(|b| b.start_us == 0));
    assert!(mixed_hist.bins.iter().any(|b| b.end_us == 0));
    assert_eq!(mixed_hist.underflow_count, 1);
    assert_eq!(mixed_hist.overflow_count, 1);
    assert_eq!(mixed_hist.total_count, 202);

    // All-zero jitter -> single bin [0, 1_000) with step_us = 1_000.
    let zero_cycles = vec![
        make_cycle(0, 0, 0),
        make_cycle(1, 50_000, 0),
        make_cycle(2, 100_000, 0),
    ];
    let zero_hist = compute_jitter_stats(&zero_cycles).unwrap().histogram;
    assert_eq!(zero_hist.step_us, 1_000);
    assert_eq!(zero_hist.origin_us, 0);
    assert_eq!(zero_hist.bins.len(), 1);
    assert_eq!(zero_hist.bins[0].start_us, 0);
    assert_eq!(zero_hist.bins[0].end_us, 1_000);
    assert_eq!(zero_hist.bins[0].count, 2);
}

#[test]
fn test_stats_concurrency_window_and_zero_peak() {
    // Carry prior level=2 from t=50 into window [100, 400):
    // [100, 150) -> level 2 (50us)
    // [150, 250) -> level 3 (100us) -> peak at t=150
    // [250, 300) -> level 0 (50us)
    // [300, 400) -> level 1 (100us, trailing segment up to window_end_us)
    let points = vec![
        ActualMetricPoint {
            time_us: 50,
            running_count: 2,
        },
        ActualMetricPoint {
            time_us: 150,
            running_count: 3,
        },
        ActualMetricPoint {
            time_us: 250,
            running_count: 0,
        },
        ActualMetricPoint {
            time_us: 300,
            running_count: 1,
        },
        ActualMetricPoint {
            time_us: 450,
            running_count: 5,
        }, // Outside window
    ];

    let stats = compute_concurrency_stats(&points, 100, 400);
    assert_eq!(stats.total_duration_us, 300);
    assert_eq!(stats.active_duration_us, 250);
    assert!((stats.active_ratio - (250.0 / 300.0)).abs() < 1e-9);
    // Weighted active sum: (1 * 100 + 2 * 50 + 3 * 100) = 500 / 250 = 2.0
    assert!((stats.active_avg_concurrency - 2.0).abs() < 1e-9);
    assert_eq!(stats.peak_concurrency, 3);
    assert_eq!(stats.peak_time_us, Some(150));
    assert_eq!(stats.distribution.len(), 4);
    assert_eq!(stats.distribution[0].duration_us, 50);
    assert_eq!(stats.distribution[1].duration_us, 100);
    assert_eq!(stats.distribution[2].duration_us, 50);
    assert_eq!(stats.distribution[3].duration_us, 100);

    // Zero concurrency across non-empty window -> peak_time_us must be None.
    let idle_points = vec![ActualMetricPoint {
        time_us: 0,
        running_count: 0,
    }];
    let idle_stats = compute_concurrency_stats(&idle_points, 0, 100);
    assert_eq!(idle_stats.peak_concurrency, 0);
    assert!(idle_stats.peak_time_us.is_none());
    assert_eq!(idle_stats.active_duration_us, 0);
    assert_eq!(idle_stats.active_avg_concurrency, 0.0);
    assert_eq!(idle_stats.distribution.len(), 1);
    assert_eq!(idle_stats.distribution[0].ratio, 1.0);
}

#[test]
fn test_store_recompute_client_budget_immutable() {
    let config = make_config(vec![(10, 10), (20, 10)]);
    let mut execs = HashMap::new();
    execs.insert(
        10,
        vec![
            make_completed_exec(1, 0, 0, 8_000, ExecutionStatus::Normal),
            make_completed_exec(2, 1, 50_000, 15_000, ExecutionStatus::Normal),
        ],
    );
    let stats = compute_log_stats(&config, &[], &execs, &HashMap::new(), &[], &[], 100_000);
    let summary = LogSummary {
        config,
        max_concurrency: 1,
        total_duration_us: 100_000,
        total_cycles: 0,
        min_start_jitter_us: 0,
        max_start_jitter_us: 0,
        is_truncated: false,
        stats,
    };
    let store = LogStore::new(
        summary,
        Vec::new(),
        execs,
        HashMap::new(),
        Vec::new(),
        Vec::new(),
    );

    // Baseline budget at expected=10ms: 1/2 covered (50%).
    let baseline = store.summary.stats.clients[0].budget.as_ref().unwrap();
    assert!((baseline.coverage_ratio - 0.5).abs() < 1e-9);
    assert_eq!(baseline.exceed_count, 1);

    // Recompute at expected=20ms: 2/2 covered (100%), margin_wcet = +5_000us.
    let recomputed = store
        .recompute_client_budget(10, 20)
        .expect("recomputed budget should be Some");
    assert_eq!(recomputed.expected_duration_us, 20_000);
    assert!((recomputed.coverage_ratio - 1.0).abs() < 1e-9);
    assert_eq!(recomputed.exceed_count, 0);
    assert_eq!(recomputed.margin_wcet_us, 5_000);

    // Store baseline remains untouched (immutable LogStore).
    let baseline_after = store.summary.stats.clients[0].budget.as_ref().unwrap();
    assert_eq!(baseline_after.expected_duration_us, 10_000);
    assert!((baseline_after.coverage_ratio - 0.5).abs() < 1e-9);

    // Client with 0 executions and unknown CID return None.
    assert!(store.recompute_client_budget(20, 20).is_none());
    assert!(store.recompute_client_budget(999, 20).is_none());
}

#[test]
fn test_store_get_anomaly_events_filtering_and_paging() {
    let config = make_config(vec![(10, 10), (20, 10)]);
    let mut events = HashMap::new();
    // Include Ready/Exit (must be excluded from anomaly_events) and tie timestamps to check line_no order.
    events.insert(
        10,
        vec![
            make_event(10, 1, 0, 1_000, InstantEventType::Ready),
            make_event(25, 2, 1, 50_000, InstantEventType::Overrun),
            make_event(40, 3, 2, 100_000, InstantEventType::Retransmit),
            make_event(50, 4, 3, 150_000, InstantEventType::Exit),
        ],
    );
    events.insert(
        20,
        vec![
            make_event(20, 5, 1, 50_000, InstantEventType::Skip),
            make_event(30, 6, 1, 60_000, InstantEventType::Late),
            make_event(45, 7, 2, 120_000, InstantEventType::Error),
        ],
    );

    let anomalies = make_anomaly_events(&events);
    let stats = compute_log_stats(
        &config,
        &[],
        &HashMap::new(),
        &events,
        &anomalies,
        &[],
        200_000,
    );
    let summary = LogSummary {
        config,
        max_concurrency: 0,
        total_duration_us: 200_000,
        total_cycles: 0,
        min_start_jitter_us: 0,
        max_start_jitter_us: 0,
        is_truncated: false,
        stats,
    };
    let store = LogStore::new(
        summary,
        Vec::new(),
        HashMap::new(),
        events,
        anomalies,
        Vec::new(),
    );

    // 5 total anomalies sorted chronologically:
    // (50_000, line 20, CID 20 Skip), (50_000, line 25, CID 10 Overrun),
    // (60_000, line 30, CID 20 Late), (100_000, line 40, CID 10 Retransmit), (120_000, line 45, CID 20 Error)
    let all_types = vec![
        InstantEventType::Overrun,
        InstantEventType::Skip,
        InstantEventType::Late,
        InstantEventType::Retransmit,
        InstantEventType::Error,
    ];
    let page1 = store.get_anomaly_events(None, &all_types, 0, 2);
    assert_eq!(page1.total_count, 5);
    assert_eq!(page1.items.len(), 2);
    assert_eq!(page1.items[0].cid, 20);
    assert_eq!(page1.items[0].event_type, InstantEventType::Skip);
    assert_eq!(page1.items[1].cid, 10);
    assert_eq!(page1.items[1].event_type, InstantEventType::Overrun);

    // Paging offset & out-of-range offset
    let page2 = store.get_anomaly_events(None, &all_types, 3, 10);
    assert_eq!(page2.total_count, 5);
    assert_eq!(page2.items.len(), 2);
    assert_eq!(page2.items[0].event_type, InstantEventType::Retransmit);
    assert_eq!(page2.items[1].event_type, InstantEventType::Error);

    let empty_offset = store.get_anomaly_events(None, &all_types, 10, 10);
    assert_eq!(empty_offset.total_count, 5);
    assert!(empty_offset.items.is_empty());

    // CID + event_type filtering
    let cid10_overrun = store.get_anomaly_events(Some(10), &[InstantEventType::Overrun], 0, 50);
    assert_eq!(cid10_overrun.total_count, 1);
    assert_eq!(cid10_overrun.items[0].log_line_no, 25);

    // Empty event_types -> 0 results
    let no_types = store.get_anomaly_events(None, &[], 0, 50);
    assert_eq!(no_types.total_count, 0);
    assert!(no_types.items.is_empty());

    // Verify limit clamp at MAX_ANOMALY_EVENTS_PAGE_LIMIT
    let mut bulk_events = HashMap::new();
    let many: Vec<ActualInstantEvent> = (0..(MAX_ANOMALY_EVENTS_PAGE_LIMIT + 50))
        .map(|i| {
            make_event(
                i as u64 + 1,
                i as u64 + 1,
                0,
                i as u64 * 10,
                InstantEventType::Late,
            )
        })
        .collect();
    bulk_events.insert(10, many);
    let bulk_anomalies = make_anomaly_events(&bulk_events);
    let bulk_store = LogStore::new(
        store.summary.clone(),
        Vec::new(),
        HashMap::new(),
        bulk_events,
        bulk_anomalies,
        Vec::new(),
    );
    let clamped = bulk_store.get_anomaly_events(
        None,
        &[InstantEventType::Late],
        0,
        MAX_ANOMALY_EVENTS_PAGE_LIMIT + 500,
    );
    assert_eq!(clamped.total_count, MAX_ANOMALY_EVENTS_PAGE_LIMIT + 50);
    assert_eq!(clamped.items.len(), MAX_ANOMALY_EVENTS_PAGE_LIMIT);
}

/* -------------------------------------------------------------------------- */
/* Real Log Fixture Integration Tests                                         */
/* -------------------------------------------------------------------------- */

#[test]
fn test_stats_real_log_normal() {
    let store = parse_log(LOG_NORMAL.as_bytes()).expect("Failed to parse server_normal.log");
    let stats = &store.summary.stats;

    assert_eq!(stats.system.total_cycles, 9);
    assert_eq!(stats.system.clean_cycles, 9);
    assert!((stats.system.clean_cycle_ratio - 1.0).abs() < 1e-9);
    assert_eq!(stats.system.overrun_count, 0);
    assert_eq!(stats.system.skip_count, 0);
    assert_eq!(stats.system.late_count, 0);
    assert_eq!(stats.system.retransmit_count, 0);
    assert_eq!(stats.system.error_count, 0);
    assert!(store.anomaly_events.is_empty());

    assert_eq!(stats.clients.len(), 3);
    assert_eq!(stats.clients[0].cid, 10);
    assert_eq!(stats.clients[0].normal_count, 5);
    assert_eq!(stats.clients[0].exit_count, 1);
    assert_eq!(stats.clients[1].cid, 11);
    assert_eq!(stats.clients[1].normal_count, 5);
    assert_eq!(stats.clients[2].cid, 20);
    assert_eq!(stats.clients[2].normal_count, 4);

    let jitter = stats.jitter.as_ref().expect("jitter should be present");
    assert_eq!(jitter.sample_count, 8); // 9 cycles minus Cycle 0
    assert_eq!(
        stats.concurrency.total_duration_us,
        store.summary.total_duration_us
    );
    assert!(stats.concurrency.active_duration_us > 0);
    assert!(stats.concurrency.peak_concurrency >= 1);
    assert!(stats.concurrency.peak_time_us.is_some());
}

#[test]
fn test_stats_real_log_lead_overrun_cascaded_skip() {
    let store = parse_log(LOG_LEAD_OVERRUN.as_bytes())
        .expect("Failed to parse server_lead_overrun_cascaded_skip.log");
    let stats = &store.summary.stats;

    // 11 cycles (0..=10); cycles 2, 6, 10 have Overrun + Late + Skip => 8 clean cycles.
    assert_eq!(stats.system.total_cycles, 11);
    assert_eq!(stats.system.clean_cycles, 8);
    assert!((stats.system.clean_cycle_ratio - (8.0 / 11.0)).abs() < 1e-9);
    assert_eq!(stats.system.overrun_count, 3);
    assert_eq!(stats.system.late_count, 3);
    assert_eq!(stats.system.skip_count, 3);
    assert_eq!(store.anomaly_events.len(), 9);

    let c10 = stats.clients.iter().find(|c| c.cid == 10).unwrap();
    assert_eq!(c10.overrun_count, 3);
    assert!(c10.duration.is_some());

    let c11 = stats.clients.iter().find(|c| c.cid == 11).unwrap();
    assert_eq!(c11.skip_count, 3);
    assert!(c11.duration.is_none());
    assert!(c11.budget.is_none());
}

#[test]
fn test_stats_real_log_follow_overrun_autonomous_skip() {
    let store = parse_log(LOG_FOLLOW_OVERRUN.as_bytes())
        .expect("Failed to parse server_follow_overrun_autonomous_skip.log");
    let stats = &store.summary.stats;

    // 9 cycles (0..=8); cycles 2 and 6 have Overrun + Late + Skip => 7 clean cycles.
    assert_eq!(stats.system.total_cycles, 9);
    assert_eq!(stats.system.clean_cycles, 7);
    assert!((stats.system.clean_cycle_ratio - (7.0 / 9.0)).abs() < 1e-9);
    assert_eq!(stats.system.overrun_count, 2);
    assert_eq!(stats.system.late_count, 2);
    assert_eq!(stats.system.skip_count, 2);
    assert_eq!(store.anomaly_events.len(), 6);
}

#[test]
fn test_nearest_rank_abs_percentile_signed_boundaries() {
    // Single-element slices across zero, negative, and positive signs.
    for pct in [1, 50, 95, 99] {
        assert_eq!(nearest_rank_abs_percentile_signed(&[0], 0, pct), 0);
        assert_eq!(nearest_rank_abs_percentile_signed(&[-42], 1, pct), 42);
        assert_eq!(nearest_rank_abs_percentile_signed(&[42], 0, pct), 42);
    }

    // All non-negative slice (zero_idx == 0).
    let all_pos: Vec<i64> = (1..=100).map(|v| v * 10).collect();
    assert_eq!(nearest_rank_abs_percentile_signed(&all_pos, 0, 1), 10);
    assert_eq!(nearest_rank_abs_percentile_signed(&all_pos, 0, 50), 500);
    assert_eq!(nearest_rank_abs_percentile_signed(&all_pos, 0, 95), 950);
    assert_eq!(nearest_rank_abs_percentile_signed(&all_pos, 0, 99), 990);

    // All negative slice (zero_idx == len).
    let all_neg: Vec<i64> = (-100..=-1).map(|v| v * 10).collect();
    assert_eq!(
        nearest_rank_abs_percentile_signed(&all_neg, all_neg.len(), 1),
        10
    );
    assert_eq!(
        nearest_rank_abs_percentile_signed(&all_neg, all_neg.len(), 50),
        500
    );
    assert_eq!(
        nearest_rank_abs_percentile_signed(&all_neg, all_neg.len(), 95),
        950
    );
    assert_eq!(
        nearest_rank_abs_percentile_signed(&all_neg, all_neg.len(), 99),
        990
    );

    // Mixed-sign slices with magnitude ties.
    assert_eq!(nearest_rank_abs_percentile_signed(&[-100, 100], 1, 50), 100);
    assert_eq!(nearest_rank_abs_percentile_signed(&[-100, 100], 1, 99), 100);
    assert_eq!(
        nearest_rank_abs_percentile_signed(&[-100, -10, 100], 2, 1),
        10
    );
    assert_eq!(
        nearest_rank_abs_percentile_signed(&[-100, -10, 100], 2, 50),
        100
    );
    assert_eq!(
        nearest_rank_abs_percentile_signed(&[-100, -10, 100], 2, 95),
        100
    );
    assert_eq!(
        nearest_rank_abs_percentile_signed(&[-100, -10, 100], 2, 99),
        100
    );

    // 100-element mixed distribution compared against naive sorted absolute magnitudes.
    let mut mixed: Vec<i64> = (1..=100)
        .map(|i| if i % 3 == 0 { -(i * 17) } else { i * 13 })
        .collect();
    mixed.sort_unstable();
    let zero_idx = mixed.partition_point(|&v| v < 0);
    let mut naive_abs: Vec<u64> = mixed.iter().map(|v| v.unsigned_abs()).collect();
    naive_abs.sort_unstable();
    for pct in [1, 50, 95, 99] {
        assert_eq!(
            nearest_rank_abs_percentile_signed(&mixed, zero_idx, pct),
            nearest_rank_percentile(&naive_abs, pct)
        );
    }
}

#[test]
fn test_parse_log_anomaly_ordering_and_positive_jitter_summary_compat() {
    let log_str = "\
2026/09/06 00:01:24.000000 [INFO ] server - <CONFIG> [server_config]
2026/09/06 00:01:24.000000 [INFO ] server - <CONFIG> port = 7878
2026/09/06 00:01:24.000000 [INFO ] server - <CONFIG> cycle_time_ms = 50
2026/09/06 00:01:24.000000 [INFO ] server - <CONFIG> stats_interval_cycle = 0
2026/09/06 00:01:24.000000 [INFO ] server - <CONFIG> [[client_configs]]
2026/09/06 00:01:24.000000 [INFO ] server - <CONFIG> client_id = 10
2026/09/06 00:01:24.000000 [INFO ] server - <CONFIG> start_cycle = 0
2026/09/06 00:01:24.000000 [INFO ] server - <CONFIG> interval = 1
2026/09/06 00:01:24.000000 [INFO ] server - <CONFIG> offset = 0
2026/09/06 00:01:24.000000 [INFO ] server - <CONFIG> depends = []
2026/09/06 00:01:24.000000 [INFO ] server - <CONFIG> expected_duration_ms = 10
2026/09/06 00:01:25.000000 [DEBUG] process_running - <STAT> CYC:000000000000 START
2026/09/06 00:01:25.001000 [DEBUG] process_running - <STAT> CYC:000000000000 CID:010 Ready -> Running
2026/09/06 00:01:25.050200 [DEBUG] process_running - <STAT> CYC:000000000001 START
2026/09/06 00:01:25.050200 [DEBUG] process_running - <STAT> CYC:000000000001 CID:010 Running -> Overrun
2026/09/06 00:01:25.055000 [DEBUG] process_running - <STAT> CYC:000000000001 CID:010 Overrun -> Late
2026/09/06 00:01:25.100700 [DEBUG] process_running - <STAT> CYC:000000000002 START
2026/09/06 00:01:25.101000 [DEBUG] process_running - <STAT> CYC:000000000002 CID:010 Ready -> Skip
";

    let store = parse_log(log_str.as_bytes()).expect("Failed to parse synthetic log");
    let stats = &store.summary.stats;

    // Anomaly events are collected during parsing and sorted chronologically.
    assert_eq!(store.anomaly_events.len(), 3);
    assert_eq!(
        store.anomaly_events[0].event_type,
        InstantEventType::Overrun
    );
    assert_eq!(store.anomaly_events[1].event_type, InstantEventType::Late);
    assert_eq!(store.anomaly_events[2].event_type, InstantEventType::Skip);
    for w in store.anomaly_events.windows(2) {
        assert!(
            (w[0].time_us, w[0].log_line_no, w[0].instance_id)
                <= (w[1].time_us, w[1].log_line_no, w[1].instance_id)
        );
    }

    // Legacy summary fields stay consistent with stats while clamping min jitter at 0 for Cycle 0 compatibility.
    assert_eq!(
        store.summary.max_concurrency,
        stats.concurrency.peak_concurrency
    );
    let jitter = stats.jitter.as_ref().unwrap();
    assert_eq!(jitter.min_us, 200);
    assert_eq!(jitter.max_us, 500);
    assert_eq!(store.summary.min_start_jitter_us, 0);
    assert_eq!(store.summary.max_start_jitter_us, 500);
}
