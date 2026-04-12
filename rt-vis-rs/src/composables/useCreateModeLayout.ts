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
   * Based on (max_cycle + 1) to cover all process patterns.
   */
  const totalCycles = computed(() => {
    if (!config.client_configs || config.client_configs.length === 0) return 2;
    const maxCycle = Math.max(...config.client_configs.map(c => c.cycle));
    return maxCycle + 1;
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
