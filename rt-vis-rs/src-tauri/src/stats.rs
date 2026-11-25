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
//! Statistics Computation Engine for Execution Logs & Plan Simulations.
//!

use rt_config::SchedulerConfig;

use crate::types::{
    ActualCycle, ActualExecution, ActualInstantEvent, ActualMetricPoint, AnomalyEventItem,
    ClientBudgetStats, ClientStats, ConcurrencyLevelStat, ConcurrencyStats, DurationStats,
    ExecutionStatus, Histogram, HistogramBin, InstantEventType, JitterOutlier, JitterStats,
    LogStats, PlannedMetricPoint, SystemStats,
};

use std::cmp::Ordering;
use std::collections::{BTreeSet, BinaryHeap, HashMap, HashSet};

#[cfg(test)]
#[path = "stats_test.rs"]
mod stats_test;

/* -------------------------------------------------------------------------- */

const HISTOGRAM_TARGET_BINS: usize = 20;
const DEFAULT_EMPTY_STEP_US: u64 = 1_000;
const MAX_JITTER_OUTLIERS: usize = 50;

/* -------------------------------------------------------------------------- */

/// Abstraction over concurrency step points so Analyze and Create modes share one aggregator.
pub trait MetricPoint {
    fn time_us(&self) -> u64;
    fn running_count(&self) -> u32;
}

impl MetricPoint for ActualMetricPoint {
    fn time_us(&self) -> u64 {
        self.time_us
    }

    fn running_count(&self) -> u32 {
        self.running_count
    }
}

impl MetricPoint for PlannedMetricPoint {
    fn time_us(&self) -> u64 {
        self.time_us
    }

    fn running_count(&self) -> u32 {
        self.running_count
    }
}

/// Min-heap wrapper placing the weakest outlier (smallest |jitter|, then largest cycle) on top.
#[derive(Debug, Clone)]
struct JitterOutlierMinEntry(JitterOutlier);

impl Ord for JitterOutlierMinEntry {
    // self < other means self ranks higher (larger |jitter|, earlier cycle)
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .0
            .jitter_us
            .unsigned_abs()
            .cmp(&self.0.jitter_us.unsigned_abs())
            .then_with(|| self.0.cycle.cmp(&other.0.cycle))
    }
}

impl PartialOrd for JitterOutlierMinEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for JitterOutlierMinEntry {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for JitterOutlierMinEntry {}

/// Execution completion and status counts for a single client.
struct ClientExecutionCounts {
    normal_count: u64,
    overrun_count: u64,
    incomplete_count: u64,
}

/// Per-client instant event counts excluding Ready and Overrun.
struct ClientEventCounts {
    skip_count: u64,
    late_count: u64,
    retransmit_count: u64,
    error_count: u64,
    exit_count: u64,
}

/* -------------------------------------------------------------------------- */
// Public Entry Points
/* -------------------------------------------------------------------------- */

/// Computes the complete statistics summary across the loaded execution log.
pub fn compute_log_stats(
    config: &SchedulerConfig,
    cycles: &[ActualCycle],
    executions_by_cid: &HashMap<u16, Vec<ActualExecution>>,
    instant_events_by_cid: &HashMap<u16, Vec<ActualInstantEvent>>,
    anomaly_events: &[AnomalyEventItem],
    metrics: &[ActualMetricPoint],
    total_duration_us: u64,
) -> LogStats {
    let system = compute_system_stats(anomaly_events, cycles);
    let clients = compute_client_stats(config, executions_by_cid, instant_events_by_cid);
    let jitter = compute_jitter_stats(cycles);
    let concurrency = compute_concurrency_stats(metrics, 0, total_duration_us);

    LogStats {
        system,
        clients,
        jitter,
        concurrency,
    }
}

/// Computes expected-duration budget alignment and histogram for a client in one linear pass.
///
/// Reuses immutable `DurationStats` percentiles and WCET so budget recomputation on
/// `expected_duration_ms` changes requires no sorting or secondary duration caches.
pub fn compute_client_budget(
    executions: &[ActualExecution],
    duration_stats: &DurationStats,
    expected_duration_ms: u32,
) -> ClientBudgetStats {
    compute_budget_from_durations(
        executions
            .iter()
            .filter(|exec| exec.log_line_no_end.is_some())
            .map(|exec| exec.duration_us),
        duration_stats,
        expected_duration_ms,
    )
}

/// Computes time-weighted concurrency statistics over `[window_start_us, window_end_us)`.
///
/// Shared by both Analyze mode (`[0, total_duration_us)`) and Create mode (steady-state
/// second period). Carries forward the active level immediately preceding `window_start_us`
/// and closes the final segment at `window_end_us`.
pub fn compute_concurrency_stats<M: MetricPoint>(
    points: &[M],
    window_start_us: u64,
    window_end_us: u64,
) -> ConcurrencyStats {
    if window_end_us <= window_start_us {
        return ConcurrencyStats::default();
    }

    // Carry forward the concurrency level active at or before window_start_us.
    let total_duration_us = window_end_us.saturating_sub(window_start_us);
    let start_idx = points.partition_point(|p| p.time_us() <= window_start_us);
    let mut current_level = if start_idx > 0 {
        points[start_idx - 1].running_count()
    } else {
        0
    };

    // Accumulate time-weighted durations per discrete concurrency level and track peak concurrency.
    let mut cursor = window_start_us;
    let mut durations_by_level: Vec<u64> = Vec::new();
    let mut peak_concurrency: u32 = 0;
    let mut peak_time_us: Option<u64> = None;
    let mut record_segment = |level: u32, start_us: u64, dt: u64| {
        if dt == 0 {
            return;
        }
        let idx = level as usize;
        if idx >= durations_by_level.len() {
            durations_by_level.resize(idx + 1, 0);
        }
        durations_by_level[idx] = durations_by_level[idx].saturating_add(dt);
        if level > peak_concurrency {
            peak_concurrency = level;
            peak_time_us = Some(start_us);
        }
    };
    for point in &points[start_idx..] {
        let pt_time = point.time_us();
        if pt_time >= window_end_us {
            break;
        }
        let dt = pt_time.saturating_sub(cursor);
        record_segment(current_level, cursor, dt);
        cursor = pt_time;
        current_level = point.running_count();
    }
    // Close the trailing segment spanning to window_end_us.
    let trailing_dt = window_end_us.saturating_sub(cursor);
    record_segment(current_level, cursor, trailing_dt);

    // Derive active utilization and contiguous 0..=peak_concurrency distribution bars.
    let mut active_duration_us: u64 = 0;
    let mut weighted_active_sum: f64 = 0.0;
    let mut distribution = Vec::with_capacity(durations_by_level.len());
    for (idx, &dur) in durations_by_level.iter().enumerate() {
        let level = idx as u32;
        if level >= 1 {
            active_duration_us = active_duration_us.saturating_add(dur);
            weighted_active_sum += (level as f64) * (dur as f64);
        }
        distribution.push(ConcurrencyLevelStat {
            level,
            duration_us: dur,
            ratio: dur as f64 / total_duration_us as f64,
        });
    }
    let active_ratio = active_duration_us as f64 / total_duration_us as f64;
    let active_avg_concurrency = if active_duration_us == 0 {
        0.0
    } else {
        weighted_active_sum / active_duration_us as f64
    };

    ConcurrencyStats {
        total_duration_us,
        active_duration_us,
        active_ratio,
        active_avg_concurrency,
        peak_concurrency,
        peak_time_us,
        distribution,
    }
}

/* -------------------------------------------------------------------------- */
// System & Client Aggregations
/* -------------------------------------------------------------------------- */

/// Aggregates system-wide anomaly counts and clean-cycle ratio (assumes `cycles` is sorted by `cycle`).
fn compute_system_stats(
    anomaly_events: &[AnomalyEventItem],
    cycles: &[ActualCycle],
) -> SystemStats {
    // Tally anomaly counts from the pre-filtered anomaly stream and collect dirty cycle IDs.
    // Note: System `overrun_count` reflects raw Overrun instant events (matching the Events tab),
    // whereas `ClientStats.overrun_count` counts completed bars and may differ by 1 if truncated mid-overrun.
    let mut overrun_count: u64 = 0;
    let mut skip_count: u64 = 0;
    let mut late_count: u64 = 0;
    let mut retransmit_count: u64 = 0;
    let mut error_count: u64 = 0;
    let mut dirty_cycles: HashSet<u64> = HashSet::new();
    for item in anomaly_events {
        match item.event_type {
            InstantEventType::Overrun => overrun_count += 1,
            InstantEventType::Skip => skip_count += 1,
            InstantEventType::Late => late_count += 1,
            InstantEventType::Retransmit => retransmit_count += 1,
            InstantEventType::Error => error_count += 1,
            InstantEventType::Ready | InstantEventType::Exit => {}
        }
        if item.event_type.is_critical_anomaly() {
            dirty_cycles.insert(item.cycle);
        }
    }

    // Count dirty cycles present in the parsed cycle list via binary search (O(D log N), O(1) when clean).
    let total_cycles = cycles.len() as u64;
    let dirty_in_cycles = if dirty_cycles.is_empty() || cycles.is_empty() {
        0
    } else {
        dirty_cycles
            .iter()
            .filter(|&&cyc| cycles.binary_search_by_key(&cyc, |c| c.cycle).is_ok())
            .count() as u64
    };
    let clean_cycles = total_cycles.saturating_sub(dirty_in_cycles);
    let clean_cycle_ratio = if total_cycles == 0 {
        0.0
    } else {
        clean_cycles as f64 / total_cycles as f64
    };

    SystemStats {
        total_cycles,
        clean_cycles,
        clean_cycle_ratio,
        overrun_count,
        skip_count,
        late_count,
        retransmit_count,
        error_count,
    }
}

/// Aggregates execution, event, duration, and budget statistics across all clients in ascending CID order.
fn compute_client_stats(
    config: &SchedulerConfig,
    executions_by_cid: &HashMap<u16, Vec<ActualExecution>>,
    instant_events_by_cid: &HashMap<u16, Vec<ActualInstantEvent>>,
) -> Vec<ClientStats> {
    // Index configured budgets and collect all known CIDs in deterministic ascending order.
    let expected_ms_by_cid: HashMap<u16, u32> = config
        .client_configs
        .iter()
        .map(|c| (c.client_id, c.expected_duration_ms))
        .collect();
    let mut all_cids: BTreeSet<u16> = BTreeSet::new();
    all_cids.extend(expected_ms_by_cid.keys().copied());
    all_cids.extend(executions_by_cid.keys().copied());
    all_cids.extend(instant_events_by_cid.keys().copied());

    // Compute statistics per client.
    let mut result = Vec::with_capacity(all_cids.len());
    for cid in all_cids {
        let execs = executions_by_cid
            .get(&cid)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let events = instant_events_by_cid
            .get(&cid)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let expected_duration_ms = expected_ms_by_cid.get(&cid).copied().unwrap_or(0);
        result.push(compute_single_client_stats(
            cid,
            expected_duration_ms,
            execs,
            events,
        ));
    }

    result
}

/// Computes execution counts, instant event counts, duration metrics, and budget alignment for one client.
fn compute_single_client_stats(
    cid: u16,
    expected_duration_ms: u32,
    execs: &[ActualExecution],
    events: &[ActualInstantEvent],
) -> ClientStats {
    // Summarize executions in a single pass and derive budget alignment from the sorted durations.
    let (exec_counts, duration_and_sorted) = summarize_client_executions(execs);
    let event_counts = count_client_events(events);
    let (duration, budget) = match duration_and_sorted {
        Some((dur_stats, sorted_durations)) => {
            let budget_stats = compute_budget_from_durations(
                sorted_durations.iter().copied(),
                &dur_stats,
                expected_duration_ms,
            );
            (Some(dur_stats), Some(budget_stats))
        }
        None => (None, None),
    };

    ClientStats {
        cid,
        normal_count: exec_counts.normal_count,
        overrun_count: exec_counts.overrun_count,
        incomplete_count: exec_counts.incomplete_count,
        skip_count: event_counts.skip_count,
        late_count: event_counts.late_count,
        retransmit_count: event_counts.retransmit_count,
        error_count: event_counts.error_count,
        exit_count: event_counts.exit_count,
        duration,
        budget,
    }
}

/// Summarizes execution status counts, duration statistics, and sorted completed durations in one pass.
fn summarize_client_executions(
    executions: &[ActualExecution],
) -> (ClientExecutionCounts, Option<(DurationStats, Vec<u64>)>) {
    // Tally completion counts, collect completed durations, and detect earliest WCET in one pass.
    let mut normal_count: u64 = 0;
    let mut overrun_count: u64 = 0;
    let mut incomplete_count: u64 = 0;
    let mut durations: Vec<u64> = Vec::with_capacity(executions.len());
    let mut sum_f64: f64 = 0.0;
    let mut wcet_duration_us: u64 = 0;
    let mut wcet_cycle: u64 = 0;
    let mut wcet_time_us: u64 = 0;
    let mut has_sample = false;
    for exec in executions {
        if exec.log_line_no_end.is_none() {
            incomplete_count += 1;
            continue;
        }
        match exec.status {
            ExecutionStatus::Normal => normal_count += 1,
            ExecutionStatus::Overrun => overrun_count += 1,
            ExecutionStatus::Skip => {}
        }
        durations.push(exec.duration_us);
        sum_f64 += exec.duration_us as f64;
        if !has_sample || exec.duration_us > wcet_duration_us {
            has_sample = true;
            wcet_duration_us = exec.duration_us;
            wcet_cycle = exec.cycle;
            wcet_time_us = exec.start_us;
        }
    }
    let counts = ClientExecutionCounts {
        normal_count,
        overrun_count,
        incomplete_count,
    };
    if durations.is_empty() {
        return (counts, None);
    }

    // Sort durations once to derive exact Nearest-Rank percentiles and population spread.
    durations.sort_unstable();
    let sample_count = durations.len() as u64;
    let avg_us = sum_f64 / sample_count as f64;
    let std_dev_us = population_std_dev(durations.iter().copied(), avg_us);
    let duration_stats = DurationStats {
        sample_count,
        min_us: durations[0],
        avg_us,
        p50_us: nearest_rank_percentile(&durations, 50),
        p95_us: nearest_rank_percentile(&durations, 95),
        p99_us: nearest_rank_percentile(&durations, 99),
        max_us: *durations.last().unwrap(),
        std_dev_us,
        wcet_cycle,
        wcet_time_us,
    };

    (counts, Some((duration_stats, durations)))
}

/// Tallies per-client instant event counts excluding Ready and Overrun.
fn count_client_events(events: &[ActualInstantEvent]) -> ClientEventCounts {
    let mut counts = ClientEventCounts {
        skip_count: 0,
        late_count: 0,
        retransmit_count: 0,
        error_count: 0,
        exit_count: 0,
    };
    for event in events {
        match event.event_type {
            InstantEventType::Skip => counts.skip_count += 1,
            InstantEventType::Late => counts.late_count += 1,
            InstantEventType::Retransmit => counts.retransmit_count += 1,
            InstantEventType::Error => counts.error_count += 1,
            InstantEventType::Exit => counts.exit_count += 1,
            InstantEventType::Ready | InstantEventType::Overrun => {}
        }
    }
    counts
}

/// Computes expected-duration budget alignment and histogram from completed duration values.
fn compute_budget_from_durations(
    durations: impl Iterator<Item = u64>,
    duration_stats: &DurationStats,
    expected_duration_ms: u32,
) -> ClientBudgetStats {
    // Initialize histogram bin layout upfront from P99 and candidate budget.
    let expected_duration_us = (expected_duration_ms as u64).saturating_mul(1_000);
    let target_upper = calc_histogram_target_upper(duration_stats.p99_us.max(expected_duration_us));
    let mut histogram = HistogramBuilder::new_non_negative(target_upper);

    // Aggregate coverage, exceedances, and histogram counts in a single pass.
    let mut completed_count: u64 = 0;
    let mut covered_count: u64 = 0;
    let mut exceed_count: u64 = 0;
    let mut exceed_sum_us: f64 = 0.0;
    let mut max_exceed_us: u64 = 0;
    for duration_us in durations {
        completed_count += 1;
        if duration_us <= expected_duration_us {
            covered_count += 1;
        } else {
            let exceed = duration_us.saturating_sub(expected_duration_us);
            exceed_count += 1;
            exceed_sum_us += exceed as f64;
            if exceed > max_exceed_us {
                max_exceed_us = exceed;
            }
        }
        histogram.record(duration_us as i64);
    }

    // Derive ratios, average exceedance, and headroom margins.
    let coverage_ratio = if completed_count == 0 {
        0.0
    } else {
        covered_count as f64 / completed_count as f64
    };
    let avg_exceed_us = if exceed_count == 0 {
        0.0
    } else {
        exceed_sum_us / exceed_count as f64
    };
    // Realized durations and expected_duration_us fit well within positive i64 bounds.
    let margin_p99_us = (expected_duration_us as i64).saturating_sub(duration_stats.p99_us as i64);
    let margin_wcet_us = (expected_duration_us as i64).saturating_sub(duration_stats.max_us as i64);

    ClientBudgetStats {
        expected_duration_us,
        coverage_ratio,
        exceed_count,
        avg_exceed_us,
        max_exceed_us,
        margin_p99_us,
        margin_wcet_us,
        duration_histogram: histogram.finish(),
    }
}

/* -------------------------------------------------------------------------- */
// Jitter Aggregation
/* -------------------------------------------------------------------------- */

/// Computes cycle-to-cycle start jitter statistics, top-50 outliers, and signed histogram.
fn compute_jitter_stats(cycles: &[ActualCycle]) -> Option<JitterStats> {
    // Cycle 0 has no preceding cycle, so its start_jitter_us is fixed at 0 and excluded from statistics.
    if cycles.len() <= 1 {
        return None;
    }

    // Single pass over cycles[1..]: collect signed values, track extrema, sums, and Top-50 outliers.
    let target = &cycles[1..];
    let sample_count = target.len() as u64;
    let mut min_us = target[0].start_jitter_us;
    let mut min_cycle = target[0].cycle;
    let mut min_time_us = target[0].start_us;
    let mut max_us = target[0].start_jitter_us;
    let mut max_cycle = target[0].cycle;
    let mut max_time_us = target[0].start_us;
    let mut signed_sum: f64 = 0.0;
    let mut abs_sum: f64 = 0.0;
    let mut signed_jitters: Vec<i64> = Vec::with_capacity(target.len());
    let mut top_outliers: BinaryHeap<JitterOutlierMinEntry> =
        BinaryHeap::with_capacity(MAX_JITTER_OUTLIERS);
    for cycle in target {
        let jitter = cycle.start_jitter_us;
        let abs_jitter = jitter.unsigned_abs();
        signed_sum += jitter as f64;
        abs_sum += abs_jitter as f64;
        signed_jitters.push(jitter);

        if jitter < min_us {
            min_us = jitter;
            min_cycle = cycle.cycle;
            min_time_us = cycle.start_us;
        }
        if jitter > max_us {
            max_us = jitter;
            max_cycle = cycle.cycle;
            max_time_us = cycle.start_us;
        }

        let candidate = JitterOutlierMinEntry(JitterOutlier {
            cycle: cycle.cycle,
            time_us: cycle.start_us,
            jitter_us: jitter,
        });
        if top_outliers.len() < MAX_JITTER_OUTLIERS {
            top_outliers.push(candidate);
        } else {
            if let Some(mut weakest) = top_outliers.peek_mut() {
                if candidate < *weakest {
                    *weakest = candidate;
                }
            }
        }
    }

    // Sort signed jitters once and locate the negative/non-negative boundary for zero-copy analytics.
    signed_jitters.sort_unstable();
    let zero_idx = signed_jitters.partition_point(|&v| v < 0);
    let avg_us = signed_sum / sample_count as f64;
    let abs_avg_us = abs_sum / sample_count as f64;
    let abs_std_dev_us =
        population_std_dev(signed_jitters.iter().map(|v| v.unsigned_abs()), abs_avg_us);
    let abs_p95_us = nearest_rank_abs_percentile_signed(&signed_jitters, zero_idx, 95);
    let abs_p99_us = nearest_rank_abs_percentile_signed(&signed_jitters, zero_idx, 99);

    // Extract Top-50 outliers ordered by descending |jitter| then ascending cycle, and build histogram.
    let outliers: Vec<JitterOutlier> = top_outliers
        .into_sorted_vec()
        .into_iter()
        .map(|entry| entry.0)
        .collect();
    let histogram = build_jitter_histogram(&signed_jitters, zero_idx);

    Some(JitterStats {
        sample_count,
        min_us,
        max_us,
        avg_us,
        min_cycle,
        min_time_us,
        max_cycle,
        max_time_us,
        abs_avg_us,
        abs_p95_us,
        abs_p99_us,
        abs_std_dev_us,
        histogram,
        outliers,
    })
}

/// Builds a dynamically asymmetric jitter histogram anchored at 0 us from sorted signed values.
fn build_jitter_histogram(sorted_jitters: &[i64], zero_idx: usize) -> Histogram {
    let neg = &sorted_jitters[..zero_idx];
    let pos = &sorted_jitters[zero_idx..];

    // Derive independent P99-based target spans on negative and positive sides.
    let neg_target = if neg.is_empty() {
        0
    } else {
        // `neg` is ascending (largest magnitude first), so index from the end for Nearest-Rank P99.
        let desc_idx = nearest_rank_index(neg.len(), 99);
        let p99_mag = neg[neg.len() - 1 - desc_idx].unsigned_abs();
        calc_histogram_target_upper(p99_mag)
    };
    let pos_target = if pos.is_empty() {
        0
    } else {
        let p99_mag = pos[nearest_rank_index(pos.len(), 99)] as u64;
        calc_histogram_target_upper(p99_mag)
    };

    // Populate histogram bins anchored at 0 us using the shared step width.
    let mut builder =
        HistogramBuilder::new_signed(neg_target, pos_target, !neg.is_empty(), !pos.is_empty());
    for &jitter_us in sorted_jitters {
        builder.record(jitter_us);
    }
    builder.finish()
}

/* -------------------------------------------------------------------------- */
// Histogram Builder
/* -------------------------------------------------------------------------- */

/// Accumulator that builds a uniform-step histogram with underflow and overflow buckets.
struct HistogramBuilder {
    histogram: Histogram,
    upper_bound_us: i64,
}

impl HistogramBuilder {
    /// Creates a builder for non-negative values over `[0, upper)`.
    fn new_non_negative(target_upper_us: u64) -> Self {
        Self::new_layout(0, target_upper_us, false, true)
    }

    /// Creates a builder for signed values anchored on a `0 us` bin boundary.
    fn new_signed(neg_target_us: u64, pos_target_us: u64, has_neg: bool, has_pos: bool) -> Self {
        Self::new_layout(neg_target_us, pos_target_us, has_neg, has_pos)
    }

    /// Initializes contiguous half-open `[start_us, end_us)` bins from target spans.
    fn new_layout(neg_target_us: u64, pos_target_us: u64, has_neg: bool, has_pos: bool) -> Self {
        let total_span = neg_target_us.saturating_add(pos_target_us);
        let (step_us, neg_bins, pos_bins) = if total_span == 0 {
            (DEFAULT_EMPTY_STEP_US, 0usize, 1usize)
        } else {
            let step = nice_step_us(total_span as f64 / HISTOGRAM_TARGET_BINS as f64);
            let n_bins = if has_neg {
                neg_target_us.div_ceil(step).max(1) as usize
            } else {
                0
            };
            let p_bins = if has_pos {
                pos_target_us.div_ceil(step).max(1) as usize
            } else {
                0
            };
            (step, n_bins, p_bins)
        };
        let origin_us = -((neg_bins as u64).saturating_mul(step_us) as i64);
        let upper_bound_us = (pos_bins as u64).saturating_mul(step_us) as i64;
        let num_bins = neg_bins + pos_bins;
        let mut bins = Vec::with_capacity(num_bins);
        for i in 0..num_bins {
            let start_us = origin_us.saturating_add((i as u64).saturating_mul(step_us) as i64);
            let end_us = start_us.saturating_add(step_us as i64);
            bins.push(HistogramBin {
                start_us,
                end_us,
                count: 0,
            });
        }

        Self {
            histogram: Histogram {
                step_us,
                origin_us,
                bins,
                underflow_count: 0,
                overflow_count: 0,
                total_count: 0,
            },
            upper_bound_us,
        }
    }

    /// Records a single sample into its matching bin or underflow/overflow bucket.
    fn record(&mut self, value_us: i64) {
        self.histogram.total_count += 1;
        if value_us < self.histogram.origin_us {
            self.histogram.underflow_count += 1;
        } else if value_us >= self.upper_bound_us {
            self.histogram.overflow_count += 1;
        } else {
            let offset_us = value_us.saturating_sub(self.histogram.origin_us) as u64;
            let idx = (offset_us / self.histogram.step_us) as usize;
            self.histogram.bins[idx].count += 1;
        }
    }

    /// Finalizes and returns the populated histogram.
    fn finish(self) -> Histogram {
        self.histogram
    }
}

/// Computes the padded histogram range target (`1.2 * base_us`, or `0` when `base_us == 0`).
fn calc_histogram_target_upper(base_us: u64) -> u64 {
    if base_us == 0 {
        return 0;
    }
    ((base_us as f64) * 1.2)
        .ceil()
        .max(base_us.saturating_add(1) as f64) as u64
}

/// Rounds a raw bin width up to a clean `1, 2, 5 * 10^n` step in microseconds (minimum 1 us).
fn nice_step_us(raw_step: f64) -> u64 {
    if !raw_step.is_finite() || raw_step <= 1.0 {
        return 1;
    }
    let exp = raw_step.log10().floor();
    let base = 10f64.powf(exp);
    let fraction = raw_step / base;
    let multiplier = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };
    (multiplier * base).round().max(1.0) as u64
}

/* -------------------------------------------------------------------------- */
// Math & Percentile Helpers
/* -------------------------------------------------------------------------- */

/// Computes the zero-based Nearest-Rank index (`ceil(p * N / 100) - 1`).
fn nearest_rank_index(len: usize, percentile_pct: usize) -> usize {
    debug_assert!(len > 0);
    let rank = len.saturating_mul(percentile_pct).saturating_add(99) / 100;
    rank.saturating_sub(1).min(len - 1)
}

/// Computes the Nearest-Rank percentile from a pre-sorted slice.
fn nearest_rank_percentile(sorted: &[u64], percentile_pct: usize) -> u64 {
    sorted[nearest_rank_index(sorted.len(), percentile_pct)]
}

/// Computes the Nearest-Rank percentile of `|v|` from an ascending-sorted signed slice split at `zero_idx`.
fn nearest_rank_abs_percentile_signed(
    sorted: &[i64],
    zero_idx: usize,
    percentile_pct: usize,
) -> u64 {
    let len = sorted.len();
    debug_assert!(len > 0);
    let idx = nearest_rank_index(len, percentile_pct);

    // Handle single-sign slices in O(1) without scanning.
    if zero_idx == 0 {
        return sorted[idx] as u64;
    }
    if zero_idx == len {
        return sorted[len - 1 - idx].unsigned_abs();
    }

    // Walk inward from both ends (largest magnitudes first) down to the target rank.
    let rank_from_max = (len - 1) - idx;
    let mut left = 0usize;
    let mut right = len - 1;
    for _ in 0..rank_from_max {
        if sorted[left].unsigned_abs() >= sorted[right].unsigned_abs() {
            left += 1;
        } else {
            right -= 1;
        }
    }

    sorted[left]
        .unsigned_abs()
        .max(sorted[right].unsigned_abs())
}

/// Computes population standard deviation (`div N`) via a two-pass sum of squared deviations.
fn population_std_dev(values: impl ExactSizeIterator<Item = u64>, mean: f64) -> f64 {
    let len = values.len();
    if len <= 1 {
        return 0.0;
    }
    let sum_sq: f64 = values
        .map(|v| {
            let diff = (v as f64) - mean;
            diff * diff
        })
        .sum();
    (sum_sq / len as f64).sqrt()
}
