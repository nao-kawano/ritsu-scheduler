<script setup lang="ts">
import { ref, computed } from 'vue';
import { useAppState } from '../composables/useAppState';
import { useTimeScale } from '../composables/useTimeScale';
import { useCreateModeLayout } from '../composables/useCreateModeLayout';

// --- State and Composables ---
const { config, planned_metrics } = useAppState();
const { cycleTimeMs, getPos } = useTimeScale();
const { totalCycles, gridInfo, totalWidth } = useCreateModeLayout();

// --- Viewport and Scrolling ---
const headerScrollEl = ref<HTMLElement | null>(null);
const contentScrollEl = ref<HTMLElement | null>(null);

const emit = defineEmits<{
  (e: 'scroll', event: Event): void
}>();

const onScroll = (e: Event) => {
  emit('scroll', e);
};

defineExpose({
  headerScrollEl,
  contentScrollEl
});

// --- Chart Constants ---
const METRICS_HEIGHT = 84; // Height of each metric chart row (px) - Matching ROW_HEIGHT in Timeline

// --- Path Generation ---

/**
 * Calculate the SVG path for the "Concurrent Processes" Area Chart.
 * This generates a "Staircase" (Step) chart to accurately reflect
 * instantaneous changes in the number of running processes.
 */
const areaPath = computed(() => {
  if (planned_metrics.value.length === 0) return '';

  // Find maximum count for normalization (Y-scaling)
  const counts = planned_metrics.value.map(m => m.running_count);
  let maxCount = counts.length > 0 ? Math.max(...counts) : 1;
  if (maxCount === 0) maxCount = 1;

  // Start path from bottom-left corner
  let path = `M 0,${METRICS_HEIGHT}`;

  for (let i = 0; i < planned_metrics.value.length; i++) {
    const current = planned_metrics.value[i];
    const x = getPos(current.time_ms);

    // Calculate Y coordinate (inverted for SVG coordinates)
    // Leaves a small top margin (10px) for visual comfort.
    const y = METRICS_HEIGHT - (current.running_count / maxCount) * (METRICS_HEIGHT - 10);

    if (i === 0) {
      // First point: move to ground, then up to the value
      path += ` L ${x},${METRICS_HEIGHT} L ${x},${y}`;
    } else {
      // Step Chart Logic:
      // 1. Draw horizontal line from previous X to current X (holding previous value)
      // 2. Draw vertical line at current X to the new value
      const prev = planned_metrics.value[i - 1];
      const prevY = METRICS_HEIGHT - (prev.running_count / maxCount) * (METRICS_HEIGHT - 10);
      path += ` L ${x},${prevY} L ${x},${y}`;
    }
  }

  // Close the area by dropping to the ground line and closing back to start
  const lastX = getPos(planned_metrics.value[planned_metrics.value.length - 1].time_ms);
  path += ` L ${lastX},${METRICS_HEIGHT} Z`;

  return path;
});
</script>

<template>
  <main class="metrics-pane" :key="config.sessionId">
    <!-- Time Header (Cycle and ms markers, synced across panes) -->
    <div class="metrics-header sb-hide-all sb-pad-v" ref="headerScrollEl">
      <div class="time-axis" :style="{ width: totalWidth + 'px' }">
        <div v-for="n in totalCycles" :key="n" class="time-tick" :style="{ width: gridInfo.majorPx + 'px' }">
          <span class="cycle-label">Cycle {{ n - 1 }}</span>
          <span class="time-label">{{ (n - 1) * cycleTimeMs }}ms</span>
        </div>
      </div>
    </div>

    <!-- Scrollable Content Area -->
    <div class="scroll-area metrics-scroll sb-hide-v sb-pad-v" ref="contentScrollEl" @scroll="onScroll">
      <div class="metrics-container sb-pad-v" :style="{
        width: totalWidth + 'px',
        backgroundSize: `${gridInfo.majorPx}px 100%, ${gridInfo.minorPx}px 100%`
      }">
        <!-- Row 1: Concurrent Processes Area Chart -->
        <div class="metrics-row" :class="{ 'info-row': planned_metrics.length === 0 }">
          <svg v-if="planned_metrics.length > 0" class="metrics-svg" :width="totalWidth" :height="METRICS_HEIGHT">
            <path :d="areaPath" class="planned-processes-path" />
          </svg>
          <div v-else class="placeholder-text">No simulation data available</div>
        </div>

        <!-- Row 2: Cycle Jitter (Placeholder for Analyze Mode) -->
        <div class="metrics-row info-row">
          <div class="placeholder-text">Cycle Jitter Line Chart (Analyze Mode Only)</div>
        </div>
      </div>
    </div>
  </main>
</template>

<style scoped>
/* ==========================================================================
   1. Layout and Containers
   ========================================================================== */

.metrics-pane {
  display: flex;
  flex-direction: column;
  background-color: var(--pane-bg);
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  height: 100%;
}

/* --- Header Section --- */
.metrics-header {
  height: var(--header-row-height);
  overflow: hidden;
  flex-shrink: 0;
  border-bottom: 1px solid var(--border-color);
  background: rgba(0, 0, 0, 0.02);
}

.time-axis {
  display: flex;
  height: 100%;
}

.time-tick {
  height: 100%;
  border-right: 1px solid var(--border-color);
  padding: 0 0.5rem;
  display: flex;
  flex-direction: column;
  justify-content: center;
  font-size: 0.7rem;
  flex-shrink: 0;
}

.cycle-label {
  font-weight: bold;
  color: var(--text-main);
}

.time-label {
  color: var(--text-dim);
  font-size: 0.65rem;
}

/* --- Content Section --- */
.scroll-area {
  flex: 1;
  min-height: 0;
  min-width: 0;
}

.metrics-scroll {
  overflow-x: scroll;
  overflow-y: hidden;
  width: 100%;
  height: 100%;
}

.metrics-container {
  min-height: 100%;
  /* Visual grid synchronization using CSS linear-gradients */
  background-image:
    linear-gradient(90deg, rgba(128, 128, 128, 0.3) 1px, transparent 1px),
    linear-gradient(90deg, rgba(128, 128, 128, 0.1) 1px, transparent 1px);
  background-position: -1px 0, -1px 0;
}

/* Base row for metric charts (no padding to allow full-width SVG alignment) */
.metrics-row {
  height: var(--row-height);
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  position: relative;
  padding: 0;
}

/* Row specialized for textual information or placeholders */
.info-row {
  padding: 0 1rem;
}

/* ==========================================================================
   2. SVG Chart Components
   ========================================================================== */

.metrics-svg {
  position: absolute;
  top: 0;
  left: 0;
  pointer-events: none;
}

/* Concurrent processes count area chart (staircase style) */
.planned-processes-path {
  fill: var(--primary-color);
  fill-opacity: 0.4;
  stroke: var(--primary-color);
  stroke-width: 1.5;
  /* Smooth transition for path changes */
  transition: d 0.3s ease;
}

/* ==========================================================================
   3. Informational UI
   ========================================================================== */

.placeholder-text {
  font-size: 0.75rem;
  color: var(--text-dim);
  opacity: 0.4;
}
</style>
