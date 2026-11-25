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
/**
 * Ritsu Simulation Types
 * Based on rt-vis-rs/src-tauri/src/simulator.rs structure.
 */

export type ExecutionStatus = 'normal' | 'overrun' | 'skip';

export interface PlannedExecution {
  instance_id: number;
  cid: number;
  anchor_cycle: number;
  anchor_offset_us: number;
  start_us: number;
  duration_us: number;
  depends_instance_ids: number[];
  status: ExecutionStatus;
}

export interface PlannedMetricPoint {
  time_us: number;
  running_count: number;
}

/**
 * Time-weighted duration and ratio spent at a discrete concurrency level.
 */
export interface ConcurrencyLevelStat {
  level: number;
  duration_us: number;
  ratio: number;
}

/**
 * System-wide concurrency and utilization statistics over a time window.
 */
export interface ConcurrencyStats {
  total_duration_us: number;
  active_duration_us: number;
  active_ratio: number;
  active_avg_concurrency: number;
  peak_concurrency: number;
  peak_time_us: number | null;
  distribution: ConcurrencyLevelStat[];
}

export interface SimulationResult {
  executions: PlannedExecution[];
  metrics: PlannedMetricPoint[];
  concurrency_stats: ConcurrencyStats | null;
  config_errors: Record<number, string[]>;
}
