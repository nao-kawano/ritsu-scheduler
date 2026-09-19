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
<script setup lang="ts">
import { ref, computed } from 'vue';
import { useAppState } from '../composables/useAppState';
import { useTimeScale } from '../composables/useTimeScale';
import { useCreateModeLayout } from '../composables/useCreateModeLayout';

// --- State and Composables ---
const { configCreateMode, plannedMetricsCreateMode } = useAppState();
const { cycleTimeMs, getPos } = useTimeScale();
const { totalCycles, gridInfo, totalWidth } = useCreateModeLayout();

// -----------------------------------------------------------------------------
// Props and Emits

const emit = defineEmits<{
  (e: 'scroll', event: Event): void
}>();

// -----------------------------------------------------------------------------
// State, Computed, and Logic

// --- Layout Constants ---

const METRICS_HEIGHT = 70; // Height of each metric chart row (px) - Matching ROW_HEIGHT in Timeline
const CHART_TOP_MARGIN = 10; // Top margin to prevent charts from touching top row border (px)
const CHART_STROKE_WIDTH = 2; // Stroke line width for step chart contour (px)

// --- Viewport and Scrolling ---

const headerScrollEl = ref<HTMLElement | null>(null);
const contentScrollEl = ref<HTMLElement | null>(null);

const onScroll = (e: Event) => {
  emit('scroll', e);
};

// --- Path Generation ---

/**
 * Calculate the SVG path for the "Concurrent Processes" Area Fill.
 * Generates a closed staircase polygon for background area fill.
 */
const areaPath = computed(() => {
  if (plannedMetricsCreateMode.value.length === 0) return '';

  // Find maximum count for normalization (Y-scaling)
  const counts = plannedMetricsCreateMode.value.map(m => m.running_count);
  let maxCount = counts.length > 0 ? Math.max(...counts) : 1;
  if (maxCount === 0) maxCount = 1;

  // Start path from bottom-left corner
  let path = `M 0,${METRICS_HEIGHT}`;

  for (let i = 0; i < plannedMetricsCreateMode.value.length; i++) {
    const current = plannedMetricsCreateMode.value[i];
    const x = getPos(current.time_ms);

    // Calculate Y coordinate (inverted for SVG coordinates)
    const y = METRICS_HEIGHT - (current.running_count / maxCount) * (METRICS_HEIGHT - CHART_TOP_MARGIN);

    if (i === 0) {
      // First point: move to ground, then up to the value
      path += ` L ${x},${METRICS_HEIGHT} L ${x},${y}`;
    } else {
      // Step Chart Logic:
      // 1. Draw horizontal line from previous X to current X (holding previous value)
      // 2. Draw vertical line at current X to the new value
      const prev = plannedMetricsCreateMode.value[i - 1];
      const prevY = METRICS_HEIGHT - (prev.running_count / maxCount) * (METRICS_HEIGHT - CHART_TOP_MARGIN);
      path += ` L ${x},${prevY} L ${x},${y}`;
    }
  }

  // Close the area by dropping to the ground line and closing back to start
  const lastX = getPos(plannedMetricsCreateMode.value[plannedMetricsCreateMode.value.length - 1].time_ms);
  path += ` L ${lastX},${METRICS_HEIGHT} Z`;

  return path;
});

/**
 * Calculate the SVG path for the "Concurrent Processes" Top Edge Line.
 * Generates an open staircase polyline tracing the upper contour of running processes.
 * Clamps bottom Y coordinate by half stroke width to keep the line fully within the row boundary.
 */
const linePath = computed(() => {
  if (plannedMetricsCreateMode.value.length === 0) return '';

  // Find maximum count for normalization (Y-scaling)
  const counts = plannedMetricsCreateMode.value.map(m => m.running_count);
  let maxCount = counts.length > 0 ? Math.max(...counts) : 1;
  if (maxCount === 0) maxCount = 1;

  const maxLineY = METRICS_HEIGHT - CHART_STROKE_WIDTH / 2;
  let path = '';

  for (let i = 0; i < plannedMetricsCreateMode.value.length; i++) {
    const current = plannedMetricsCreateMode.value[i];
    const x = getPos(current.time_ms);

    // Calculate Y coordinate with bottom boundary clamping for stroke width preservation
    const rawY = METRICS_HEIGHT - (current.running_count / maxCount) * (METRICS_HEIGHT - CHART_TOP_MARGIN);
    const y = Math.min(rawY, maxLineY);

    if (i === 0) {
      path += `M ${x},${y}`;
    } else {
      const prev = plannedMetricsCreateMode.value[i - 1];
      const prevRawY = METRICS_HEIGHT - (prev.running_count / maxCount) * (METRICS_HEIGHT - CHART_TOP_MARGIN);
      const prevY = Math.min(prevRawY, maxLineY);
      path += ` L ${x},${prevY} L ${x},${y}`;
    }
  }

  return path;
});

// -----------------------------------------------------------------------------
// Expose for App / ScrollSync

defineExpose({
  headerScrollEl,
  contentScrollEl
});
</script>

<template>
  <main class="metrics-pane" :key="configCreateMode.sessionId">
    <!-- Time Header (Cycle and ms markers, synced across panes) -->
    <div class="timeline-header sb-hide-all sb-pad-v" ref="headerScrollEl">
      <div class="time-axis" :style="{ width: totalWidth + 'px' }">
        <div v-for="n in totalCycles" :key="n" class="time-tick" :style="{ width: gridInfo.majorPx + 'px' }">
          <span class="cycle-label">Cycle {{ n - 1 }}</span>
          <span class="time-label">{{ (n - 1) * cycleTimeMs }}ms</span>
        </div>
      </div>
    </div>

    <!-- Scrollable Content Area -->
    <div class="scroll-area metrics-scroll sb-hide-v sb-pad-v" ref="contentScrollEl" @scroll="onScroll">
      <div class="metrics-content" :style="{
        width: totalWidth + 'px',
        backgroundSize: `${gridInfo.majorPx}px 100%, ${gridInfo.minorPx}px 100%`
      }">
        <!-- Row 1: Concurrent Processes Area Chart -->
        <div class="metrics-row" :class="{ 'info-row': plannedMetricsCreateMode.length === 0 }">
          <svg v-if="plannedMetricsCreateMode.length > 0" class="metrics-svg" :width="totalWidth"
            :height="METRICS_HEIGHT">
            <path :d="areaPath" class="planned-processes-area" />
            <path :d="linePath" class="planned-processes-line" />
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
   Layout and Containers
   ========================================================================== */

.metrics-pane {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  height: 100%;
  background-color: var(--rt-color-surface);
  overflow: hidden;
}

/* --- Header Section --- */
.timeline-header {
  flex-shrink: 0;
  height: var(--header-row-height);
  background: var(--rt-color-surface-header);
  border-bottom: var(--rt-border-main);
  overflow: hidden;
}

.time-axis {
  display: flex;
  height: 100%;
}

.time-tick {
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  justify-content: center;
  height: 100%;
  padding: 0 8px;
  border-right: var(--rt-border-main);
  font-size: var(--rt-font-xs);
}

.cycle-label {
  font-weight: bold;
  color: var(--rt-color-text);
}

.time-label {
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
  opacity: 0.8;
}

/* --- Content Section --- */
.scroll-area {
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.metrics-scroll {
  width: 100%;
  height: 100%;
  overflow-x: scroll;
  overflow-y: hidden;
}

.metrics-content {
  position: relative;
  min-height: 100%;
  /* Visual grid synchronization using CSS linear-gradients */
  background-image:
    linear-gradient(90deg, var(--rt-grid-major) 1px, transparent 1px),
    linear-gradient(90deg, var(--rt-grid-minor) 1px, transparent 1px);
  background-position: -1px 0, -1px 0;
}

/* Base row for metric charts */
.metrics-row {
  height: var(--row-height);
  border-bottom: var(--rt-border-main);
}

/* Row specialized for textual information or placeholders */
.info-row {
  display: flex;
  align-items: center;
  padding: 0 16px;
}

/* ==========================================================================
   SVG Chart Components
   ========================================================================== */

.metrics-svg {
  position: absolute;
  top: 0;
  left: 0;
  /* Let scroll events pass through to container */
  pointer-events: none;
}

/* Concurrent processes count area fill (translucent filled step polygon) */
.planned-processes-area {
  fill: var(--rt-color-primary);
  fill-opacity: 0.4;
  stroke: none;
  /* Smooth transition for path changes */
  transition: d 0.3s ease;
}

/* Concurrent processes count step line (solid top contour) */
.planned-processes-line {
  stroke: var(--rt-color-primary);
  stroke-width: v-bind('CHART_STROKE_WIDTH + "px"');
  fill: none;
  /* Smooth transition for path changes */
  transition: d 0.3s ease;
}

/* ==========================================================================
   Informational UI
   ========================================================================== */

.placeholder-text {
  font-size: 12px;
  color: var(--rt-color-text-dim);
  opacity: 0.4;
}
</style>
