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
import {
  ANOMALY_EVENT_TYPES,
  type ActualCycle,
  type ActualExecution,
  type ActualInstantEvent,
  type ActualMetricPoint,
  type AnomalyEventItem,
  type AnomalyEventsPage,
  type ClientBudgetStats,
  type ClientStats,
  type ConcurrencyLevelStat,
  type ConcurrencyStats,
  type DurationStats,
  type Histogram,
  type HistogramBin,
  type InstantEventType,
  type JitterOutlier,
  type JitterStats,
  type LogRangeData,
  type LogStats,
  type LogSummary,
  type SystemStats
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
// Statistics Computation Helpers (Mock Parity with src-tauri/src/stats.rs)
// -----------------------------------------------------------------------------

const HISTOGRAM_TARGET_BINS = 20;
const DEFAULT_EMPTY_STEP_US = 1_000;
const MAX_JITTER_OUTLIERS = 50;
const MAX_ANOMALY_EVENTS_PAGE_LIMIT = 1_000;

const ANOMALY_EVENT_SET = new Set<string>(ANOMALY_EVENT_TYPES);
const CRITICAL_ANOMALY_SET = new Set<string>(['overrun', 'skip', 'late', 'error']);

function nearestRankIndex(len: number, percentilePct: number): number {
  const rank = Math.floor((len * percentilePct + 99) / 100);
  return Math.min(len - 1, Math.max(0, rank - 1));
}

function nearestRankPercentile(sorted: number[], percentilePct: number): number {
  return sorted[nearestRankIndex(sorted.length, percentilePct)];
}

function populationStdDev(values: number[], mean: number): number {
  if (values.length <= 1) return 0;
  let sumSq = 0;
  for (const v of values) {
    const diff = v - mean;
    sumSq += diff * diff;
  }
  return Math.sqrt(sumSq / values.length);
}

function niceStepUs(rawStep: number): number {
  if (!Number.isFinite(rawStep) || rawStep <= 1) return 1;
  const exp = Math.floor(Math.log10(rawStep));
  const base = Math.pow(10, exp);
  const fraction = rawStep / base;
  let multiplier = 10;
  if (fraction <= 1) {
    multiplier = 1;
  } else if (fraction <= 2) {
    multiplier = 2;
  } else if (fraction <= 5) {
    multiplier = 5;
  }
  return Math.max(1, Math.round(multiplier * base));
}

function calcHistogramTargetUpper(baseUs: number): number {
  if (baseUs <= 0) return 0;
  return Math.max(baseUs + 1, Math.ceil(baseUs * 1.2));
}

function createHistogram(
  negTargetUs: number,
  posTargetUs: number,
  hasNeg: boolean,
  hasPos: boolean,
  valuesUs: number[]
): Histogram {
  const totalSpan = negTargetUs + posTargetUs;
  let stepUs = DEFAULT_EMPTY_STEP_US;
  let negBins = 0;
  let posBins = 1;

  if (totalSpan > 0) {
    stepUs = niceStepUs(totalSpan / HISTOGRAM_TARGET_BINS);
    negBins = hasNeg ? Math.max(1, Math.ceil(negTargetUs / stepUs)) : 0;
    posBins = hasPos ? Math.max(1, Math.ceil(posTargetUs / stepUs)) : 0;
  }

  const originUs = -(negBins * stepUs);
  const upperBoundUs = posBins * stepUs;
  const numBins = negBins + posBins;
  const bins: HistogramBin[] = [];

  for (let i = 0; i < numBins; i++) {
    const startUs = originUs + i * stepUs;
    bins.push({
      start_us: startUs,
      end_us: startUs + stepUs,
      count: 0
    });
  }

  let underflowCount = 0;
  let overflowCount = 0;
  for (const v of valuesUs) {
    if (v < originUs) {
      underflowCount += 1;
    } else if (v >= upperBoundUs) {
      overflowCount += 1;
    } else {
      const idx = Math.floor((v - originUs) / stepUs);
      bins[idx].count += 1;
    }
  }

  return {
    step_us: stepUs,
    origin_us: originUs,
    bins,
    underflow_count: underflowCount,
    overflow_count: overflowCount,
    total_count: valuesUs.length
  };
}

function computeMockClientBudget(
  executions: ActualExecution[],
  durationStats: DurationStats,
  expectedDurationMs: number
): ClientBudgetStats {
  const expectedDurationUs = Math.max(0, Math.round(expectedDurationMs)) * 1_000;
  const targetUpper = calcHistogramTargetUpper(Math.max(durationStats.p99_us, expectedDurationUs));

  const completedDurations: number[] = [];
  let coveredCount = 0;
  let exceedCount = 0;
  let exceedSumUs = 0;
  let maxExceedUs = 0;

  for (const exec of executions) {
    if (exec.log_line_no_end === null) continue;
    completedDurations.push(exec.duration_us);
    if (exec.duration_us <= expectedDurationUs) {
      coveredCount += 1;
    } else {
      const exceed = exec.duration_us - expectedDurationUs;
      exceedCount += 1;
      exceedSumUs += exceed;
      if (exceed > maxExceedUs) maxExceedUs = exceed;
    }
  }

  const completedCount = completedDurations.length;
  const coverageRatio = completedCount === 0 ? 0 : coveredCount / completedCount;
  const avgExceedUs = exceedCount === 0 ? 0 : exceedSumUs / exceedCount;
  const durationHistogram = createHistogram(0, targetUpper, false, true, completedDurations);

  return {
    expected_duration_us: expectedDurationUs,
    coverage_ratio: coverageRatio,
    exceed_count: exceedCount,
    avg_exceed_us: avgExceedUs,
    max_exceed_us: maxExceedUs,
    margin_p99_us: expectedDurationUs - durationStats.p99_us,
    margin_wcet_us: expectedDurationUs - durationStats.max_us,
    duration_histogram: durationHistogram
  };
}

function computeMockSystemStats(
  anomalyEvents: AnomalyEventItem[],
  cycles: ActualCycle[]
): SystemStats {
  let overrunCount = 0;
  let skipCount = 0;
  let lateCount = 0;
  let retransmitCount = 0;
  let errorCount = 0;
  const dirtyCycles = new Set<number>();

  for (const item of anomalyEvents) {
    switch (item.event_type) {
      case 'overrun':
        overrunCount += 1;
        break;
      case 'skip':
        skipCount += 1;
        break;
      case 'late':
        lateCount += 1;
        break;
      case 'retransmit':
        retransmitCount += 1;
        break;
      case 'error':
        errorCount += 1;
        break;
    }
    if (CRITICAL_ANOMALY_SET.has(item.event_type)) {
      dirtyCycles.add(item.cycle);
    }
  }

  const cycleNumbers = new Set(cycles.map(c => c.cycle));
  let dirtyInCycles = 0;
  dirtyCycles.forEach(cyc => {
    if (cycleNumbers.has(cyc)) dirtyInCycles += 1;
  });

  const totalCycles = cycles.length;
  const cleanCycles = Math.max(0, totalCycles - dirtyInCycles);
  const cleanCycleRatio = totalCycles === 0 ? 0 : cleanCycles / totalCycles;

  return {
    total_cycles: totalCycles,
    clean_cycles: cleanCycles,
    clean_cycle_ratio: cleanCycleRatio,
    overrun_count: overrunCount,
    skip_count: skipCount,
    late_count: lateCount,
    retransmit_count: retransmitCount,
    error_count: errorCount
  };
}

function computeMockClientStats(
  config: SchedulerConfig,
  executionsByCid: Map<number, ActualExecution[]>,
  instantEventsByCid: Map<number, ActualInstantEvent[]>
): ClientStats[] {
  const expectedMsByCid = new Map<number, number>();
  for (const c of config.client_configs) {
    expectedMsByCid.set(c.client_id, c.expected_duration_ms);
  }

  const allCids = new Set<number>([
    ...expectedMsByCid.keys(),
    ...executionsByCid.keys(),
    ...instantEventsByCid.keys()
  ]);
  const sortedCids = [...allCids].sort((a, b) => a - b);

  return sortedCids.map(cid => {
    const execs = executionsByCid.get(cid) ?? [];
    const events = instantEventsByCid.get(cid) ?? [];
    const expectedDurationMs = expectedMsByCid.get(cid) ?? 0;

    let normalCount = 0;
    let overrunCount = 0;
    let incompleteCount = 0;
    const durations: number[] = [];
    let sumUs = 0;
    let wcetDurationUs = 0;
    let wcetCycle = 0;
    let wcetTimeUs = 0;

    for (const exec of execs) {
      if (exec.log_line_no_end === null) {
        incompleteCount += 1;
        continue;
      }
      if (exec.status === 'normal') normalCount += 1;
      else if (exec.status === 'overrun') overrunCount += 1;

      durations.push(exec.duration_us);
      sumUs += exec.duration_us;
      if (durations.length === 1 || exec.duration_us > wcetDurationUs) {
        wcetDurationUs = exec.duration_us;
        wcetCycle = exec.cycle;
        wcetTimeUs = exec.start_us;
      }
    }

    let skipCount = 0;
    let lateCount = 0;
    let retransmitCount = 0;
    let errorCount = 0;
    let exitCount = 0;
    for (const evt of events) {
      switch (evt.event_type) {
        case 'skip':
          skipCount += 1;
          break;
        case 'late':
          lateCount += 1;
          break;
        case 'retransmit':
          retransmitCount += 1;
          break;
        case 'error':
          errorCount += 1;
          break;
        case 'exit':
          exitCount += 1;
          break;
      }
    }

    let duration: DurationStats | null = null;
    let budget: ClientBudgetStats | null = null;

    if (durations.length > 0) {
      durations.sort((a, b) => a - b);
      const avgUs = sumUs / durations.length;
      duration = {
        sample_count: durations.length,
        min_us: durations[0],
        avg_us: avgUs,
        p50_us: nearestRankPercentile(durations, 50),
        p95_us: nearestRankPercentile(durations, 95),
        p99_us: nearestRankPercentile(durations, 99),
        max_us: durations[durations.length - 1],
        std_dev_us: populationStdDev(durations, avgUs),
        wcet_cycle: wcetCycle,
        wcet_time_us: wcetTimeUs
      };
      budget = computeMockClientBudget(execs, duration, expectedDurationMs);
    }

    return {
      cid,
      normal_count: normalCount,
      overrun_count: overrunCount,
      incomplete_count: incompleteCount,
      skip_count: skipCount,
      late_count: lateCount,
      retransmit_count: retransmitCount,
      error_count: errorCount,
      exit_count: exitCount,
      duration,
      budget
    };
  });
}

function computeMockJitterStats(cycles: ActualCycle[]): JitterStats | null {
  if (cycles.length <= 1) return null;

  const target = cycles.slice(1);
  let minUs = target[0].start_jitter_us;
  let minCycle = target[0].cycle;
  let minTimeUs = target[0].start_us;
  let maxUs = target[0].start_jitter_us;
  let maxCycle = target[0].cycle;
  let maxTimeUs = target[0].start_us;
  let signedSum = 0;
  let absSum = 0;

  const signedJitters: number[] = [];
  const absJitters: number[] = [];
  const negJitters: number[] = [];
  const posJitters: number[] = [];
  const outliers: JitterOutlier[] = [];

  for (const c of target) {
    const jitter = c.start_jitter_us;
    const absJitter = Math.abs(jitter);
    signedSum += jitter;
    absSum += absJitter;
    signedJitters.push(jitter);
    absJitters.push(absJitter);
    if (jitter < 0) negJitters.push(absJitter);
    else posJitters.push(jitter);

    if (jitter < minUs) {
      minUs = jitter;
      minCycle = c.cycle;
      minTimeUs = c.start_us;
    }
    if (jitter > maxUs) {
      maxUs = jitter;
      maxCycle = c.cycle;
      maxTimeUs = c.start_us;
    }

    outliers.push({
      cycle: c.cycle,
      time_us: c.start_us,
      jitter_us: jitter
    });
  }

  absJitters.sort((a, b) => a - b);
  negJitters.sort((a, b) => a - b);
  posJitters.sort((a, b) => a - b);

  outliers.sort((a, b) => {
    const diff = Math.abs(b.jitter_us) - Math.abs(a.jitter_us);
    return diff !== 0 ? diff : a.cycle - b.cycle;
  });

  const sampleCount = target.length;
  const avgUs = signedSum / sampleCount;
  const absAvgUs = absSum / sampleCount;
  const negTarget = negJitters.length === 0 ? 0 : calcHistogramTargetUpper(nearestRankPercentile(negJitters, 99));
  const posTarget = posJitters.length === 0 ? 0 : calcHistogramTargetUpper(nearestRankPercentile(posJitters, 99));

  return {
    sample_count: sampleCount,
    min_us: minUs,
    max_us: maxUs,
    avg_us: avgUs,
    min_cycle: minCycle,
    min_time_us: minTimeUs,
    max_cycle: maxCycle,
    max_time_us: maxTimeUs,
    abs_avg_us: absAvgUs,
    abs_p95_us: nearestRankPercentile(absJitters, 95),
    abs_p99_us: nearestRankPercentile(absJitters, 99),
    abs_std_dev_us: populationStdDev(absJitters, absAvgUs),
    histogram: createHistogram(negTarget, posTarget, negJitters.length > 0, posJitters.length > 0, signedJitters),
    outliers: outliers.slice(0, MAX_JITTER_OUTLIERS)
  };
}

function computeMockConcurrencyStats(
  metrics: ActualMetricPoint[],
  totalDurationUs: number
): ConcurrencyStats {
  if (totalDurationUs <= 0) {
    return {
      total_duration_us: 0,
      active_duration_us: 0,
      active_ratio: 0,
      active_avg_concurrency: 0,
      peak_concurrency: 0,
      peak_time_us: null,
      distribution: []
    };
  }

  let cursor = 0;
  let currentLevel = 0;
  const durationsByLevel: number[] = [];
  let peakConcurrency = 0;
  let peakTimeUs: number | null = null;

  const recordSegment = (level: number, startUs: number, dt: number) => {
    if (dt <= 0) return;
    while (durationsByLevel.length <= level) {
      durationsByLevel.push(0);
    }
    durationsByLevel[level] += dt;
    if (level > peakConcurrency) {
      peakConcurrency = level;
      peakTimeUs = startUs;
    }
  };

  for (const pt of metrics) {
    if (pt.time_us >= totalDurationUs) break;
    const dt = pt.time_us - cursor;
    recordSegment(currentLevel, cursor, dt);
    cursor = pt.time_us;
    currentLevel = pt.running_count;
  }
  recordSegment(currentLevel, cursor, totalDurationUs - cursor);

  let activeDurationUs = 0;
  let weightedActiveSum = 0;
  const distribution: ConcurrencyLevelStat[] = durationsByLevel.map((dur, level) => {
    if (level >= 1) {
      activeDurationUs += dur;
      weightedActiveSum += level * dur;
    }
    return {
      level,
      duration_us: dur,
      ratio: dur / totalDurationUs
    };
  });

  return {
    total_duration_us: totalDurationUs,
    active_duration_us: activeDurationUs,
    active_ratio: activeDurationUs / totalDurationUs,
    active_avg_concurrency: activeDurationUs === 0 ? 0 : weightedActiveSum / activeDurationUs,
    peak_concurrency: peakConcurrency,
    peak_time_us: peakTimeUs,
    distribution
  };
}

function computeMockLogStats(
  config: SchedulerConfig,
  cycles: ActualCycle[],
  executionsByCid: Map<number, ActualExecution[]>,
  instantEventsByCid: Map<number, ActualInstantEvent[]>,
  anomalyEvents: AnomalyEventItem[],
  metrics: ActualMetricPoint[],
  totalDurationUs: number
): LogStats {
  return {
    system: computeMockSystemStats(anomalyEvents, cycles),
    clients: computeMockClientStats(config, executionsByCid, instantEventsByCid),
    jitter: computeMockJitterStats(cycles),
    concurrency: computeMockConcurrencyStats(metrics, totalDurationUs)
  };
}

// -----------------------------------------------------------------------------
// In-Memory Full Log Generator
// -----------------------------------------------------------------------------

interface FullLogDataset {
  summary: LogSummary;
  actualExecutionsByCid: Map<number, ActualExecution[]>;
  actualInstantEventsByCid: Map<number, ActualInstantEvent[]>;
  anomalyEvents: AnomalyEventItem[];
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

  for (let cycle = 0; cycle < MOCK_PARAMS.total_cycles; cycle++) {
    if (cycle === 0) {
      actualCycles.push({
        cycle: 0,
        start_us: 0,
        start_jitter_us: 0,
      });
    } else {
      const isSpike = (cycle % MOCK_PARAMS.jitter.spike_interval_cycles === 0);
      const jitterUs = isSpike
        ? getRandomUs(MOCK_PARAMS.jitter.spike_min_us, MOCK_PARAMS.jitter.spike_max_us)
        : getRandomUs(MOCK_PARAMS.jitter.base_min_us, MOCK_PARAMS.jitter.base_max_us);

      currentCycleStartUs += MOCK_PARAMS.cycle_time_us + jitterUs;

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
  const anomalyEvents: AnomalyEventItem[] = [];

  function pushInstantEvent(
    cid: number,
    cycle: number,
    timeUs: number,
    eventType: InstantEventType
  ): void {
    const event: ActualInstantEvent = {
      log_line_no: nextLogLineNo++,
      instance_id: nextInstantEventId++,
      cycle,
      time_us: timeUs,
      event_type: eventType,
    };
    actualInstantEventsByCid.get(cid)?.push(event);
    if (ANOMALY_EVENT_SET.has(eventType)) {
      anomalyEvents.push({
        log_line_no: event.log_line_no,
        instance_id: event.instance_id,
        cid,
        cycle,
        time_us: timeUs,
        event_type: eventType,
      });
    }
  }

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
    pushInstantEvent(cid, cycle, readyTimeUs, 'ready');

    if (isOverrun) {
      // Overrun event recorded when scheduler crosses cycle boundary
      pushInstantEvent(cid, cycle, Math.round(boundaryUs), 'overrun');
      // Late event recorded when the overrun task finishes
      pushInstantEvent(cid, cycle, endUs, 'late');
    }

    return { endUs, status };
  }

  function recordSkip(cid: number, cycle: number, skipTimeUs: number): void {
    const roundedTime = Math.round(skipTimeUs);
    pushInstantEvent(cid, cycle, roundedTime, 'skip');

    // Ready event sent shortly after skip notification
    const readyTimeUs = roundedTime + getRandomUs(MOCK_PARAMS.ready_delay.min_us, MOCK_PARAMS.ready_delay.max_us);
    pushInstantEvent(cid, cycle, readyTimeUs, 'ready');
  }

  // Simulate even cycle executions: Camera -> (ObjectDetect & LaneDetect) -> Control
  let prevEvenEndUs = 0;

  for (let cycle = 0; cycle < MOCK_PARAMS.total_cycles; cycle += 2) {
    const cycleStartUs = actualCycles[cycle].start_us;
    const boundaryUs = actualCycles[cycle + 2]?.start_us ?? (cycleStartUs + MOCK_PARAMS.cycle_time_us * 2);

    // Check if previous even cycle dragged past this cycle's start time
    if (prevEvenEndUs > cycleStartUs) {
      recordSkip(1, cycle, cycleStartUs);
      recordSkip(101, cycle, cycleStartUs);
      recordSkip(102, cycle, cycleStartUs);
      recordSkip(501, cycle, cycleStartUs);
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
      recordSkip(501, cycle, boundaryUs);
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
      recordSkip(901, cycle, cycleStartUs);
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

  rawMetricEvents.forEach(evt => {
    currentRunning += evt.change;
    const running = Math.max(0, currentRunning);

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

  anomalyEvents.sort((a, b) =>
    (a.time_us - b.time_us) || (a.log_line_no - b.log_line_no) || (a.instance_id - b.instance_id)
  );

  const lastCycle = actualCycles[actualCycles.length - 1];
  const totalDurationUs = Math.ceil(
    lastCycle ? lastCycle.start_us + MOCK_PARAMS.cycle_time_us : MOCK_PARAMS.total_cycles * MOCK_PARAMS.cycle_time_us
  );

  const stats = computeMockLogStats(
    mockConfig,
    actualCycles,
    actualExecutionsByCid,
    actualInstantEventsByCid,
    anomalyEvents,
    actualMetrics,
    totalDurationUs
  );

  const summary: LogSummary = {
    config: mockConfig,
    total_duration_us: totalDurationUs,
    total_cycles: MOCK_PARAMS.total_cycles,
    stats,
    is_truncated: false
  };

  return {
    summary,
    actualExecutionsByCid,
    actualInstantEventsByCid,
    anomalyEvents,
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

/**
 * Simulates get_anomaly_events IPC command.
 * Returns filtered, paginated cross-client anomaly events without mutating the cached log.
 */
export async function mockGetAnomalyEvents(
  cid: number | null,
  eventTypes: InstantEventType[],
  offset: number,
  limit: number
): Promise<AnomalyEventsPage> {
  if (!fullLogCache) {
    fullLogCache = generateFullLogDataset();
  }

  await new Promise(resolve => setTimeout(resolve, 10));

  if (eventTypes.length === 0) {
    return { total_count: 0, items: [] };
  }

  const allowedTypes = new Set<InstantEventType>(eventTypes);
  const clampedLimit = Math.max(0, Math.min(MAX_ANOMALY_EVENTS_PAGE_LIMIT, limit));
  const safeOffset = Math.max(0, offset);

  const filtered = fullLogCache.anomalyEvents.filter(item => {
    if (cid !== null && item.cid !== cid) return false;
    return allowedTypes.has(item.event_type);
  });

  return {
    total_count: filtered.length,
    items: filtered.slice(safeOffset, safeOffset + clampedLimit)
  };
}

/**
 * Simulates recompute_client_budget IPC command.
 * Evaluates a candidate expected_duration_ms against immutable mock execution data.
 */
export async function mockRecomputeClientBudget(
  cid: number,
  expectedDurationMs: number
): Promise<ClientBudgetStats | null> {
  if (!fullLogCache) {
    fullLogCache = generateFullLogDataset();
  }

  await new Promise(resolve => setTimeout(resolve, 10));

  const clientStats = fullLogCache.summary.stats.clients.find(c => c.cid === cid);
  if (!clientStats || !clientStats.duration) {
    return null;
  }

  const executions = fullLogCache.actualExecutionsByCid.get(cid);
  if (!executions) {
    return null;
  }

  return computeMockClientBudget(executions, clientStats.duration, expectedDurationMs);
}
