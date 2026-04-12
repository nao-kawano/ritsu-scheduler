import { ref, computed } from 'vue';
import { useAppState } from './useAppState';

// Singleton state to synchronize zoom across all components
const pxPerCycle = ref(500);

// Predefined zoom steps for pxPerCycle
const ZOOM_STEPS = [75, 125, 250, 375, 500, 750, 1000, 2000];
const DEFAULT_PX_PER_CYCLE = 500;

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

  // Current zoom percentage based on default 500px/cycle
  const zoomPercent = computed(() => Math.round((pxPerCycle.value / DEFAULT_PX_PER_CYCLE) * 100));

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
    const currentIndex = ZOOM_STEPS.indexOf(pxPerCycle.value);

    // If current value is not in steps (e.g. manual edit), find the closest one
    if (currentIndex === -1) {
      if (direction === 'in') {
        pxPerCycle.value = ZOOM_STEPS.find(s => s > pxPerCycle.value) || ZOOM_STEPS[ZOOM_STEPS.length - 1];
      } else {
        pxPerCycle.value = [...ZOOM_STEPS].reverse().find(s => s < pxPerCycle.value) || ZOOM_STEPS[0];
      }
      return;
    }

    if (direction === 'in') {
      if (currentIndex < ZOOM_STEPS.length - 1) {
        pxPerCycle.value = ZOOM_STEPS[currentIndex + 1];
      }
    } else {
      if (currentIndex > 0) {
        pxPerCycle.value = ZOOM_STEPS[currentIndex - 1];
      }
    }
  };

  /**
   * Reset zoom to default
   */
  const resetZoom = () => {
    pxPerCycle.value = DEFAULT_PX_PER_CYCLE;
  };

  return {
    pxPerCycle,
    cycleTimeMs,
    pxPerMs,
    zoomPercent,
    minZoom: ZOOM_STEPS[0],
    maxZoom: ZOOM_STEPS[ZOOM_STEPS.length - 1],
    getPos,
    getMs,
    zoom,
    resetZoom,
  };
}
