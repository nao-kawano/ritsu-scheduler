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

import type { PlannedExecution } from '../types/simulation';
import type { ActualCycle, ActualMetricPoint } from '../types/analyze';
import { groupPlansByAnchorCycle, filterVisibleActualCycles } from './cycle';

/**
 * Discrete time and level step point for concurrency waveform rendering.
 */
export interface ConcurrencyStep {
  timeMs: number;
  count: number;
}

/**
 * Compute continuous concurrency step points on the actual physical timeline
 * by synthesizing visible planned executions across actual cycles.
 */
export function computePlannedConcurrencySteps(
  plannedExecs: PlannedExecution[],
  actualCycles: ActualCycle[],
  templateCycles: number,
  startMs: number,
  endMs: number
): ConcurrencyStep[] {
  // Group planned executions by anchor cycle phase (excluding skipped tasks)
  const plansByAnchorCycle = groupPlansByAnchorCycle(plannedExecs, true);

  // Filter visible actual cycles within viewport time bounds
  const visibleCycles = filterVisibleActualCycles(actualCycles, startMs, endMs);
  if (visibleCycles.length === 0) return [];

  // Collect discrete start (+1) and end (-1) time events across visible planned instances
  const timeDeltaMap = new Map<number, number>();
  visibleCycles.forEach(ac => {
    const templateCycle = ac.cycle % templateCycles;
    const matchingPlans = plansByAnchorCycle.get(templateCycle);
    if (!matchingPlans) return;

    matchingPlans.forEach(plan => {
      const execStartMs = ac.start_ms + (plan.anchor_offset_ms || 0);
      const execEndMs = execStartMs + plan.duration_ms;

      timeDeltaMap.set(execStartMs, (timeDeltaMap.get(execStartMs) || 0) + 1);
      timeDeltaMap.set(execEndMs, (timeDeltaMap.get(execEndMs) || 0) - 1);
    });
  });

  if (timeDeltaMap.size === 0) return [];

  // Synthesize running concurrency waveform sorted chronologically
  const sortedTimes = Array.from(timeDeltaMap.keys()).sort((a, b) => a - b);
  const steps: ConcurrencyStep[] = [];
  let currentRunning = 0;

  sortedTimes.forEach(timeMs => {
    currentRunning += timeDeltaMap.get(timeMs)!;
    steps.push({
      timeMs,
      count: Math.max(0, currentRunning)
    });
  });

  return steps;
}

/**
 * Find the actual running process count at timeMs using binary search.
 */
export function findActualConcurrencyForTime(actuals: ActualMetricPoint[], timeMs: number): number {
  if (!actuals || actuals.length === 0) return 0;
  let low = 0;
  let high = actuals.length - 1;
  let count = 0;
  while (low <= high) {
    const mid = (low + high) >> 1;
    if (actuals[mid].time_ms <= timeMs) {
      count = actuals[mid].running_count;
      low = mid + 1;
    } else {
      high = mid - 1;
    }
  }
  return count;
}

/**
 * Find the planned running process count at timeMs using binary search on cached steps.
 */
export function findPlannedConcurrencyForTime(steps: ConcurrencyStep[], timeMs: number): number | null {
  if (!steps || steps.length === 0) return null;
  if (timeMs < steps[0].timeMs) return 0;
  let low = 0;
  let high = steps.length - 1;
  let count = 0;
  while (low <= high) {
    const mid = (low + high) >> 1;
    if (steps[mid].timeMs <= timeMs) {
      count = steps[mid].count;
      low = mid + 1;
    } else {
      high = mid - 1;
    }
  }
  return count;
}
