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
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue';
import { useAppState } from '../composables/useAppState';
import { useTimeScale } from '../composables/useTimeScale';
import { useAnalyzeModeLayout } from '../composables/useAnalyzeModeLayout';
import { useCanvasRender, type ThemeStyles } from '../composables/useCanvasRender';
import { getSimulationCycles, groupPlansByAnchorCycle, filterVisibleActualCycles } from '../utils/simulation';

// -----------------------------------------------------------------------------
// Global State & Composables

const {
  activeConfig,
  plannedExecutionsAnalyzeMode,
  logSummaryAnalyzeMode,
  logRangeDataAnalyzeMode,
  fetchLogRange
} = useAppState();
const { pxPerCycle, cycleTimeMs, pxPerMs } = useTimeScale();
const { totalCycles, totalWidth, gridInfo } = useAnalyzeModeLayout();
const { getThemeStyles, prepareCanvas, renderTimelineHeader, renderBackgroundGrid, renderActualCycleLines } = useCanvasRender();

// -----------------------------------------------------------------------------
// Props & Emits

const emit = defineEmits<{
  (e: 'scroll', event: Event): void
}>();

// -----------------------------------------------------------------------------
// Constants & Layout

const ROW_HEIGHT = 70;         // Fixed height of each process row in pixels (matching Create Mode)
const PLAN_RECT_HEIGHT = 46;  // Outer planned execution box height (matching Create Mode bar height)
const ACTUAL_RECT_HEIGHT = 28; // Inner solid actual execution bar height

// -----------------------------------------------------------------------------
// Local State & Computed

const headerScrollEl = ref<HTMLElement | null>(null);
const contentScrollEl = ref<HTMLElement | null>(null);
const headerCanvasEl = ref<HTMLCanvasElement | null>(null);
const contentCanvasEl = ref<HTMLCanvasElement | null>(null);

const cachedThemeStyles = ref<ThemeStyles | null>(null);

/**
 * Map Client ID to process row index for alignment.
 */
const cidToRowIndex = computed(() => {
  const map = new Map<number, number>();
  activeConfig.value.client_configs.forEach((c, idx) => map.set(c.data.client_id, idx));
  return map;
});

/**
 * Total content height calculation.
 * Includes client configs count plus 1 extra placeholder row (70px)
 * to maintain 100% layout and row-border parity with Create Mode.
 */
const totalContentHeight = computed(() => {
  const count = activeConfig.value.client_configs.length + 1;
  return count * ROW_HEIGHT;
});

// -----------------------------------------------------------------------------
// Methods & Logic

/**
 * Extract and cache theme styles to avoid costly getComputedStyle calls on every scroll event.
 */
const updateThemeStyles = () => {
  const container = contentScrollEl.value || headerScrollEl.value;
  if (container) {
    cachedThemeStyles.value = getThemeStyles(container);
  }
};

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
    styles: cachedThemeStyles.value,
    actualCycles: logRangeDataAnalyzeMode.value?.actual_cycles,
    pxPerMs: pxPerMs.value
  });
};

/**
 * Render planned execution boxes (outer translucent primary fills with dashed borders) into canvas context.
 * Uses viewport culling with cycle margins and vertical scroll offsets to restrict loop scope.
 */
const renderPlanBoxes = (
  ctx: CanvasRenderingContext2D,
  scrollLeft: number,
  scrollTop: number,
  width: number,
  height: number,
  styles: ThemeStyles,
  cidToRowIndex: Map<number, number>
) => {
  // ONLY render planned boxes after log is loaded
  if (!logSummaryAnalyzeMode.value) return;

  const plans = plannedExecutionsAnalyzeMode.value;
  if (!plans || plans.length === 0 || totalCycles.value <= 0 || cycleTimeMs.value <= 0) return;

  // Derive templateCycles (maxCycle * 2) using shared simulation utility
  const templateCycles = getSimulationCycles(activeConfig.value.client_configs);

  // Calculate culled time range with 2-cycle padding margin in physical milliseconds
  const marginMs = cycleTimeMs.value * 2;
  const startMs = Math.max(0, scrollLeft / pxPerMs.value - marginMs);
  const endMs = (scrollLeft + width) / pxPerMs.value + marginMs;

  const actualCycles = logRangeDataAnalyzeMode.value?.actual_cycles;
  if (!actualCycles || actualCycles.length === 0) return;

  // Filter visible actual cycles based on physical time bounds
  const visibleCycles = filterVisibleActualCycles(actualCycles, startMs, endMs);
  if (visibleCycles.length === 0) return;

  // Group plans by anchor cycle phase for fast lookup
  const plansByAnchorCycle = groupPlansByAnchorCycle(plans);

  // Phase alignment pattern: render exact matching template phase for each visible cycle
  visibleCycles.forEach(ac => {
    const c = ac.cycle;
    const templateCycle = c % templateCycles;
    const matchingPlans = plansByAnchorCycle.get(templateCycle);
    if (!matchingPlans) return;

    const cycleStartMs = ac.start_ms;

    matchingPlans.forEach(plan => {
      const r = cidToRowIndex.get(plan.cid);
      if (r === undefined) return;

      const startMs = cycleStartMs + (plan.anchor_offset_ms || 0);
      const x = Math.floor(startMs * pxPerMs.value - scrollLeft);
      const barWidth = Math.max(4, Math.floor(plan.duration_ms * pxPerMs.value));
      const y = Math.floor(r * ROW_HEIGHT - scrollTop + (ROW_HEIGHT - PLAN_RECT_HEIGHT) / 2);

      if (y + PLAN_RECT_HEIGHT >= 0 && y <= height && x + barWidth >= 0 && x <= width) {
        ctx.save();
        {
          let barColor = styles.primaryColor;
          if (plan.status === 'overrun') barColor = styles.errorColor;

          // Translucent primary fill matching Create Mode theme accent
          ctx.globalAlpha = 0.4;
          ctx.fillStyle = barColor;
          ctx.beginPath();
          if (typeof ctx.roundRect === 'function') {
            ctx.roundRect(x, y, barWidth, PLAN_RECT_HEIGHT, 6);
          } else {
            ctx.rect(x, y, barWidth, PLAN_RECT_HEIGHT);
          }
          if (plan.status != 'skip') {
            ctx.fill();
          }

          // Subtle dashed border outline
          ctx.strokeStyle = styles.primaryColor;
          ctx.lineWidth = 1;
          ctx.setLineDash([4, 4]);
          ctx.stroke();
        }
        ctx.restore();
      }
    });
  });
};

/**
 * Render actual execution bars (inner solid rounded fills) into canvas context.
 */
const renderActualBars = (
  ctx: CanvasRenderingContext2D,
  scrollLeft: number,
  scrollTop: number,
  width: number,
  height: number,
  styles: ThemeStyles,
  cidToRowIndex: Map<number, number>
) => {
  if (!logSummaryAnalyzeMode.value) return;

  const rangeData = logRangeDataAnalyzeMode.value;
  if (!rangeData || !rangeData.actual_executions_by_cid) return;

  rangeData.actual_executions_by_cid.forEach(([cid, actuals]) => {
    const r = cidToRowIndex.get(cid);
    if (r === undefined) return;

    actuals.forEach(actual => {
      // Guard against non-running or skipped executions with no elapsed duration
      if (actual.status === 'skip' || actual.duration_ms <= 0) return;

      const x = Math.floor(actual.start_ms * pxPerMs.value - scrollLeft);
      const barWidth = Math.max(4, Math.floor(actual.duration_ms * pxPerMs.value));
      const y = Math.floor(r * ROW_HEIGHT - scrollTop + (ROW_HEIGHT - ACTUAL_RECT_HEIGHT) / 2);

      if (y + ACTUAL_RECT_HEIGHT >= 0 && y <= height && x + barWidth >= 0 && x <= width) {
        ctx.save();
        {
          // Status color fill: use evaluated theme error color for overrun
          let barColor = styles.accentColor;
          if (actual.status === 'overrun') barColor = styles.errorColor;

          ctx.fillStyle = barColor;
          ctx.beginPath();
          if (typeof ctx.roundRect === 'function') {
            ctx.roundRect(x, y, barWidth, ACTUAL_RECT_HEIGHT, 4);
          } else {
            ctx.rect(x, y, barWidth, ACTUAL_RECT_HEIGHT);
          }
          ctx.fill();

          // Subtle highlight border
          ctx.strokeStyle = 'rgba(255, 255, 255, 0.6)';
          ctx.lineWidth = 2;
          ctx.stroke();
        }
        ctx.restore();
      }
    });
  });
};

/**
 * Render horizontal process row separator borders.
 */
const renderRowBorders = (
  ctx: CanvasRenderingContext2D,
  scrollTop: number,
  width: number,
  height: number,
  styles: ThemeStyles
) => {
  ctx.save();
  {
    const totalRows = activeConfig.value.client_configs.length + 1;
    ctx.strokeStyle = styles.borderColor;
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let r = 1; r <= totalRows; r++) {
      const y = Math.floor(r * ROW_HEIGHT - scrollTop) - 0.5;
      if (y >= 0 && y <= height) {
        ctx.moveTo(0, y);
        ctx.lineTo(width, y);
      }
    }
    ctx.stroke();
  }
  ctx.restore();
};

/**
 * Request asynchronous range fetch for visible physical time bounds if needed.
 * Adheres to optimistic rendering architecture: renders immediately using cached local data
 * for smooth frame rates while requesting missing range data in the background.
 */
const requestVisibleLogRange = (scrollLeft: number, width: number) => {
  if (logSummaryAnalyzeMode.value && cycleTimeMs.value > 0) {
    const startMs = Math.max(0, Math.floor(scrollLeft / pxPerMs.value));
    const endMs = Math.ceil((scrollLeft + width) / pxPerMs.value);
    fetchLogRange(startMs, endMs);
  }
};

/**
 * Render timeline content background grid, process row borders, plan boxes, and actual bars.
 */
const renderContent = () => {
  if (!contentCanvasEl.value || !contentScrollEl.value) return;

  const container = contentScrollEl.value;

  // Clamp scrollLeft to valid boundary if layout width shrank (e.g. cycle time increased)
  const maxScrollLeft = Math.max(0, container.scrollWidth - container.clientWidth);
  if (container.scrollLeft > maxScrollLeft && maxScrollLeft > 0) {
    container.scrollLeft = maxScrollLeft;
    if (headerScrollEl.value) {
      headerScrollEl.value.scrollLeft = maxScrollLeft;
    }
  }

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

  // Render actual cycle start lines overlay (double grid structure)
  if (logRangeDataAnalyzeMode.value?.actual_cycles && cycleTimeMs.value > 0) {
    renderActualCycleLines(ctx, {
      scrollLeft,
      width,
      height,
      actualCycles: logRangeDataAnalyzeMode.value.actual_cycles,
      pxPerMs: pxPerMs.value,
      styles
    });
  }

  renderRowBorders(ctx, scrollTop, width, height, styles);

  renderPlanBoxes(ctx, scrollLeft, scrollTop, width, height, styles, cidToRowIndex.value);
  renderActualBars(ctx, scrollLeft, scrollTop, width, height, styles, cidToRowIndex.value);

  requestVisibleLogRange(scrollLeft, width);
};

const renderAll = () => {
  renderHeader();
  renderContent();
};

// -----------------------------------------------------------------------------
// Event Handlers

let renderRafId: number | null = null;

const onScroll = (e: Event) => {
  if (renderRafId === null) {
    renderRafId = window.requestAnimationFrame(() => {
      renderAll();
      renderRafId = null;
    });
  }
  emit('scroll', e);
};

// -----------------------------------------------------------------------------
// Watchers & Reactive Triggers

/**
 * Automatically adjust scroll position to keep center physical time anchored upon scale/cycle changes.
 */
watch(pxPerMs, (newPx, oldPx) => {
  if (!contentScrollEl.value || !oldPx || !newPx || oldPx === newPx) return;

  const container = contentScrollEl.value;
  const viewportWidth = container.clientWidth;
  if (viewportWidth <= 0) return;

  const currentScrollLeft = container.scrollLeft;
  const centerMs = (currentScrollLeft + viewportWidth / 2) / oldPx;
  const targetScrollLeft = Math.floor(centerMs * newPx - viewportWidth / 2);

  nextTick(() => {
    if (!container) return;
    const maxScroll = Math.max(0, container.scrollWidth - viewportWidth);
    const clampedScrollLeft = Math.max(0, Math.min(maxScroll, targetScrollLeft));

    container.scrollLeft = clampedScrollLeft;
    if (headerScrollEl.value) {
      headerScrollEl.value.scrollLeft = clampedScrollLeft;
    }
    renderAll();
  });
});

/**
 * Consolidated reactive dependency bundle for triggering canvas re-renders.
 * Encapsulates all scale, layout, configuration, and simulation state changes
 * into a single computed property to eliminate duplicate render churn.
 */
const renderDependencies = computed(() => ({
  pxPerCycle: pxPerCycle.value,
  cycleTimeMs: cycleTimeMs.value,
  totalCycles: totalCycles.value,
  totalWidth: totalWidth.value,
  clientConfigs: activeConfig.value.client_configs,
  plannedExecutions: plannedExecutionsAnalyzeMode.value,
  logSummary: logSummaryAnalyzeMode.value,
  logRangeData: logRangeDataAnalyzeMode.value,
}));

/**
 * Automatically re-render header and background grid whenever any layout or content dependency changes.
 */
watch(renderDependencies, () => {
  nextTick(() => {
    renderAll();
  });
}, { deep: true });

// -----------------------------------------------------------------------------
// Lifecycle Hooks & Observers

/**
 * Handle viewport resize or theme attribute changes by updating theme cache and re-rendering.
 */
const onLayoutOrThemeChange = () => {
  updateThemeStyles();
  renderAll();
};

const themeMutationObserver = new MutationObserver(onLayoutOrThemeChange);

onMounted(() => {
  updateThemeStyles();
  nextTick(() => renderAll());

  window.addEventListener('resize', onLayoutOrThemeChange);
  document.documentElement && themeMutationObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['class', 'style', 'data-theme']
  });
});

onUnmounted(() => {
  if (renderRafId !== null) {
    window.cancelAnimationFrame(renderRafId);
    renderRafId = null;
  }
  window.removeEventListener('resize', onLayoutOrThemeChange);
  themeMutationObserver.disconnect();
});

// -----------------------------------------------------------------------------
// Expose & Exports

defineExpose({
  headerScrollEl,
  contentScrollEl
});
</script>

<!-- ========================================================================== -->
<!-- Template Section                                                           -->
<!-- ========================================================================== -->
<template>
  <main class="timeline-pane" :key="activeConfig.sessionId">
    <!-- Time Header Section (Cycle and time markers in Canvas overlay) -->
    <div class="timeline-header sb-hide-all sb-pad-v" ref="headerScrollEl" @scroll="onScroll">
      <div class="header-content" :style="{ width: totalWidth + 'px', height: '100%' }">
        <canvas ref="headerCanvasEl" class="canvas-layer"></canvas>
      </div>
    </div>

    <!-- Scrollable Content Section (Background Grid & Process Timeline) -->
    <div class="scroll-area timeline-scroll sb-hide-h" ref="contentScrollEl" @scroll="onScroll">
      <div class="timeline-content" :style="{ width: totalWidth + 'px', height: totalContentHeight + 'px' }">
        <canvas ref="contentCanvasEl" class="canvas-layer"></canvas>
      </div>
    </div>
  </main>
</template>

<!-- ========================================================================== -->
<!-- Style Section                                                              -->
<!-- ========================================================================== -->
<style scoped>
/* -----------------------------------------------------------------------------
 * Layout & Containers
 * ----------------------------------------------------------------------------- */

.timeline-pane {
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

.timeline-scroll {
  width: 100%;
  height: 100%;
  overflow-x: scroll;
  overflow-y: scroll;
}

.timeline-content {
  position: relative;
  min-height: 100%;
}

/* -----------------------------------------------------------------------------
 * Canvas & Visual Components
 * ----------------------------------------------------------------------------- */

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
