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
import type { SchedulerConfig } from '../types/config';
import type {
  LogSummary,
  LogRangeData,
  ActualExecution,
  ActualInstantEvent,
  ActualMetricPoint,
  ActualCycle
} from '../types/analyze';

// -----------------------------------------------------------------------------
// Sample Baseline Scheduler Configuration
// -----------------------------------------------------------------------------

const mockConfig: SchedulerConfig = {
  server_config: {
    port: 7878,
    cycle_time_ms: 50,
    stats_interval_cycle: 200
  },
  client_configs: [
    { client_id: 1, display_name: "Camera", cycle: 2, cycle_offset: 0, depends: [], expected_duration_ms: 10 },
    { client_id: 101, display_name: "ObjectDetect", cycle: 2, cycle_offset: 0, depends: [1], expected_duration_ms: 60 },
    { client_id: 102, display_name: "LaneDetect", cycle: 2, cycle_offset: 0, depends: [1], expected_duration_ms: 20 },
    { client_id: 501, display_name: "Control", cycle: 2, cycle_offset: 0, depends: [101, 102], expected_duration_ms: 20 },
    { client_id: 901, display_name: "Telemetry", cycle: 2, cycle_offset: 1, depends: [], expected_duration_ms: 25 },
  ]
};

// -----------------------------------------------------------------------------
// Simulation & Mock Generation Parameters
// -----------------------------------------------------------------------------

const MOCK_PARAMS = {
  total_cycles: 500,
  cycle_time_ms: 50,

  // Cycle jitter configuration (positive delay accumulation with occasional spikes)
  jitter: {
    base_min: 0.1,
    base_max: 1.5,
    spike_interval_cycles: 45,
    spike_min: 4.0,
    spike_max: 8.0,
  },

  // Internal scheduler dispatch latency configuration
  dispatch_delay: {
    min: 0.02,
    max: 0.08,
  },

  // Post-execution ready response latency configuration
  ready_delay: {
    min: 0.4,
    max: 0.8,
  },

  // Execution duration ratio profile (relative to expected_duration_ms)
  duration_ratio: {
    min: 0.60,
    median: 0.80,
    max: 1.00,
  },

  // Execution duration spike anomaly configuration
  spike_anomaly: {
    interval_cycles: 50,
    ratio: 1.50,
  },
};

// -----------------------------------------------------------------------------
// Helper Functions for Random Value Calculation
// -----------------------------------------------------------------------------

/**
 * Returns a random floating point number between min and max.
 */
function getRandom(min: number, max: number): number {
  return min + Math.random() * (max - min);
}

/**
 * Computes realistic execution duration based on configured WCET ratio profile.
 */
function computeDuration(expectedMs: number, isSpike = false): number {
  if (isSpike) {
    return Math.round(expectedMs * MOCK_PARAMS.spike_anomaly.ratio * 10) / 10;
  }
  const r = Math.random();
  const ratio = r < 0.5
    ? MOCK_PARAMS.duration_ratio.min + (MOCK_PARAMS.duration_ratio.median - MOCK_PARAMS.duration_ratio.min) * (r * 2)
    : MOCK_PARAMS.duration_ratio.median + (MOCK_PARAMS.duration_ratio.max - MOCK_PARAMS.duration_ratio.median) * ((r - 0.5) * 2);
  return Math.max(1, Math.round(expectedMs * ratio * 10) / 10);
}

// -----------------------------------------------------------------------------
// In-Memory Full Log Generator
// -----------------------------------------------------------------------------

interface FullLogDataset {
  summary: LogSummary;
  actualExecutionsByCid: Map<number, ActualExecution[]>;
  actualInstantEventsByCid: Map<number, ActualInstantEvent[]>;
  actualMetrics: ActualMetricPoint[];
  actualCycles: ActualCycle[];
}

let fullLogCache: FullLogDataset | null = null;

/**
 * Generates realistic full log dataset covering 500 cycles with cumulative clock drift,
 * WCET execution time profiles, and scheduler-compliant overrun/skip cascading.
 */
function generateFullLogDataset(): FullLogDataset {
  const actualExecutionsByCid = new Map<number, ActualExecution[]>();
  const actualInstantEventsByCid = new Map<number, ActualInstantEvent[]>();

  mockConfig.client_configs.forEach(c => {
    actualExecutionsByCid.set(c.client_id, []);
    actualInstantEventsByCid.set(c.client_id, []);
  });

  // Generate cumulative cycles with realistic timer jitter
  const actualCycles: ActualCycle[] = [];
  let currentCycleStartMs = 0;
  let minStartJitterMs = Infinity;
  let maxStartJitterMs = -Infinity;

  for (let cycle = 0; cycle < MOCK_PARAMS.total_cycles; cycle++) {
    if (cycle === 0) {
      actualCycles.push({
        cycle: 0,
        start_ms: 0,
        start_jitter_ms: 0,
      });
      minStartJitterMs = 0;
      maxStartJitterMs = 0;
    } else {
      const isSpike = (cycle % MOCK_PARAMS.jitter.spike_interval_cycles === 0);
      const jitter = isSpike
        ? getRandom(MOCK_PARAMS.jitter.spike_min, MOCK_PARAMS.jitter.spike_max)
        : getRandom(MOCK_PARAMS.jitter.base_min, MOCK_PARAMS.jitter.base_max);
      const roundedJitter = Math.round(jitter * 100) / 100;

      currentCycleStartMs += MOCK_PARAMS.cycle_time_ms + roundedJitter;
      const roundedStartMs = Math.round(currentCycleStartMs * 100) / 100;

      if (roundedJitter < minStartJitterMs) minStartJitterMs = roundedJitter;
      if (roundedJitter > maxStartJitterMs) maxStartJitterMs = roundedJitter;

      actualCycles.push({
        cycle,
        start_ms: roundedStartMs,
        start_jitter_ms: roundedJitter,
      });
    }
  }

  // Helper utilities for recording executions, metrics, and instant events
  let nextExecutionId = 1;
  let nextInstantEventId = 1;
  const rawMetricEvents: { time_ms: number; change: number }[] = [];

  function recordExecution(
    cid: number,
    cycle: number,
    startMs: number,
    durationMs: number,
    boundaryMs: number
  ): { endMs: number; status: 'normal' | 'overrun' } {
    const roundedStart = Math.round(startMs * 10) / 10;
    const roundedDuration = Math.round(durationMs * 10) / 10;
    const endMs = Math.round((roundedStart + roundedDuration) * 10) / 10;
    const isOverrun = endMs > boundaryMs;
    const status = isOverrun ? 'overrun' : 'normal';
    const executionId = nextExecutionId++;

    actualExecutionsByCid.get(cid)?.push({
      instance_id: executionId,
      cycle,
      start_ms: roundedStart,
      duration_ms: roundedDuration,
      status,
    });

    // Concurrency metric step events
    rawMetricEvents.push({ time_ms: roundedStart, change: 1 });
    rawMetricEvents.push({ time_ms: endMs, change: -1 });

    // Ready event sent shortly after process completion
    const readyTime = endMs + getRandom(MOCK_PARAMS.ready_delay.min, MOCK_PARAMS.ready_delay.max);
    actualInstantEventsByCid.get(cid)?.push({
      instance_id: nextInstantEventId++,
      time_ms: Math.round(readyTime * 10) / 10,
      event_type: 'ready',
    });

    if (isOverrun) {
      // Overrun event recorded when scheduler crosses cycle boundary
      actualInstantEventsByCid.get(cid)?.push({
        instance_id: nextInstantEventId++,
        time_ms: Math.round(boundaryMs * 10) / 10,
        event_type: 'overrun',
      });
      // Late event recorded when the overrun task finishes
      actualInstantEventsByCid.get(cid)?.push({
        instance_id: nextInstantEventId++,
        time_ms: endMs,
        event_type: 'late',
      });
    }

    return { endMs, status };
  }

  function recordSkip(cid: number, skipTimeMs: number): void {
    const roundedTime = Math.round(skipTimeMs * 10) / 10;

    actualInstantEventsByCid.get(cid)?.push({
      instance_id: nextInstantEventId++,
      time_ms: roundedTime,
      event_type: 'skip',
    });

    // Ready event sent shortly after skip notification
    const readyTime = roundedTime + getRandom(MOCK_PARAMS.ready_delay.min, MOCK_PARAMS.ready_delay.max);
    actualInstantEventsByCid.get(cid)?.push({
      instance_id: nextInstantEventId++,
      time_ms: Math.round(readyTime * 10) / 10,
      event_type: 'ready',
    });
  }

  // Simulate even cycle executions: Camera -> (ObjectDetect & LaneDetect) -> Control
  let prevEvenEndMs = 0;

  for (let cycle = 0; cycle < MOCK_PARAMS.total_cycles; cycle += 2) {
    const cycleStartMs = actualCycles[cycle].start_ms;
    const boundaryMs = actualCycles[cycle + 2]?.start_ms ?? (cycleStartMs + MOCK_PARAMS.cycle_time_ms * 2);

    // Check if previous even cycle dragged past this cycle's start time
    if (prevEvenEndMs > cycleStartMs) {
      recordSkip(1, cycleStartMs);
      recordSkip(101, cycleStartMs);
      recordSkip(102, cycleStartMs);
      recordSkip(501, cycleStartMs);
      continue;
    }

    // Camera execution
    const camDelay = getRandom(MOCK_PARAMS.dispatch_delay.min, MOCK_PARAMS.dispatch_delay.max);
    const camStart = Math.round((cycleStartMs + camDelay) * 10) / 10;
    const camDuration = computeDuration(10);
    const camExec = recordExecution(1, cycle, camStart, camDuration, boundaryMs);

    // ObjectDetect and LaneDetect parallel execution (physically starts at or after Camera completion)
    const isSpike = (cycle % MOCK_PARAMS.spike_anomaly.interval_cycles === 0 && cycle > 0);
    const objDuration = computeDuration(60, isSpike);
    const laneDuration = computeDuration(20);

    const objDelay = getRandom(MOCK_PARAMS.dispatch_delay.min, MOCK_PARAMS.dispatch_delay.max);
    const laneDelay = getRandom(MOCK_PARAMS.dispatch_delay.min, MOCK_PARAMS.dispatch_delay.max);
    const objStart = Math.max(camExec.endMs, Math.round((camExec.endMs + objDelay) * 10) / 10);
    const laneStart = Math.max(camExec.endMs, Math.round((camExec.endMs + laneDelay) * 10) / 10);

    const objExec = recordExecution(101, cycle, objStart, objDuration, boundaryMs);
    const laneExec = recordExecution(102, cycle, laneStart, laneDuration, boundaryMs);

    // Control execution or cascaded skip evaluation (physically starts at or after all dependencies finish)
    if (objExec.status === 'overrun') {
      // Preceding task overran past the boundary, so dependent Control is skipped (Cascaded Skip)
      recordSkip(501, boundaryMs);
      prevEvenEndMs = objExec.endMs;
    } else {
      // Preceding tasks completed before the boundary, so scheduler dispatches Control
      const ctrlDelay = getRandom(MOCK_PARAMS.dispatch_delay.min, MOCK_PARAMS.dispatch_delay.max);
      const precedingEndMs = Math.max(objExec.endMs, laneExec.endMs);
      const ctrlStart = Math.max(precedingEndMs, Math.round((precedingEndMs + ctrlDelay) * 10) / 10);
      const ctrlDuration = computeDuration(20);
      const ctrlExec = recordExecution(501, cycle, ctrlStart, ctrlDuration, boundaryMs);
      prevEvenEndMs = ctrlExec.endMs;
    }
  }

  // Simulate odd cycle executions: Telemetry
  let prevOddEndMs = 0;

  for (let cycle = 1; cycle < MOCK_PARAMS.total_cycles; cycle += 2) {
    const cycleStartMs = actualCycles[cycle].start_ms;
    const boundaryMs = actualCycles[cycle + 2]?.start_ms ?? (cycleStartMs + MOCK_PARAMS.cycle_time_ms * 2);

    if (prevOddEndMs > cycleStartMs) {
      recordSkip(901, cycleStartMs);
      continue;
    }

    const telemDelay = getRandom(MOCK_PARAMS.dispatch_delay.min, MOCK_PARAMS.dispatch_delay.max);
    const telemStart = Math.round((cycleStartMs + telemDelay) * 10) / 10;
    const telemDuration = computeDuration(25);
    const telemExec = recordExecution(901, cycle, telemStart, telemDuration, boundaryMs);
    prevOddEndMs = telemExec.endMs;
  }

  // Aggregate concurrency step events and compile summary metadata
  rawMetricEvents.sort((a, b) => a.time_ms - b.time_ms);
  const actualMetrics: ActualMetricPoint[] = [{ time_ms: 0, running_count: 0 }];
  let currentRunning = 0;
  let maxConcurrency = 0;

  rawMetricEvents.forEach(evt => {
    currentRunning += evt.change;
    const running = Math.max(0, currentRunning);
    if (running > maxConcurrency) {
      maxConcurrency = running;
    }

    const last = actualMetrics[actualMetrics.length - 1];
    if (last && last.time_ms === evt.time_ms) {
      // Overwrite previous count at the exact same timestamp to prevent duplicate points
      last.running_count = running;
    } else {
      actualMetrics.push({
        time_ms: evt.time_ms,
        running_count: running
      });
    }
  });

  const lastCycle = actualCycles[actualCycles.length - 1];
  const totalDurationMs = Math.ceil(
    lastCycle ? lastCycle.start_ms + MOCK_PARAMS.cycle_time_ms : MOCK_PARAMS.total_cycles * MOCK_PARAMS.cycle_time_ms
  );

  const summary: LogSummary = {
    config: mockConfig,
    max_concurrency: maxConcurrency,
    total_duration_ms: totalDurationMs,
    total_cycles: MOCK_PARAMS.total_cycles,
    min_start_jitter_ms: isFinite(minStartJitterMs) ? minStartJitterMs : 0,
    max_start_jitter_ms: isFinite(maxStartJitterMs) ? maxStartJitterMs : 0,
    is_truncated: false
  };

  return {
    summary,
    actualExecutionsByCid,
    actualInstantEventsByCid,
    actualMetrics,
    actualCycles
  };
}

// -----------------------------------------------------------------------------
// Mock IPC API Methods
// -----------------------------------------------------------------------------

/**
 * Simulates load_log IPC command.
 * Initializes and caches the full log dataset in memory and returns LogSummary.
 */
export async function mockLoadLog(): Promise<LogSummary> {
  // Simulate asynchronous file read and parsing latency
  await new Promise(resolve => setTimeout(resolve, 50));
  fullLogCache = generateFullLogDataset();
  return fullLogCache.summary;
}

/**
 * Simulates get_log_range IPC command.
 * Returns range-sliced actual data for the given [start_ms, end_ms] window.
 */
export async function mockGetLogRange(start_ms: number, end_ms: number): Promise<LogRangeData> {
  if (!fullLogCache) {
    fullLogCache = generateFullLogDataset();
  }

  // Simulate non-blocking IPC slice latency
  await new Promise(resolve => setTimeout(resolve, 10));

  const { actualExecutionsByCid, actualInstantEventsByCid, actualMetrics, actualCycles } = fullLogCache;

  // Slice executions per CID
  const slicedExecutions: [number, ActualExecution[]][] = [];
  actualExecutionsByCid.forEach((execs, cid) => {
    const matched = execs.filter(e => {
      const execEnd = e.start_ms + e.duration_ms;
      return execEnd >= start_ms && e.start_ms <= end_ms;
    });
    slicedExecutions.push([cid, matched]);
  });

  // Slice instant events per CID
  const slicedEvents: [number, ActualInstantEvent[]][] = [];
  actualInstantEventsByCid.forEach((events, cid) => {
    const matched = events.filter(e => e.time_ms >= start_ms && e.time_ms <= end_ms);
    slicedEvents.push([cid, matched]);
  });

  // Slice metrics
  const slicedMetrics = actualMetrics.filter(m => m.time_ms >= start_ms && m.time_ms <= end_ms);

  // Slice cycles with padding margin for rendering continuity
  const cycleMarginMs = (fullLogCache.summary.config.server_config.cycle_time_ms || 50) * 2;
  const slicedCycles = actualCycles.filter(
    c => c.start_ms >= start_ms - cycleMarginMs && c.start_ms <= end_ms + cycleMarginMs
  );

  return {
    actual_executions_by_cid: slicedExecutions,
    actual_instant_events_by_cid: slicedEvents,
    actual_metrics: slicedMetrics,
    actual_cycles: slicedCycles,
    is_truncated: false
  };
}

