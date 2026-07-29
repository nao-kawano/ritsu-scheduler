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
import { useTimeScale } from './useTimeScale';

/**
 * Analyze Mode Layout Engine
 * Provides layout calculations specific to log analysis and execution visualization (Analyze Mode).
 * 
 * NOTE: Currently uses fixed values for early skeleton rendering, but designed
 * to mirror `useCreateModeLayout` so that log summary metadata (e.g. total logged cycles)
 * can easily replace these calculations in subsequent phases without breaking consumers.
 */
export function useAnalyzeModeLayout() {
  const { pxPerCycle, cycleTimeMs } = useTimeScale();

  /**
   * Temporary constant for initial Analyze mode skeleton rendering.
   * Will be replaced by log data bounds in Phase 4-C (Log Analysis Backend).
   */
  const TOTAL_CYCLES = 4;

  /**
   * Calculate how many cycles to render in Analyze Mode.
   */
  const totalCycles = computed(() => TOTAL_CYCLES);

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
    cycleTimeMs
  };
}
