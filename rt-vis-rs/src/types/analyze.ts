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
import type { ExecutionStatus } from './simulation';

/**
 * Event types for point-in-time execution events.
 */
export type InstantEventType = 'ready' | 'overrun' | 'skip' | 'late' | 'error' | 'exit' | 'retransmit';

/**
 * Point-in-time event recorded during execution.
 */
export interface ActualInstantEvent {
  log_line_no: number;
  instance_id: number;
  time_ms: number;
  event_type: InstantEventType;
}

/**
 * Actual execution bar data for a client process.
 */
export interface ActualExecution {
  log_line_no_start: number;
  log_line_no_end: number | null;
  instance_id: number;
  cycle: number;
  start_ms: number;
  duration_ms: number;
  status: ExecutionStatus;
}

/**
 * Actual metric data point for concurrency level over time.
 */
export interface ActualMetricPoint {
  time_ms: number;
  running_count: number;
}

/**
 * Actual cycle record for server timer and jitter tracking.
 */
export interface ActualCycle {
  cycle: number;
  start_ms: number;
  start_jitter_ms: number;
}

/**
 * Summary metadata returned by load_log IPC upon initial loading.
 */
export interface LogSummary {
  config: SchedulerConfig;
  max_concurrency: number;
  total_duration_ms: number;
  total_cycles: number;
  min_start_jitter_ms: number;
  max_start_jitter_ms: number;
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
