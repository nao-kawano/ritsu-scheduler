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
