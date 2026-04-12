import { ref, computed } from 'vue';
import { useAppState } from './useAppState';

// Singleton state to synchronize zoom across all components
const pxPerCycle = ref(500);

/**
 * Time Scale Engine (Core)
 * Provides coordinate transformations (ms <-> px) and shared zoom state.
 * This is a low-level utility used by mode-specific layout composables.
 */
export function useTimeScale() {
  const { config } = useAppState();

  // Basic time unit from server config
  const cycleTimeMs = computed(() => config.server_config.cycle_time_ms || 100);

  // Pixels per millisecond (Current zoom level)
  const pxPerMs = computed(() => pxPerCycle.value / cycleTimeMs.value);

  /**
   * Convert time (ms) to horizontal pixel position (x).
   * @param ms Absolute time in milliseconds.
   * @param originMs Viewport origin in milliseconds.
   */
  const getPos = (ms: number, originMs: number = 0) => {
    return (ms - originMs) * pxPerMs.value;
  };

  /**
   * Convert horizontal pixel position (x) to time (ms).
   * @param x Pixel position relative to origin.
   * @param originMs Viewport origin in milliseconds.
   */
  const getMs = (x: number, originMs: number = 0) => {
    return originMs + (x / pxPerMs.value);
  };

  /**
   * Zoom control
   */
  const zoom = (direction: 'in' | 'out') => {
    const step = 50;
    const min = 100;
    const max = 5000;

    if (direction === 'in') {
      pxPerCycle.value = Math.min(pxPerCycle.value + step, max);
    } else {
      pxPerCycle.value = Math.max(pxPerCycle.value - step, min);
    }
  };

  return {
    pxPerCycle,
    cycleTimeMs,
    pxPerMs,
    getPos,
    getMs,
    zoom,
  };
}
