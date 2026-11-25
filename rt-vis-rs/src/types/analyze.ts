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
import type { SchedulerConfig } from './config';
import type { ConcurrencyLevelStat, ConcurrencyStats, ExecutionStatus } from './simulation';

export type { ConcurrencyLevelStat, ConcurrencyStats };

/**
 * Event types for point-in-time execution events.
 */
export type InstantEventType = 'ready' | 'overrun' | 'skip' | 'late' | 'error' | 'exit' | 'retransmit';

/**
 * Anomaly instant event types displayed in the anomaly timeline and filter controls.
 */
export type AnomalyEventType = 'overrun' | 'skip' | 'late' | 'retransmit' | 'error';

/**
 * Frozen canonical list of anomaly event types in display order.
 */
export const ANOMALY_EVENT_TYPES: readonly AnomalyEventType[] = Object.freeze([
  'overrun',
  'skip',
  'late',
  'retransmit',
  'error'
]);

/**
 * Frozen display labels for anomaly event types.
 */
export const ANOMALY_EVENT_LABELS: Readonly<Record<AnomalyEventType, string>> = Object.freeze({
  overrun: 'Overrun',
  skip: 'Skip',
  late: 'Late',
  retransmit: 'Retransmit',
  error: 'Error'
});

/**
 * Point-in-time event recorded during execution.
 */
export interface ActualInstantEvent {
  log_line_no: number;
  instance_id: number;
  cycle: number;
  time_us: number;
  event_type: InstantEventType;
}

/**
 * Actual execution bar data for a client.
 */
export interface ActualExecution {
  log_line_no_start: number;
  log_line_no_end: number | null;
  instance_id: number;
  cycle: number;
  start_us: number;
  duration_us: number;
  status: ExecutionStatus;
}

/**
 * Actual metric data point for concurrency level over time.
 */
export interface ActualMetricPoint {
  time_us: number;
  running_count: number;
}

/**
 * Actual cycle record for server timer and jitter tracking.
 */
export interface ActualCycle {
  cycle: number;
  start_us: number;
  start_jitter_us: number;
}

/**
 * System-wide health and anomaly event summary across all clients.
 */
export interface SystemStats {
  total_cycles: number;
  clean_cycles: number;
  clean_cycle_ratio: number;
  overrun_count: number;
  skip_count: number;
  late_count: number;
  retransmit_count: number;
  error_count: number;
}

/**
 * Execution duration distribution metrics in microseconds.
 */
export interface DurationStats {
  sample_count: number;
  min_us: number;
  avg_us: number;
  p50_us: number;
  p95_us: number;
  p99_us: number;
  max_us: number;
  std_dev_us: number;
  wcet_cycle: number;
  wcet_time_us: number;
}

/**
 * Single half-open bin [start_us, end_us) inside a histogram.
 */
export interface HistogramBin {
  start_us: number;
  end_us: number;
  count: number;
}

/**
 * Histogram with uniform nice-number step size and explicit underflow/overflow buckets.
 */
export interface Histogram {
  step_us: number;
  origin_us: number;
  bins: HistogramBin[];
  underflow_count: number;
  overflow_count: number;
  total_count: number;
}

/**
 * Expected-duration budget coverage, exceedances, margins, and histogram.
 */
export interface ClientBudgetStats {
  expected_duration_us: number;
  coverage_ratio: number;
  exceed_count: number;
  avg_exceed_us: number;
  max_exceed_us: number;
  margin_p99_us: number;
  margin_wcet_us: number;
  duration_histogram: Histogram;
}

/**
 * Execution, event, duration, and budget statistics for a single client.
 */
export interface ClientStats {
  cid: number;
  normal_count: number;
  overrun_count: number;
  incomplete_count: number;
  skip_count: number;
  late_count: number;
  retransmit_count: number;
  error_count: number;
  exit_count: number;
  duration: DurationStats | null;
  budget: ClientBudgetStats | null;
}

/**
 * Single extreme jitter cycle entry for outlier inspection.
 */
export interface JitterOutlier {
  cycle: number;
  time_us: number;
  jitter_us: number;
}

/**
 * Cycle-to-cycle start jitter statistics (excluding Cycle 0).
 */
export interface JitterStats {
  sample_count: number;
  min_us: number;
  max_us: number;
  avg_us: number;
  min_cycle: number;
  min_time_us: number;
  max_cycle: number;
  max_time_us: number;
  abs_avg_us: number;
  abs_p95_us: number;
  abs_p99_us: number;
  abs_std_dev_us: number;
  histogram: Histogram;
  outliers: JitterOutlier[];
}

/**
 * Full statistics bundle aggregated across the loaded execution log.
 */
export interface LogStats {
  system: SystemStats;
  clients: ClientStats[];
  jitter: JitterStats | null;
  concurrency: ConcurrencyStats;
}

/**
 * Flattened anomaly event entry for cross-client chronological event inspection.
 */
export interface AnomalyEventItem {
  log_line_no: number;
  instance_id: number;
  cid: number;
  cycle: number;
  time_us: number;
  event_type: InstantEventType;
}

/**
 * Paginated response returned by get_anomaly_events IPC command.
 */
export interface AnomalyEventsPage {
  total_count: number;
  items: AnomalyEventItem[];
}

/**
 * Summary metadata returned by load_log IPC upon initial loading.
 */
export interface LogSummary {
  config: SchedulerConfig;
  total_duration_us: number;
  total_cycles: number;
  stats: LogStats;
  is_truncated: boolean;
}

/**
 * Range-restricted actual log data returned by get_log_range IPC for viewport rendering.
 */
export interface LogRangeData {
  actual_executions_by_cid: [number, ActualExecution[]][];
  actual_instant_events_by_cid: [number, ActualInstantEvent[]][];
  actual_metrics: ActualMetricPoint[];
  actual_cycles: ActualCycle[];
  is_truncated: boolean;
}
