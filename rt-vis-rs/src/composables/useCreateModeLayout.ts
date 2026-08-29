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
import { computed } from 'vue';
import { useAppState } from './useAppState';
import { useTimeScale } from './useTimeScale';
import { getSimulationCycles } from '../utils/simulation';

/**
 * Create Mode Layout Engine
 * Provides layout calculations specific to the schedule editor (Create Mode).
 */
export function useCreateModeLayout() {
  const { configCreateMode } = useAppState();
  const { pxPerCycle } = useTimeScale();

  /**
   * Calculate how many cycles to render in Create Mode.
   * Derived from the shared simulation cycle formula.
   */
  const totalCycles = computed(() => getSimulationCycles(configCreateMode.client_configs));

  /**
   * Total width of the timeline in pixels for the current zoom level.
   */
  const totalWidth = computed(() => totalCycles.value * pxPerCycle.value);

  /**
   * Grid intervals in pixels.
   * Major: 1 full cycle
   * Minor: 1/10 of a cycle
   */
  const gridInfo = computed(() => {
    return {
      majorPx: pxPerCycle.value,
      minorPx: pxPerCycle.value / 10,
    };
  });

  return {
    totalCycles,
    totalWidth,
    gridInfo,
  };
}
