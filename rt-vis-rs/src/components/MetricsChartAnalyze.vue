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
import { ref, onMounted, onUnmounted, nextTick } from 'vue';
import { useAppState } from '../composables/useAppState';
import { useAnalyzeModeLayout } from '../composables/useAnalyzeModeLayout';
import { useCanvasRender, type ThemeStyles } from '../composables/useCanvasRender';

// --- State and Composables ---
const { activeConfig } = useAppState();
const { totalCycles, totalWidth, gridInfo, cycleTimeMs } = useAnalyzeModeLayout();
const { getThemeStyles, prepareCanvas, renderTimelineHeader, renderBackgroundGrid } = useCanvasRender();

// -----------------------------------------------------------------------------
// Props and Emits

const emit = defineEmits<{
  (e: 'scroll', event: Event): void
}>();

// -----------------------------------------------------------------------------
// Layout Constants

const ROW_HEIGHT = 70; // Height of each metric chart row in pixels (matching Create Mode)
const METRIC_ROWS = 2; // Total metric chart rows (1: Concurrent Processes, 2: Cycle Jitter)

// -----------------------------------------------------------------------------
// State, Computed, and Logic

// --- Elements & Scroll Handling ---

const headerScrollEl = ref<HTMLElement | null>(null);
const contentScrollEl = ref<HTMLElement | null>(null);
const headerCanvasEl = ref<HTMLCanvasElement | null>(null);
const contentCanvasEl = ref<HTMLCanvasElement | null>(null);

// --- Theme Cache & Optimization ---

const cachedThemeStyles = ref<ThemeStyles | null>(null);

/**
 * Extract and cache theme styles to avoid costly getComputedStyle calls on every scroll event.
 */
const updateThemeStyles = () => {
  const container = contentScrollEl.value || headerScrollEl.value;
  if (container) {
    cachedThemeStyles.value = getThemeStyles(container);
  }
};

// --- Render Logic ---

/**
 * Render sticky time header canvas using shared canvas rendering composable.
 */
const renderHeader = () => {
  if (!headerCanvasEl.value || !headerScrollEl.value) return;

  const { width, height, ctx } = prepareCanvas(headerCanvasEl.value, headerScrollEl.value);
  if (!ctx) return;

  // ALWAYS use contentScrollEl's scrollLeft as master to guarantee 100% pixel sync across panes
  const scrollLeft = contentScrollEl.value ? contentScrollEl.value.scrollLeft : headerScrollEl.value.scrollLeft;

  // Pin canvas overlay dynamically to current scroll viewport to avoid clipping or blank bleeding
  headerCanvasEl.value.style.transform = `translate(${scrollLeft}px, 0px)`;

  if (!cachedThemeStyles.value) {
    updateThemeStyles();
  }
  if (!cachedThemeStyles.value) return;

  renderTimelineHeader(ctx, {
    scrollLeft,
    width,
    height,
    totalCycles: totalCycles.value,
    cycleTimeMs: cycleTimeMs.value,
    majorPx: gridInfo.value.majorPx,
    styles: cachedThemeStyles.value
  });
};

/**
 * Render metrics content background grid and row border into canvas context.
 */
const renderContent = () => {
  if (!contentCanvasEl.value || !contentScrollEl.value) return;

  const container = contentScrollEl.value;
  const scrollLeft = container.scrollLeft;
  const scrollTop = container.scrollTop;

  const { width, height, ctx } = prepareCanvas(contentCanvasEl.value, container);
  if (!ctx) return;

  // Pin canvas overlay dynamically to current scroll viewport to avoid clipping or blank bleeding
  contentCanvasEl.value.style.transform = `translate(${scrollLeft}px, ${scrollTop}px)`;

  if (!cachedThemeStyles.value) {
    updateThemeStyles();
  }
  if (!cachedThemeStyles.value) return;
  const styles = cachedThemeStyles.value;

  // Render shared background grid surface and vertical time grid lines
  renderBackgroundGrid(ctx, {
    scrollLeft,
    width,
    height,
    totalCycles: totalCycles.value,
    majorPx: gridInfo.value.majorPx,
    minorPx: gridInfo.value.minorPx,
    styles
  });

  // Metrics horizontal row borders (separating Concurrent Processes and Cycle Jitter rows)
  ctx.strokeStyle = styles.border;
  ctx.lineWidth = 1;
  ctx.beginPath();
  for (let r = 1; r <= METRIC_ROWS; r++) {
    const y = Math.floor(r * ROW_HEIGHT) - 0.5;
    ctx.moveTo(0, y);
    ctx.lineTo(width, y);
  }
  ctx.stroke();
};

const renderAll = () => {
  renderHeader();
  renderContent();
};

const onScroll = (e: Event) => {
  renderAll();
  emit('scroll', e);
};

// --- Lifecycle & Observers ---

/**
 * Handle viewport resize or theme attribute changes by updating theme cache and re-rendering.
 */
const onLayoutOrThemeChange = () => {
  updateThemeStyles();
  renderAll();
};

const resizeObserver = new ResizeObserver(onLayoutOrThemeChange);
const themeMutationObserver = new MutationObserver(onLayoutOrThemeChange);

onMounted(() => {
  updateThemeStyles();
  nextTick(() => renderAll());

  contentScrollEl.value && resizeObserver.observe(contentScrollEl.value);
  document.documentElement && themeMutationObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['class', 'style', 'data-theme']
  });
});

onUnmounted(() => {
  resizeObserver.disconnect();
  themeMutationObserver.disconnect();
});

// -----------------------------------------------------------------------------
// Expose for App / ScrollSync

defineExpose({
  headerScrollEl,
  contentScrollEl
});
</script>

<template>
  <main class="metrics-pane" :key="activeConfig.sessionId">
    <!-- Time Header Section (Cycle and time markers in Canvas overlay) -->
    <div class="timeline-header sb-hide-all sb-pad-v" ref="headerScrollEl" @scroll="onScroll">
      <div class="header-content" :style="{ width: totalWidth + 'px', height: '100%' }">
        <canvas ref="headerCanvasEl" class="canvas-layer"></canvas>
      </div>
    </div>

    <!-- Scrollable Content Section (Background Grid & Metrics Viewer) -->
    <div class="scroll-area metrics-scroll sb-hide-v sb-pad-v" ref="contentScrollEl" @scroll="onScroll">
      <div class="metrics-content" :style="{ width: totalWidth + 'px', height: (ROW_HEIGHT * 2) + 'px' }">
        <canvas ref="contentCanvasEl" class="canvas-layer"></canvas>
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
  position: relative;
  flex-shrink: 0;
  height: var(--header-row-height);
  background: var(--rt-color-surface-header);
  border-bottom: var(--rt-border-main);
  overflow: hidden;
}

.header-content {
  position: relative;
  min-height: 100%;
}

/* --- Content Section --- */
.scroll-area {
  position: relative;
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
}

/* --- Canvas Layer --- */
.canvas-layer {
  position: absolute;
  top: 0;
  left: 0;
  display: block;
  z-index: 1;
  pointer-events: auto;
}
</style>
