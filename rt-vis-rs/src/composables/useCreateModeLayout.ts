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

/**
 * Create Mode Layout Engine
 * Provides layout calculations specific to the schedule editor (Create Mode).
 */
export function useCreateModeLayout() {
  const { config } = useAppState();
  const { pxPerCycle } = useTimeScale();

  /**
   * Calculate how many cycles to show in Create Mode.
   * Based on (maxCycle * 2) to cover all process patterns including offsets.
   * NOTE: Keep in sync with backend: simulator.rs -> max_manager_cycle
   */
  const totalCycles = computed(() => {
    if (!config.client_configs || config.client_configs.length === 0) return 2;
    const maxCycle = Math.max(...config.client_configs.map(c => c.data.cycle));
    return maxCycle * 2;
  });

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
