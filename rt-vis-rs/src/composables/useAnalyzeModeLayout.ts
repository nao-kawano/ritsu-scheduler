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
import { useExecutionLog } from './useExecutionLog';

/**
 * Analyze Mode Layout Engine
 * Provides layout calculations specific to log analysis and execution visualization (Analyze Mode).
 */
export function useAnalyzeModeLayout() {
  const { pxPerCycle, cycleTimeMs } = useTimeScale();
  const { logSummaryAnalyzeMode } = useExecutionLog();

  /**
   * Calculate how many cycles to render in Analyze Mode.
   * Dynamically derived from the loaded log summary metadata total_cycles.
   */
  const totalCycles = computed(() => {
    if (logSummaryAnalyzeMode.value && logSummaryAnalyzeMode.value.total_cycles > 0) {
      return logSummaryAnalyzeMode.value.total_cycles;
    }
    return 4; // Fallback skeleton cycle count before log is loaded
  });

  /**
   * Total duration in milliseconds for Analyze Mode.
   * Uses total_duration_ms from log metadata, or falls back to a 4-cycle skeleton duration before log load.
   */
  const totalDurationMs = computed(() => {
    if (logSummaryAnalyzeMode.value && logSummaryAnalyzeMode.value.total_duration_ms > 0) {
      return logSummaryAnalyzeMode.value.total_duration_ms;
    }
    return 4 * (cycleTimeMs.value || 50);
  });

  /**
   * Total width of the timeline in pixels for the current zoom level.
   * Based on linear physical time (pxPerMs).
   */
  const totalWidth = computed(() => {
    const pxPerMs = cycleTimeMs.value > 0 ? pxPerCycle.value / cycleTimeMs.value : 0;
    return Math.ceil(totalDurationMs.value * pxPerMs);
  });

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
