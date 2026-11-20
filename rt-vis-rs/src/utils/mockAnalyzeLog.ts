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
import { DEFAULT_SERVER_CONFIG } from '../types/config';
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
    ...DEFAULT_SERVER_CONFIG,
    stats_interval_cycle: 200
  },
  client_configs: [
    { client_id: 1, start_cycle: 0, interval: 2, offset: 0, depends: [], display_name: "Camera", expected_duration_ms: 10 },
    { client_id: 101, start_cycle: 0, interval: 2, offset: 0, depends: [1], display_name: "ObjectDetect", expected_duration_ms: 60 },
    { client_id: 102, start_cycle: 0, interval: 2, offset: 0, depends: [1], display_name: "LaneDetect", expected_duration_ms: 20 },
    { client_id: 501, start_cycle: 0, interval: 2, offset: 0, depends: [101, 102], display_name: "Control", expected_duration_ms: 20 },
    { client_id: 901, start_cycle: 0, interval: 2, offset: 1, depends: [], display_name: "Telemetry", expected_duration_ms: 25 },
  ]
};

// -----------------------------------------------------------------------------
// Simulation & Mock Generation Parameters
// -----------------------------------------------------------------------------

const MOCK_PARAMS = {
  total_cycles: 500,
  cycle_time_us: 50_000,

  // Cycle jitter configuration in microseconds (positive delay accumulation with occasional spikes)
  jitter: {
    base_min_us: 100,
    base_max_us: 1_500,
    spike_interval_cycles: 45,
    spike_min_us: 4_000,
    spike_max_us: 8_000,
  },

  // Internal scheduler dispatch latency configuration in microseconds
  dispatch_delay: {
    min_us: 20,
    max_us: 80,
  },

  // Post-execution ready response latency configuration in microseconds
  ready_delay: {
    min_us: 400,
    max_us: 800,
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
 * Returns a random integer microsecond value between min and max.
 */
function getRandomUs(min: number, max: number): number {
  return Math.round(min + Math.random() * (max - min));
}

/**
 * Computes realistic execution duration in microseconds based on configured WCET ratio profile.
 */
function computeDurationUs(expectedMs: number, isSpike = false): number {
  const expectedUs = expectedMs * 1_000;
  if (isSpike) {
    return Math.round(expectedUs * MOCK_PARAMS.spike_anomaly.ratio);
  }
  const r = Math.random();
  const ratio = r < 0.5
    ? MOCK_PARAMS.duration_ratio.min + (MOCK_PARAMS.duration_ratio.median - MOCK_PARAMS.duration_ratio.min) * (r * 2)
    : MOCK_PARAMS.duration_ratio.median + (MOCK_PARAMS.duration_ratio.max - MOCK_PARAMS.duration_ratio.median) * ((r - 0.5) * 2);
  return Math.max(1_000, Math.round(expectedUs * ratio));
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
  let currentCycleStartUs = 0;
  let minStartJitterUs = Infinity;
  let maxStartJitterUs = -Infinity;

  for (let cycle = 0; cycle < MOCK_PARAMS.total_cycles; cycle++) {
    if (cycle === 0) {
      actualCycles.push({
        cycle: 0,
        start_us: 0,
        start_jitter_us: 0,
      });
      minStartJitterUs = 0;
      maxStartJitterUs = 0;
    } else {
      const isSpike = (cycle % MOCK_PARAMS.jitter.spike_interval_cycles === 0);
      const jitterUs = isSpike
        ? getRandomUs(MOCK_PARAMS.jitter.spike_min_us, MOCK_PARAMS.jitter.spike_max_us)
        : getRandomUs(MOCK_PARAMS.jitter.base_min_us, MOCK_PARAMS.jitter.base_max_us);

      currentCycleStartUs += MOCK_PARAMS.cycle_time_us + jitterUs;

      if (jitterUs < minStartJitterUs) minStartJitterUs = jitterUs;
      if (jitterUs > maxStartJitterUs) maxStartJitterUs = jitterUs;

      actualCycles.push({
        cycle,
        start_us: currentCycleStartUs,
        start_jitter_us: jitterUs,
      });
    }
  }

  // Helper utilities for recording executions, metrics, and instant events
  let nextLogLineNo = 100;
  let nextExecutionId = 1;
  let nextInstantEventId = 1;
  const rawMetricEvents: { time_us: number; change: number }[] = [];

  function recordExecution(
    cid: number,
    cycle: number,
    startUs: number,
    durationUs: number,
    boundaryUs: number
  ): { endUs: number; status: 'normal' | 'overrun' } {
    const roundedStart = Math.round(startUs);
    const roundedDuration = Math.round(durationUs);
    const endUs = roundedStart + roundedDuration;
    const isOverrun = endUs > boundaryUs;
    const status = isOverrun ? 'overrun' : 'normal';
    const executionId = nextExecutionId++;

    actualExecutionsByCid.get(cid)?.push({
      log_line_no_start: nextLogLineNo++,
      log_line_no_end: nextLogLineNo++,
      instance_id: executionId,
      cycle,
      start_us: roundedStart,
      duration_us: roundedDuration,
      status,
    });

    // Concurrency metric step events
    rawMetricEvents.push({ time_us: roundedStart, change: 1 });
    rawMetricEvents.push({ time_us: endUs, change: -1 });

    // Ready event sent shortly after client completion
    const readyTimeUs = endUs + getRandomUs(MOCK_PARAMS.ready_delay.min_us, MOCK_PARAMS.ready_delay.max_us);
    actualInstantEventsByCid.get(cid)?.push({
      log_line_no: nextLogLineNo++,
      instance_id: nextInstantEventId++,
      time_us: readyTimeUs,
      event_type: 'ready',
    });

    if (isOverrun) {
      // Overrun event recorded when scheduler crosses cycle boundary
      actualInstantEventsByCid.get(cid)?.push({
        log_line_no: nextLogLineNo++,
        instance_id: nextInstantEventId++,
        time_us: Math.round(boundaryUs),
        event_type: 'overrun',
      });
      // Late event recorded when the overrun task finishes
      actualInstantEventsByCid.get(cid)?.push({
        log_line_no: nextLogLineNo++,
        instance_id: nextInstantEventId++,
        time_us: endUs,
        event_type: 'late',
      });
    }

    return { endUs, status };
  }

  function recordSkip(cid: number, skipTimeUs: number): void {
    const roundedTime = Math.round(skipTimeUs);

    actualInstantEventsByCid.get(cid)?.push({
      log_line_no: nextLogLineNo++,
      instance_id: nextInstantEventId++,
      time_us: roundedTime,
      event_type: 'skip',
    });

    // Ready event sent shortly after skip notification
    const readyTimeUs = roundedTime + getRandomUs(MOCK_PARAMS.ready_delay.min_us, MOCK_PARAMS.ready_delay.max_us);
    actualInstantEventsByCid.get(cid)?.push({
      log_line_no: nextLogLineNo++,
      instance_id: nextInstantEventId++,
      time_us: readyTimeUs,
      event_type: 'ready',
    });
  }

  // Simulate even cycle executions: Camera -> (ObjectDetect & LaneDetect) -> Control
  let prevEvenEndUs = 0;

  for (let cycle = 0; cycle < MOCK_PARAMS.total_cycles; cycle += 2) {
    const cycleStartUs = actualCycles[cycle].start_us;
    const boundaryUs = actualCycles[cycle + 2]?.start_us ?? (cycleStartUs + MOCK_PARAMS.cycle_time_us * 2);

    // Check if previous even cycle dragged past this cycle's start time
    if (prevEvenEndUs > cycleStartUs) {
      recordSkip(1, cycleStartUs);
      recordSkip(101, cycleStartUs);
      recordSkip(102, cycleStartUs);
      recordSkip(501, cycleStartUs);
      continue;
    }

    // Camera execution
    const camDelayUs = getRandomUs(MOCK_PARAMS.dispatch_delay.min_us, MOCK_PARAMS.dispatch_delay.max_us);
    const camStartUs = cycleStartUs + camDelayUs;
    const camDurationUs = computeDurationUs(10);
    const camExec = recordExecution(1, cycle, camStartUs, camDurationUs, boundaryUs);

    // ObjectDetect and LaneDetect parallel execution (physically starts at or after Camera completion)
    const isSpike = (cycle % MOCK_PARAMS.spike_anomaly.interval_cycles === 0 && cycle > 0);
    const objDurationUs = computeDurationUs(60, isSpike);
    const laneDurationUs = computeDurationUs(20);

    const objDelayUs = getRandomUs(MOCK_PARAMS.dispatch_delay.min_us, MOCK_PARAMS.dispatch_delay.max_us);
    const laneDelayUs = getRandomUs(MOCK_PARAMS.dispatch_delay.min_us, MOCK_PARAMS.dispatch_delay.max_us);
    const objStartUs = camExec.endUs + objDelayUs;
    const laneStartUs = camExec.endUs + laneDelayUs;

    const objExec = recordExecution(101, cycle, objStartUs, objDurationUs, boundaryUs);
    const laneExec = recordExecution(102, cycle, laneStartUs, laneDurationUs, boundaryUs);

    // Control execution or cascaded skip evaluation (physically starts at or after all dependencies finish)
    if (objExec.status === 'overrun') {
      // Preceding task overran past the boundary, so dependent Control is skipped (Cascaded Skip)
      recordSkip(501, boundaryUs);
      prevEvenEndUs = objExec.endUs;
    } else {
      // Preceding tasks completed before the boundary, so scheduler dispatches Control
      const ctrlDelayUs = getRandomUs(MOCK_PARAMS.dispatch_delay.min_us, MOCK_PARAMS.dispatch_delay.max_us);
      const precedingEndUs = Math.max(objExec.endUs, laneExec.endUs);
      const ctrlStartUs = precedingEndUs + ctrlDelayUs;
      const ctrlDurationUs = computeDurationUs(20);
      const ctrlExec = recordExecution(501, cycle, ctrlStartUs, ctrlDurationUs, boundaryUs);
      prevEvenEndUs = ctrlExec.endUs;
    }
  }

  // Simulate odd cycle executions: Telemetry
  let prevOddEndUs = 0;

  for (let cycle = 1; cycle < MOCK_PARAMS.total_cycles; cycle += 2) {
    const cycleStartUs = actualCycles[cycle].start_us;
    const boundaryUs = actualCycles[cycle + 2]?.start_us ?? (cycleStartUs + MOCK_PARAMS.cycle_time_us * 2);

    if (prevOddEndUs > cycleStartUs) {
      recordSkip(901, cycleStartUs);
      continue;
    }

    const telemDelayUs = getRandomUs(MOCK_PARAMS.dispatch_delay.min_us, MOCK_PARAMS.dispatch_delay.max_us);
    const telemStartUs = cycleStartUs + telemDelayUs;
    const telemDurationUs = computeDurationUs(25);
    const telemExec = recordExecution(901, cycle, telemStartUs, telemDurationUs, boundaryUs);
    prevOddEndUs = telemExec.endUs;
  }

  // Aggregate concurrency step events and compile summary metadata
  rawMetricEvents.sort((a, b) => a.time_us - b.time_us);
  const actualMetrics: ActualMetricPoint[] = [{ time_us: 0, running_count: 0 }];
  let currentRunning = 0;
  let maxConcurrency = 0;

  rawMetricEvents.forEach(evt => {
    currentRunning += evt.change;
    const running = Math.max(0, currentRunning);
    if (running > maxConcurrency) {
      maxConcurrency = running;
    }

    const last = actualMetrics[actualMetrics.length - 1];
    if (last && last.time_us === evt.time_us) {
      // Overwrite previous count at the exact same timestamp to prevent duplicate points
      last.running_count = running;
    } else {
      actualMetrics.push({
        time_us: evt.time_us,
        running_count: running
      });
    }
  });

  const lastCycle = actualCycles[actualCycles.length - 1];
  const totalDurationUs = Math.ceil(
    lastCycle ? lastCycle.start_us + MOCK_PARAMS.cycle_time_us : MOCK_PARAMS.total_cycles * MOCK_PARAMS.cycle_time_us
  );

  const summary: LogSummary = {
    config: mockConfig,
    max_concurrency: maxConcurrency,
    total_duration_us: totalDurationUs,
    total_cycles: MOCK_PARAMS.total_cycles,
    min_start_jitter_us: isFinite(minStartJitterUs) ? minStartJitterUs : 0,
    max_start_jitter_us: isFinite(maxStartJitterUs) ? maxStartJitterUs : 0,
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
 * Returns range-sliced actual data for the given [start_us, end_us] window.
 */
export async function mockGetLogRange(start_us: number, end_us: number): Promise<LogRangeData> {
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
      const execEnd = e.start_us + e.duration_us;
      return execEnd >= start_us && e.start_us <= end_us;
    });
    slicedExecutions.push([cid, matched]);
  });

  // Slice instant events per CID
  const slicedEvents: [number, ActualInstantEvent[]][] = [];
  actualInstantEventsByCid.forEach((events, cid) => {
    const matched = events.filter(e => e.time_us >= start_us && e.time_us <= end_us);
    slicedEvents.push([cid, matched]);
  });

  // Slice metrics
  const slicedMetrics = actualMetrics.filter(m => m.time_us >= start_us && m.time_us <= end_us);

  // Slice cycles with padding margin for rendering continuity
  const cycleMarginUs = (fullLogCache.summary.config.server_config.cycle_time_ms || 50) * 1_000 * 2;
  const slicedCycles = actualCycles.filter(
    c => c.start_us >= start_us - cycleMarginUs && c.start_us <= end_us + cycleMarginUs
  );

  return {
    actual_executions_by_cid: slicedExecutions,
    actual_instant_events_by_cid: slicedEvents,
    actual_metrics: slicedMetrics,
    actual_cycles: slicedCycles,
    is_truncated: false
  };
}
