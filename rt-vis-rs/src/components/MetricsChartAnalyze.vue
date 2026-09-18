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
import { getSimulationCycles, findActualCycleForTime } from '../utils/cycle';
import {
  computePlannedConcurrencySteps,
  findActualConcurrencyForTime,
  findPlannedConcurrencyForTime,
  type ConcurrencyStep
} from '../utils/metrics';
import { formatDelta, formatDeltaCount } from '../utils/format';

// -----------------------------------------------------------------------------
// Global State & Composables

const {
  activeConfig,
  plannedExecutionsAnalyzeMode,
  plannedMetricsAnalyzeMode,
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
// Types & Interfaces

/**
 * Hover payload data for metrics chart tooltip and crosshair tracking.
 */
interface MetricHoverData {
  timeMs: number;
  cycle: number;
  actualConcurrency: number;
  plannedConcurrency: number | null;
  jitterMs: number | null;
}

// -----------------------------------------------------------------------------
// Constants & Layout

const ROW_HEIGHT = 70; // Height of each metric chart row in pixels (matching Create Mode)
const METRIC_ROWS = 2; // Total metric chart rows (1: Concurrent Processes, 2: Cycle Jitter)
const CHART_TOP_MARGIN = 10; // Top margin to prevent charts from touching top row border (px)
const CHART_BOTTOM_MARGIN = 10; // Bottom margin to prevent charts and baselines from touching bottom row border (px)
const CHART_STROKE_WIDTH = 2; // Stroke line width for metrics chart lines (px)

// -----------------------------------------------------------------------------
// Local State & Computed

const headerScrollEl = ref<HTMLElement | null>(null);
const contentScrollEl = ref<HTMLElement | null>(null);
const headerCanvasEl = ref<HTMLCanvasElement | null>(null);
const contentCanvasEl = ref<HTMLCanvasElement | null>(null);

const cachedThemeStyles = ref<ThemeStyles | null>(null);

// Cached steps for planned concurrency across visible range
let visiblePlannedSteps: ConcurrencyStep[] = [];

// Interactive hover and tooltip states
const hoveredMetric = ref<MetricHoverData | null>(null);
const tooltipPos = ref<{
  mouseX: number;
  mouseY: number;
  scrollLeft: number;
  scrollTop: number;
} | null>(null);

const hasHoverTarget = computed(() => hoveredMetric.value !== null);

/**
 * Compute floating tooltip placement with boundary-aware smart clamping.
 * Uses anchor positioning with CSS transform translation (-100%) when flipped,
 * ensuring seamless edge-aligned gap (12px) regardless of dynamic tooltip dimensions.
 * Applies vertical safety clamping tailored for compact 2-row layout (140px)
 * to completely eliminate top and bottom clipping.
 */
const tooltipStyle = computed(() => {
  if (!tooltipPos.value || !contentScrollEl.value) return { display: 'none' };

  const { mouseX, mouseY, scrollLeft, scrollTop } = tooltipPos.value;
  const containerWidth = contentScrollEl.value.clientWidth;
  const containerHeight = contentScrollEl.value.clientHeight;

  // Estimated bounding box thresholds for boundary overflow detection
  // Aligned with actual rendered tooltip dimensions (~80-82px) to preserve design parity with TimelineView
  const ESTIMATED_MAX_WIDTH = 300;
  const ESTIMATED_MAX_HEIGHT = 84;
  const MARGIN_Y = 6;
  const offset = 12;

  // Determine flip state for right container edge
  const flipX = mouseX + offset + ESTIMATED_MAX_WIDTH > containerWidth;
  // Determine vertical direction based on container center (row boundary: Row 1 vs Row 2)
  const isLowerHalf = mouseY >= containerHeight / 2;

  // Horizontal anchor point: offset cursor by 12px in either positive or negative direction
  const left = mouseX + scrollLeft + (flipX ? -offset : offset);

  // Vertical anchor point with safety boundary clamping:
  // When cursor is in lower half (Row 2), flip upward: ensure anchor top >= ESTIMATED_MAX_HEIGHT + MARGIN_Y
  // so that translateY(-100%) guarantees top edge maintains at least MARGIN_Y margin from container top.
  // When cursor is in upper half (Row 1), position downward: ensure anchor top <= containerHeight - ESTIMATED_MAX_HEIGHT - MARGIN_Y
  // so that bottom edge maintains at least MARGIN_Y margin from container bottom.
  const top = isLowerHalf
    ? Math.max(ESTIMATED_MAX_HEIGHT + MARGIN_Y, mouseY - offset) + scrollTop
    : Math.min(containerHeight - ESTIMATED_MAX_HEIGHT - MARGIN_Y, mouseY + offset) + scrollTop;

  // Shift by 100% of element's actual rendered dimensions when flipped
  const translateX = flipX ? '-100%' : '0%';
  const translateY = isLowerHalf ? '-100%' : '0%';

  return {
    left: `${left}px`,
    top: `${top}px`,
    transform: `translate(${translateX}, ${translateY})`
  };
});

/**
 * Compute vertical crosshair placement along physical time axis.
 */
const crosshairStyle = computed(() => {
  if (!tooltipPos.value) return { display: 'none' };
  const { mouseX, scrollLeft } = tooltipPos.value;
  return {
    left: `${mouseX + scrollLeft}px`
  };
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
 * Render Row 1: Planned concurrency area chart (translucent filled step polygon).
 */
const renderMetricsRow1Planned = (
  ctx: CanvasRenderingContext2D,
  scrollLeft: number,
  width: number,
  maxConcurrency: number,
  baseY: number,
  styles: ThemeStyles
) => {
  const plannedExecs = plannedExecutionsAnalyzeMode.value;
  const actualCycles = logRangeDataAnalyzeMode.value?.actual_cycles;
  if (!plannedExecs || plannedExecs.length === 0 || !actualCycles || actualCycles.length === 0) {
    visiblePlannedSteps = [];
    return;
  }

  const curCycleTime = cycleTimeMs.value;
  const templateCycles = getSimulationCycles(activeConfig.value.client_configs);

  const marginMs = curCycleTime * 4;
  const startMs = Math.max(0, scrollLeft / pxPerMs.value - marginMs);
  const endMs = (scrollLeft + width) / pxPerMs.value + marginMs;

  const steps = computePlannedConcurrencySteps(plannedExecs, actualCycles, templateCycles, startMs, endMs);
  visiblePlannedSteps = steps;
  if (steps.length === 0) return;

  ctx.save();
  {
    ctx.fillStyle = styles.primaryColor;
    ctx.globalAlpha = 0.4;
    ctx.beginPath();

    const firstX = Math.floor(steps[0].timeMs * pxPerMs.value - scrollLeft);
    ctx.moveTo(firstX, baseY);

    let prevY = baseY;
    steps.forEach(step => {
      const x = Math.floor(step.timeMs * pxPerMs.value - scrollLeft);
      const y = baseY - (step.count / maxConcurrency) * (ROW_HEIGHT - CHART_TOP_MARGIN);

      // Horizontal segment: maintain previous concurrency count up to current event time
      ctx.lineTo(x, prevY);
      // Vertical step edge: adjust to new concurrency count at current event time
      ctx.lineTo(x, y);

      prevY = y;
    });

    const lastX = Math.floor(steps[steps.length - 1].timeMs * pxPerMs.value - scrollLeft);
    ctx.lineTo(lastX, baseY);
    ctx.closePath();
    ctx.fill();
  }
  ctx.restore();
};

/**
 * Render Row 1: Actual concurrency line chart (solid error-colored step line).
 * Clamps bottom Y coordinate by half stroke width to keep the line fully within the row boundary.
 */
const renderMetricsRow1Actual = (
  ctx: CanvasRenderingContext2D,
  scrollLeft: number,
  maxConcurrency: number,
  baseY: number,
  styles: ThemeStyles
) => {
  const actuals = logRangeDataAnalyzeMode.value?.actual_metrics;
  if (!actuals || actuals.length === 0) return;

  const maxLineY = baseY - CHART_STROKE_WIDTH / 2;

  ctx.save();
  {
    ctx.strokeStyle = styles.accentColor;
    ctx.lineWidth = CHART_STROKE_WIDTH;
    ctx.beginPath();

    actuals.forEach((pt, idx) => {
      const x = Math.floor(pt.time_ms * pxPerMs.value - scrollLeft);
      const rawY = baseY - (pt.running_count / maxConcurrency) * (ROW_HEIGHT - CHART_TOP_MARGIN);
      const y = Math.min(rawY, maxLineY);

      if (idx === 0) {
        ctx.moveTo(x, y);
      } else {
        const prevPt = actuals[idx - 1];
        const prevRawY = baseY - (prevPt.running_count / maxConcurrency) * (ROW_HEIGHT - CHART_TOP_MARGIN);
        const prevY = Math.min(prevRawY, maxLineY);
        // Horizontal segment followed by vertical step edge
        ctx.lineTo(x, prevY);
        ctx.lineTo(x, y);
      }
    });
    ctx.stroke();
  }
  ctx.restore();
};

/**
 * Render Row 1: Concurrent process count metrics (Planned translucent area + Actual error-colored step chart).
 * Uses dynamic scaling based on log max_concurrency metadata and planned metrics.
 */
const renderMetricsRow1 = (
  ctx: CanvasRenderingContext2D,
  scrollLeft: number,
  width: number,
  styles: ThemeStyles
) => {
  if (!logSummaryAnalyzeMode.value || totalCycles.value <= 0 || cycleTimeMs.value <= 0) return;

  const row1BaseY = ROW_HEIGHT;

  const planned = plannedMetricsAnalyzeMode.value;
  const plannedMax = planned && planned.length > 0 ? Math.max(...planned.map(p => p.running_count)) : 1;
  const logMax = logSummaryAnalyzeMode.value.max_concurrency || 1;
  const maxConcurrency = Math.max(1, plannedMax, logMax);

  renderMetricsRow1Planned(ctx, scrollLeft, width, maxConcurrency, row1BaseY, styles);
  renderMetricsRow1Actual(ctx, scrollLeft, maxConcurrency, row1BaseY, styles);
};

/**
 * Render Row 2: Server Cycle Jitter 0ms baseline (dashed reference line).
 */
const renderMetricsRow2JitterBaseline = (
  ctx: CanvasRenderingContext2D,
  row2ZeroY: number,
  width: number,
  styles: ThemeStyles
) => {
  ctx.save();
  {
    ctx.strokeStyle = styles.textDimColor;
    ctx.lineWidth = 1;
    ctx.setLineDash([3, 3]);
    ctx.beginPath();
    ctx.moveTo(0, row2ZeroY - 0.5);
    ctx.lineTo(width, row2ZeroY - 0.5);
    ctx.stroke();
  }
  ctx.restore();
};

/**
 * Render Row 2: Server Cycle Jitter line chart for actual cycle start jitters.
 */
const renderMetricsRow2Jitter = (
  ctx: CanvasRenderingContext2D,
  scrollLeft: number,
  row2ZeroY: number,
  jitterSpan: number,
  availableHeight: number,
  styles: ThemeStyles
) => {
  const actualCycles = logRangeDataAnalyzeMode.value?.actual_cycles;
  if (!actualCycles || actualCycles.length === 0) return;

  ctx.save();
  {
    ctx.strokeStyle = styles.accentColor;
    ctx.lineWidth = CHART_STROKE_WIDTH;
    ctx.beginPath();

    actualCycles.forEach((ac, idx) => {
      const x = Math.floor(ac.start_ms * pxPerMs.value - scrollLeft);
      const jitterY = Math.floor(row2ZeroY - (ac.start_jitter_ms / jitterSpan) * availableHeight);

      if (idx === 0) {
        ctx.moveTo(x, jitterY);
      } else {
        ctx.lineTo(x, jitterY);
      }
    });
    ctx.stroke();
  }
  ctx.restore();
};

/**
 * Render Row 2: Server Cycle Jitter line chart with dynamic 0ms baseline positioning.
 * Applies symmetrical top/bottom margins (10px) to guarantee breathing room for maximum jitter
 * and prevent line clipping against row borders.
 */
const renderMetricsRow2 = (
  ctx: CanvasRenderingContext2D,
  scrollLeft: number,
  width: number,
  styles: ThemeStyles
) => {
  const summary = logSummaryAnalyzeMode.value;
  if (!summary) return;

  const row2TopY = ROW_HEIGHT + CHART_TOP_MARGIN;
  const row2BottomY = ROW_HEIGHT * 2 - CHART_BOTTOM_MARGIN;
  const availableHeight = row2BottomY - row2TopY;

  // Dynamic baseline calculations using min/max start jitter metadata
  // Prevents negative jitter space from wasting 50% of the canvas when delays are predominantly positive.
  const minJitter = Math.min(0, summary.min_start_jitter_ms ?? 0);
  const maxJitter = Math.max(0.1, summary.max_start_jitter_ms ?? 0.1);
  const jitterSpan = Math.max(0.1, maxJitter - minJitter);

  // Position 0ms baseline dynamically based on min/max ratio.
  const zeroRatio = (0 - minJitter) / jitterSpan;
  const row2ZeroY = Math.floor(row2BottomY - zeroRatio * availableHeight);

  renderMetricsRow2JitterBaseline(ctx, row2ZeroY, width, styles);
  renderMetricsRow2Jitter(ctx, scrollLeft, row2ZeroY, jitterSpan, availableHeight, styles);
};

/**
 * Render horizontal row separator borders between metric charts.
 */
const renderRowBorders = (
  ctx: CanvasRenderingContext2D,
  width: number,
  styles: ThemeStyles
) => {
  ctx.save();
  {
    ctx.strokeStyle = styles.borderColor;
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let r = 1; r <= METRIC_ROWS; r++) {
      const y = Math.floor(r * ROW_HEIGHT) - 0.5;
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
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
 * Render metrics content background grid, row borders, concurrency chart, and cycle jitter line.
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
  const { width, height, ctx } = prepareCanvas(contentCanvasEl.value, container);
  if (!ctx) return;

  // Pin canvas overlay dynamically to current scroll viewport to avoid clipping or blank bleeding
  contentCanvasEl.value.style.transform = `translate(${scrollLeft}px, 0px)`;

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

  // Render actual cycle start lines overlay (double grid structure matching TimelineView)
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

  renderRowBorders(ctx, width, styles);

  renderMetricsRow1(ctx, scrollLeft, width, styles);
  renderMetricsRow2(ctx, scrollLeft, width, styles);

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
  // Clear tooltip and crosshair when actively scrolling to prevent stale floating overlays
  hoveredMetric.value = null;

  if (renderRafId === null) {
    renderRafId = window.requestAnimationFrame(() => {
      renderAll();
      renderRafId = null;
    });
  }
  emit('scroll', e);
};

/**
 * Handle mouse move over metrics content for crosshair and tooltip tracking.
 */
const onMouseMove = (e: MouseEvent) => {
  if (!contentScrollEl.value || pxPerMs.value <= 0) return;

  const rect = contentScrollEl.value.getBoundingClientRect();
  const mouseX = e.clientX - rect.left;
  const mouseY = e.clientY - rect.top;
  const scrollLeft = contentScrollEl.value.scrollLeft;
  const scrollTop = contentScrollEl.value.scrollTop;

  // Calculate physical time on timeline
  const timeMs = (mouseX + scrollLeft) / pxPerMs.value;
  if (timeMs < 0) {
    hoveredMetric.value = null;
    return;
  }

  const actualCycles = logRangeDataAnalyzeMode.value?.actual_cycles;
  const actualMetrics = logRangeDataAnalyzeMode.value?.actual_metrics;

  const matchedCycle = actualCycles ? findActualCycleForTime(actualCycles, timeMs) : null;
  if (!matchedCycle) {
    hoveredMetric.value = null;
    return;
  }

  const actualConcurrency = actualMetrics ? findActualConcurrencyForTime(actualMetrics, timeMs) : 0;
  const plannedConcurrency = findPlannedConcurrencyForTime(visiblePlannedSteps, timeMs);

  hoveredMetric.value = {
    timeMs,
    cycle: matchedCycle.cycle,
    actualConcurrency,
    plannedConcurrency,
    jitterMs: matchedCycle.start_jitter_ms
  };

  tooltipPos.value = { mouseX, mouseY, scrollLeft, scrollTop };
};

/**
 * Handle mouse leaving metrics content area.
 */
const onMouseLeave = () => {
  hoveredMetric.value = null;
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
  plannedMetrics: plannedMetricsAnalyzeMode.value,
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
  <main class="metrics-pane" :key="activeConfig.sessionId">
    <!-- Time Header Section (Cycle and time markers in Canvas overlay) -->
    <div class="timeline-header sb-hide-all sb-pad-v" ref="headerScrollEl" @scroll="onScroll">
      <div class="header-content" :style="{ width: totalWidth + 'px', height: '100%' }">
        <canvas ref="headerCanvasEl" class="canvas-layer"></canvas>
      </div>
    </div>

    <!-- Scrollable Content Section (Background Grid & Metrics Viewer) -->
    <div class="scroll-area metrics-scroll sb-hide-v sb-pad-v" :class="{ 'has-hover': hasHoverTarget }"
      ref="contentScrollEl" @scroll="onScroll" @mousemove="onMouseMove" @mouseleave="onMouseLeave">
      <div class="metrics-content" :style="{ width: totalWidth + 'px', height: (ROW_HEIGHT * 2) + 'px' }">
        <canvas ref="contentCanvasEl" class="canvas-layer"></canvas>

        <!-- Vertical Crosshair Line -->
        <div v-if="hoveredMetric" class="metrics-crosshair" :style="crosshairStyle"></div>

        <!-- Floating Tooltip Overlay -->
        <div v-if="hoveredMetric" class="metrics-tooltip" :style="tooltipStyle">
          <div class="tooltip-card">
            <div class="tooltip-header">
              <span class="tooltip-cycle">Cycle #{{ hoveredMetric.cycle }}</span>
              <span class="tooltip-time tooltip-mono">{{ hoveredMetric.timeMs.toFixed(2) }} ms</span>
            </div>
            <div class="tooltip-metric-body">
              <div class="tooltip-metric-row">
                <span class="tooltip-metric-label">Concurrency:</span>
                <span class="tooltip-metric-value tooltip-mono">
                  Actual {{ hoveredMetric.actualConcurrency }}
                  <template v-if="hoveredMetric.plannedConcurrency !== null">
                    <span class="tooltip-metric-sub">
                      (Plan {{ hoveredMetric.plannedConcurrency }} / Δ{{
                        formatDeltaCount(hoveredMetric.actualConcurrency - hoveredMetric.plannedConcurrency) }})
                    </span>
                  </template>
                </span>
              </div>
              <div class="tooltip-metric-row">
                <span class="tooltip-metric-label">Cycle Jitter:</span>
                <span class="tooltip-metric-value tooltip-mono">
                  {{ hoveredMetric.jitterMs !== null ? formatDelta(hoveredMetric.jitterMs) : 'N/A' }}
                </span>
              </div>
            </div>
          </div>
        </div>
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

/* --- Hover State --- */
.metrics-scroll.has-hover .canvas-layer {
  cursor: default;
}

/* -----------------------------------------------------------------------------
 * Crosshair & Tooltip Overlay Components
 * ----------------------------------------------------------------------------- */

/* --- Crosshair Line --- */
.metrics-crosshair {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 0;
  border-left: 1px dashed var(--rt-color-text, #ffffff);
  opacity: 0.5;
  z-index: 20;
  pointer-events: none;
}

/* --- Base Tooltip --- */
.metrics-tooltip {
  position: absolute;
  top: 0;
  left: 0;
  z-index: 30;
  width: max-content;
  max-width: 320px;
  padding: 8px 12px;
  background-color: var(--rt-color-surface-elevated, #24272e);
  border: var(--rt-border-main);
  border-radius: var(--rt-radius-m, 6px);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.45);
  color: var(--rt-color-text);
  font-size: var(--rt-font-s, 12px);
  line-height: 1.4;
  user-select: none;
  pointer-events: none;
}

.tooltip-card {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.tooltip-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 16px;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--rt-color-border);
}

.tooltip-cycle {
  color: var(--rt-color-text);
  font-size: var(--rt-font-s, 12px);
  font-weight: 700;
}

.tooltip-time {
  color: var(--rt-color-text-dim);
  font-size: var(--rt-font-xs, 11px);
}

.tooltip-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-variant-numeric: tabular-nums;
}

/* --- Metric Tooltip Rows (tooltip-metric-*) --- */
.tooltip-metric-body {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.tooltip-metric-row {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  font-size: var(--rt-font-xs, 11px);
}

.tooltip-metric-label {
  flex-shrink: 0;
  color: var(--rt-color-text-dim);
}

.tooltip-metric-value {
  color: var(--rt-color-text);
}

.tooltip-metric-sub {
  margin-left: 4px;
  color: var(--rt-color-text-dim);
}
</style>
