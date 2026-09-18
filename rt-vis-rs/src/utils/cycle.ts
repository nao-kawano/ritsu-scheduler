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

import type { ClientConfig, ClientConfigUI } from '../types/config';
import type { PlannedExecution } from '../types/simulation';
import type { ActualCycle } from '../types/analyze';

/**
 * Calculates the total simulation cycles to render/calculate.
 * Formula: max(1, max_cycle_of_clients) * 2
 * NOTE: Keep in sync with backend: simulator.rs -> max_manager_cycle
 */
export function getSimulationCycles(configs: ClientConfig[] | ClientConfigUI[]): number {
  if (!configs || configs.length === 0) return 2;

  const maxCycle = Math.max(
    1,
    ...configs.map(c => {
      if ('data' in c) {
        return c.data.cycle || 1;
      }
      return c.cycle || 1;
    })
  );

  return maxCycle * 2;
}

/**
 * Group planned executions by anchor cycle phase for efficient lookup.
 * Optionally exclude skipped execution instances.
 */
export function groupPlansByAnchorCycle(
  plans: PlannedExecution[],
  excludeSkips: boolean = false
): Map<number, PlannedExecution[]> {
  const map = new Map<number, PlannedExecution[]>();

  plans.forEach(plan => {
    if (excludeSkips && plan.status === 'skip') return;

    const list = map.get(plan.anchor_cycle) || [];
    list.push(plan);
    map.set(plan.anchor_cycle, list);
  });

  return map;
}

/**
 * Filter actual cycle records that fall within the specified physical time range.
 */
export function filterVisibleActualCycles(
  actualCycles: ActualCycle[],
  startMs: number,
  endMs: number
): ActualCycle[] {
  return actualCycles.filter(ac => ac.start_ms >= startMs && ac.start_ms <= endMs);
}

/**
 * Find the actual cycle corresponding to timeMs using binary search.
 * Returns the cycle whose start_ms is the greatest value <= timeMs.
 */
export function findActualCycleForTime(cycles: ActualCycle[], timeMs: number): ActualCycle | null {
  if (!cycles || cycles.length === 0) return null;
  let low = 0;
  let high = cycles.length - 1;
  let found: ActualCycle | null = null;
  while (low <= high) {
    const mid = (low + high) >> 1;
    if (cycles[mid].start_ms <= timeMs) {
      found = cycles[mid];
      low = mid + 1;
    } else {
      high = mid - 1;
    }
  }
  return found;
}
