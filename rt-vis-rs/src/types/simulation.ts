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
  cycle: number;
  cycle_offset_ms: number;
  start_ms: number;
  duration_ms: number;
  depends_instance_ids: number[];
  status: ExecutionStatus;
}

export interface PlannedMetricPoint {
  time_ms: number;
  running_count: number;
}

export interface SimulationResult {
  executions: PlannedExecution[];
  metrics: PlannedMetricPoint[];
  config_errors: Record<number, string[]>;
}
