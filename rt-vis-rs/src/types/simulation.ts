/**
 * Ritsu Simulation Types
 * Based on rt-vis-rs/src-tauri/src/simulator.rs structure.
 */

export interface PlannedExecution {
  instanceId: number;
  cid: number;
  cycle: number;
  cycleOffsetMs: number;
  startMs: number;
  durationMs: number;
  dependsInstanceIds: number[];
}

export interface PlannedMetricPoint {
  timeMs: number;
  runningCount: number;
}

export interface SimulationResult {
  executions: PlannedExecution[];
  metrics: PlannedMetricPoint[];
}
