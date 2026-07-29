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
import { useCanvasRender } from '../composables/useCanvasRender';

// --- State and Composables ---
const { activeConfig } = useAppState();
const { totalCycles, totalWidth, gridInfo, cycleTimeMs } = useAnalyzeModeLayout();
const { getThemeStyles, prepareCanvas, renderTimelineHeader } = useCanvasRender();

// -----------------------------------------------------------------------------
// Props and Emits

const emit = defineEmits<{
  (e: 'scroll', event: Event): void
}>();

// -----------------------------------------------------------------------------
// Layout Constants

const ROW_HEIGHT = 70; // Height of each metric chart row in pixels (matching Create Mode)

// -----------------------------------------------------------------------------
// Elements & Scroll Handling

const headerScrollEl = ref<HTMLElement | null>(null);
const contentScrollEl = ref<HTMLElement | null>(null);
const headerCanvasEl = ref<HTMLCanvasElement | null>(null);
const contentCanvasEl = ref<HTMLCanvasElement | null>(null);

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

  const styles = getThemeStyles(headerScrollEl.value);

  renderTimelineHeader(ctx, {
    scrollLeft,
    width,
    height,
    totalCycles: totalCycles.value,
    cycleTimeMs: cycleTimeMs.value,
    pxPerCycle: gridInfo.value.majorPx,
    styles
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

  const styles = getThemeStyles(container);
  const pxPerCycle = gridInfo.value.majorPx;
  const minorInterval = gridInfo.value.minorPx;
  const numCycles = totalCycles.value;

  // Background fill
  ctx.fillStyle = styles.surface;
  ctx.fillRect(0, 0, width, height);

  // Minor Grids
  const startMinor = Math.max(0, Math.floor(scrollLeft / minorInterval));
  const endMinor = Math.min(numCycles * 10, Math.floor((scrollLeft + width) / minorInterval));

  ctx.strokeStyle = styles.gridMinor;
  ctx.lineWidth = 1;
  ctx.beginPath();
  for (let i = startMinor; i <= endMinor; i++) {
    if (i % 10 === 0) continue; // Skip major grid lines
    const x = Math.floor(i * minorInterval - scrollLeft) - 0.5;
    ctx.moveTo(x, 0);
    ctx.lineTo(x, height);
  }
  ctx.stroke();

  // Major Grids (matching exact tick border X alignment)
  const startMajor = Math.max(0, Math.floor(scrollLeft / pxPerCycle));
  const endMajor = Math.min(numCycles, Math.floor((scrollLeft + width) / pxPerCycle));

  ctx.strokeStyle = styles.gridMajor;
  ctx.lineWidth = 1;
  ctx.beginPath();
  for (let i = startMajor; i <= endMajor; i++) {
    const x = Math.floor(i * pxPerCycle - scrollLeft) - 0.5;
    ctx.moveTo(x, 0);
    ctx.lineTo(x, height);
  }
  ctx.stroke();

  // Metrics horizontal row borders (separating Concurrent Processes and Cycle Jitter rows)
  ctx.strokeStyle = styles.border;
  ctx.lineWidth = 1;
  ctx.beginPath();
  for (let r = 1; r <= 2; r++) {
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
let resizeObserver: ResizeObserver | null = null;

onMounted(() => {
  nextTick(() => {
    renderAll();
  });

  if (contentScrollEl.value) {
    resizeObserver = new ResizeObserver(() => {
      renderAll();
    });
    resizeObserver.observe(contentScrollEl.value);
  }
});

onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.disconnect();
  }
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
