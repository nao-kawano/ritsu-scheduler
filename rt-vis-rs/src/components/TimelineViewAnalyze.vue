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
import { getSimulationCycles, groupPlansByAnchorCycle, filterVisibleActualCycles, findActualCycleForTime } from '../utils/cycle';
import { formatDelta } from '../utils/format';
import type { ActualExecution, ActualInstantEvent, ActualCycle } from '../types/analyze';

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
// Types & Interfaces

/**
 * Cached hit item for rendered actual execution bar.
 */
interface HitBarItem {
  cid: number;
  clientName: string;
  x: number;
  y: number;
  width: number;
  height: number;
  actual: ActualExecution;
  planDurationMs: number | null;
  planStartMs: number | null;
}

/**
 * Cached hit item for rendered instant event diamond marker.
 */
interface HitEventItem {
  cid: number;
  clientName: string;
  x: number;
  y: number;
  event: ActualInstantEvent;
  cycle: number;
}

// -----------------------------------------------------------------------------
// Constants & Layout

const ROW_HEIGHT = 70;         // Fixed height of each process row in pixels (matching Create Mode)
const PLAN_RECT_HEIGHT = 46;  // Outer planned execution box height (matching Create Mode bar height)
const ACTUAL_RECT_HEIGHT = 28; // Inner solid actual execution bar height
const EVENT_DIAMOND_RADIUS_PX = 8; // Radius of instant event diamond marker in pixels
const HIT_RADIUS_PX = 8;       // Hit detection radius for instant event markers in pixels

// -----------------------------------------------------------------------------
// Local State & Computed

const headerScrollEl = ref<HTMLElement | null>(null);
const contentScrollEl = ref<HTMLElement | null>(null);
const headerCanvasEl = ref<HTMLCanvasElement | null>(null);
const contentCanvasEl = ref<HTMLCanvasElement | null>(null);

const cachedThemeStyles = ref<ThemeStyles | null>(null);

// Primary caches for visible elements populated during canvas render
let visibleHitBars: HitBarItem[] = [];
let visibleHitEvents: HitEventItem[] = [];

// Interactive hover and tooltip states
const hoveredBar = ref<HitBarItem | null>(null);
const hoveredEvents = ref<HitEventItem[] | null>(null);
const tooltipPos = ref<{
  mouseX: number;
  mouseY: number;
  scrollLeft: number;
  scrollTop: number;
} | null>(null);

const hasHoverTarget = computed(() => {
  return !!hoveredBar.value || (hoveredEvents.value !== null && hoveredEvents.value.length > 0);
});

/**
 * Map Client ID to process row index for alignment.
 */
const cidToRowIndex = computed(() => {
  const map = new Map<number, number>();
  activeConfig.value.client_configs.forEach((c, idx) => map.set(c.data.client_id, idx));
  return map;
});

/**
 * Map Client ID to process display name.
 */
const cidToClientName = computed(() => {
  const map = new Map<number, string>();
  activeConfig.value.client_configs.forEach(c => map.set(c.data.client_id, c.data.display_name));
  return map;
});

/**
 * Map Cycle number to ActualCycle object for O(1) lookup.
 */
const actualCyclesMap = computed(() => {
  const map = new Map<number, ActualCycle>();
  const cycles = logRangeDataAnalyzeMode.value?.actual_cycles;
  if (cycles) {
    cycles.forEach(c => map.set(c.cycle, c));
  }
  return map;
});

/**
 * Number of cycles in one full template simulation period.
 */
const templateCycles = computed(() => {
  return getSimulationCycles(activeConfig.value.client_configs);
});

/**
 * Planned executions grouped by anchor cycle phase for fast lookup.
 */
const plansByAnchorCycle = computed(() => {
  const plans = plannedExecutionsAnalyzeMode.value;
  return plans && plans.length > 0 ? groupPlansByAnchorCycle(plans) : null;
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

/**
 * Compute floating tooltip placement with boundary-aware smart clamping.
 * Uses anchor positioning with CSS transform translation (-100%) when flipped,
 * ensuring seamless edge-aligned gap (14px) regardless of dynamic tooltip dimensions.
 * Evaluates available container space and applies safety clamping to eliminate
 * top/bottom and left/right clipping even on narrow or resized windows.
 */
const tooltipStyle = computed(() => {
  if (!tooltipPos.value || !contentScrollEl.value) return { display: 'none' };

  const { mouseX, mouseY, scrollLeft, scrollTop } = tooltipPos.value;
  const containerWidth = contentScrollEl.value.clientWidth;
  const containerHeight = contentScrollEl.value.clientHeight;

  // Estimated dimensions for boundary overflow detection
  // Dynamically estimate height based on whether hovering multiple instant events or an actual execution bar
  const ESTIMATED_MAX_WIDTH = 340;
  const events = hoveredEvents.value;
  const estimatedHeight = events && events.length > 0
    ? Math.min(250, 60 + Math.min(events.length, 4) * 44 + (events.length > 4 ? 20 : 0))
    : 150;

  const MARGIN_X = 8;
  const MARGIN_Y = 6;
  const offset = 14;

  // Horizontal placement: check available space to the right vs left
  const hasSpaceRight = mouseX + offset + ESTIMATED_MAX_WIDTH + MARGIN_X <= containerWidth;
  const hasSpaceLeft = mouseX - offset - ESTIMATED_MAX_WIDTH >= MARGIN_X;
  const isRightHalf = mouseX >= containerWidth / 2;
  const flipX = !hasSpaceRight && (hasSpaceLeft || isRightHalf);

  // Vertical placement: check available space below vs above
  const hasSpaceBelow = mouseY + offset + estimatedHeight + MARGIN_Y <= containerHeight;
  const hasSpaceAbove = mouseY - offset - estimatedHeight >= MARGIN_Y;
  const isLowerHalf = mouseY >= containerHeight / 2;
  const flipY = !hasSpaceBelow && (hasSpaceAbove || isLowerHalf);

  // Horizontal anchor point with safety boundary clamping
  const anchorX = flipX
    ? Math.max(ESTIMATED_MAX_WIDTH + MARGIN_X, mouseX - offset)
    : Math.min(Math.max(MARGIN_X, containerWidth - ESTIMATED_MAX_WIDTH - MARGIN_X), mouseX + offset);

  // Vertical anchor point with safety boundary clamping
  const anchorY = flipY
    ? Math.max(estimatedHeight + MARGIN_Y, mouseY - offset)
    : Math.min(Math.max(MARGIN_Y, containerHeight - estimatedHeight - MARGIN_Y), mouseY + offset);

  const left = anchorX + scrollLeft;
  const top = anchorY + scrollTop;

  // Shift by 100% of element's actual rendered dimensions when flipped
  const translateX = flipX ? '-100%' : '0%';
  const translateY = flipY ? '-100%' : '0%';

  return {
    left: `${left}px`,
    top: `${top}px`,
    transform: `translate(${translateX}, ${translateY})`
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
  if (!plansByAnchorCycle.value || templateCycles.value <= 0) return;

  // Calculate culled time range with 2-cycle padding margin in physical milliseconds
  const marginMs = cycleTimeMs.value * 2;
  const startMs = Math.max(0, scrollLeft / pxPerMs.value - marginMs);
  const endMs = (scrollLeft + width) / pxPerMs.value + marginMs;

  const actualCycles = logRangeDataAnalyzeMode.value?.actual_cycles;
  if (!actualCycles || actualCycles.length === 0) return;

  // Filter visible actual cycles based on physical time bounds
  const visibleCycles = filterVisibleActualCycles(actualCycles, startMs, endMs);
  if (visibleCycles.length === 0) return;

  // Phase alignment pattern: render exact matching template phase for each visible cycle
  visibleCycles.forEach(ac => {
    const c = ac.cycle;
    const templateCycle = c % templateCycles.value;
    const matchingPlans = plansByAnchorCycle.value!.get(templateCycle);
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
          if (plan.status === 'overrun') barColor = styles.statusOverrunColor;

          // Translucent primary fill matching Create Mode theme accent
          ctx.globalAlpha = 0.4;
          ctx.fillStyle = barColor;
          ctx.beginPath();
          if (typeof ctx.roundRect === 'function') {
            ctx.roundRect(x, y, barWidth, PLAN_RECT_HEIGHT, 6);
          } else {
            ctx.rect(x, y, barWidth, PLAN_RECT_HEIGHT);
          }
          if (plan.status !== 'skip') {
            ctx.fill();
          }

          // Subtle dashed border outline
          ctx.strokeStyle = barColor;
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

    const clientName = cidToClientName.value.get(cid) || `Client ${cid}`;

    actuals.forEach(actual => {
      // Guard against skipped executions (sub-millisecond or cut-off executions with duration_ms == 0 are rendered with minimum width)
      if (actual.status === 'skip' || actual.duration_ms < 0) return;

      const x = Math.floor(actual.start_ms * pxPerMs.value - scrollLeft);
      const barWidth = Math.max(4, Math.floor(actual.duration_ms * pxPerMs.value));
      const y = Math.floor(r * ROW_HEIGHT - scrollTop + (ROW_HEIGHT - ACTUAL_RECT_HEIGHT) / 2);

      if (y + ACTUAL_RECT_HEIGHT >= 0 && y <= height && x + barWidth >= 0 && x <= width) {
        // Resolve corresponding planned duration and planned start time for tooltip comparison
        let planDurationMs: number | null = null;
        let planStartMs: number | null = null;
        if (plansByAnchorCycle.value && templateCycles.value > 0) {
          const matchingPlans = plansByAnchorCycle.value.get(actual.cycle % templateCycles.value);
          const plan = matchingPlans?.find(p => p.cid === cid);
          if (plan) {
            planDurationMs = plan.duration_ms;
            const actualCycle = actualCyclesMap.value.get(actual.cycle);
            const cycleStartMs = actualCycle ? actualCycle.start_ms : (actual.cycle * cycleTimeMs.value);
            planStartMs = cycleStartMs + (plan.anchor_offset_ms || 0);
          }
        }

        // Cache hit item in container viewport coordinates
        visibleHitBars.push({
          cid,
          clientName,
          x,
          y,
          width: barWidth,
          height: ACTUAL_RECT_HEIGHT,
          actual,
          planDurationMs,
          planStartMs
        });

        ctx.save();
        {
          // Status color fill: use dedicated status semantic tokens matching tooltip badges
          let barColor = styles.statusNormalColor;
          if (actual.status === 'overrun') barColor = styles.statusOverrunColor;

          // Apply translucency for incomplete executions cut off at log end
          if (actual.log_line_no_end === null) {
            ctx.globalAlpha = 0.55;
          }

          ctx.fillStyle = barColor;
          ctx.beginPath();
          if (typeof ctx.roundRect === 'function') {
            ctx.roundRect(x, y, barWidth, ACTUAL_RECT_HEIGHT, 4);
          } else {
            ctx.rect(x, y, barWidth, ACTUAL_RECT_HEIGHT);
          }
          ctx.fill();

          // Subtle highlight border
          ctx.strokeStyle = actual.log_line_no_end === null ? 'rgba(255, 255, 255, 0.3)' : 'rgba(255, 255, 255, 0.6)';
          ctx.lineWidth = 2;
          ctx.stroke();
        }
        ctx.restore();
      }
    });
  });
};

/**
 * Render point-in-time execution events (diamond markers) into canvas context.
 */
const renderInstantEvents = (
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
  if (!rangeData || !rangeData.actual_instant_events_by_cid) return;

  const actualCycles = rangeData.actual_cycles || [];

  rangeData.actual_instant_events_by_cid.forEach(([cid, events]) => {
    const r = cidToRowIndex.get(cid);
    if (r === undefined) return;

    const clientName = cidToClientName.value.get(cid) || `Client ${cid}`;
    const yCenter = Math.floor(r * ROW_HEIGHT - scrollTop + ROW_HEIGHT / 2);
    // Early vertical viewport culling for the process row
    if (yCenter + EVENT_DIAMOND_RADIUS_PX < 0 || yCenter - EVENT_DIAMOND_RADIUS_PX > height) return;

    events.forEach(event => {
      const x = Math.floor(event.time_ms * pxPerMs.value - scrollLeft);
      // Horizontal viewport culling
      if (x + EVENT_DIAMOND_RADIUS_PX < 0 || x - EVENT_DIAMOND_RADIUS_PX > width) return;

      // Reverse lookup cycle number and cache hit item for tooltip hit testing
      const cycle = findActualCycleForTime(actualCycles, event.time_ms)?.cycle ?? 0;
      visibleHitEvents.push({
        cid,
        clientName,
        x,
        y: yCenter,
        event,
        cycle
      });

      ctx.save();
      {
        let fillColor = styles.eventReadyColor;
        switch (event.event_type) {
          case 'ready':
            fillColor = styles.eventReadyColor;
            break;
          case 'exit':
            fillColor = styles.eventExitColor;
            break;
          case 'overrun':
            fillColor = styles.eventOverrunColor;
            break;
          case 'error':
            fillColor = styles.eventErrorColor;
            break;
          case 'skip':
            fillColor = styles.eventSkipColor;
            break;
          case 'late':
            fillColor = styles.eventLateColor;
            break;
          case 'retransmit':
            fillColor = styles.eventRetransmitColor;
            break;
        }

        // Diamond marker path centered at (x, yCenter)
        ctx.fillStyle = fillColor;
        ctx.beginPath();
        ctx.moveTo(x, yCenter - EVENT_DIAMOND_RADIUS_PX);
        ctx.lineTo(x + EVENT_DIAMOND_RADIUS_PX, yCenter);
        ctx.lineTo(x, yCenter + EVENT_DIAMOND_RADIUS_PX);
        ctx.lineTo(x - EVENT_DIAMOND_RADIUS_PX, yCenter);
        ctx.closePath();
        ctx.fill();

        // Subtle outline stroke for high contrast against bars and background
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.85)';
        ctx.lineWidth = 1;
        ctx.stroke();
      }
      ctx.restore();
    });
  });
};

/**
 * Render sticky time header canvas using shared canvas rendering composable.
 */
const renderHeader = (styles: ThemeStyles) => {
  if (!headerCanvasEl.value || !headerScrollEl.value) return;

  const { width, height, ctx } = prepareCanvas(headerCanvasEl.value, headerScrollEl.value);
  if (!ctx) return;

  // ALWAYS use contentScrollEl's scrollLeft as master to guarantee 100% pixel sync across panes
  const scrollLeft = contentScrollEl.value ? contentScrollEl.value.scrollLeft : headerScrollEl.value.scrollLeft;

  // Pin canvas overlay dynamically to current scroll viewport to avoid clipping or blank bleeding
  headerCanvasEl.value.style.transform = `translate(${scrollLeft}px, 0px)`;

  renderTimelineHeader(ctx, {
    scrollLeft,
    width,
    height,
    totalCycles: totalCycles.value,
    cycleTimeMs: cycleTimeMs.value,
    majorPx: gridInfo.value.majorPx,
    styles,
    actualCycles: logRangeDataAnalyzeMode.value?.actual_cycles,
    pxPerMs: pxPerMs.value
  });
};

/**
 * Render timeline content background grid, process row borders, plan boxes, and actual bars.
 */
const renderContent = (styles: ThemeStyles) => {
  if (!contentCanvasEl.value || !contentScrollEl.value) return;

  // Clear hit testing primary caches before collecting visible items
  visibleHitBars = [];
  visibleHitEvents = [];

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
  renderInstantEvents(ctx, scrollLeft, scrollTop, width, height, styles, cidToRowIndex.value);
};

/**
 * Request asynchronous range fetch for visible physical time bounds if needed.
 * Adheres to optimistic rendering architecture: renders immediately using cached local data
 * for smooth frame rates while requesting missing range data in the background.
 */
const requestVisibleLogRange = () => {
  if (!contentScrollEl.value || !logSummaryAnalyzeMode.value || cycleTimeMs.value <= 0) return;

  const { scrollLeft, clientWidth } = contentScrollEl.value;
  const startMs = Math.max(0, Math.floor(scrollLeft / pxPerMs.value));
  const endMs = Math.ceil((scrollLeft + clientWidth) / pxPerMs.value);
  fetchLogRange(startMs, endMs);
};

const renderAll = () => {
  if (!cachedThemeStyles.value) {
    updateThemeStyles();
  }
  const styles = cachedThemeStyles.value;
  if (!styles) return;

  renderHeader(styles);
  renderContent(styles);
  requestVisibleLogRange();
};

// -----------------------------------------------------------------------------
// Event Handlers

let renderRafId: number | null = null;

const onScroll = (e: Event) => {
  // Clear tooltip when actively scrolling to prevent stale floating overlays
  hoveredBar.value = null;
  hoveredEvents.value = null;

  if (renderRafId === null) {
    renderRafId = window.requestAnimationFrame(() => {
      renderAll();
      renderRafId = null;
    });
  }
  emit('scroll', e);
};

/**
 * Handle mouse move over timeline content for hit testing.
 */
const onMouseMove = (e: MouseEvent) => {
  if (!contentScrollEl.value) return;

  const rect = contentScrollEl.value.getBoundingClientRect();
  const mouseX = e.clientX - rect.left;
  const mouseY = e.clientY - rect.top;
  const scrollLeft = contentScrollEl.value.scrollLeft;
  const scrollTop = contentScrollEl.value.scrollTop;

  // Priority 1: Instant event diamond markers (Hit Radius = 8px)
  const hitEvents: HitEventItem[] = [];
  for (let i = 0; i < visibleHitEvents.length; i++) {
    const item = visibleHitEvents[i];
    const dx = item.x - mouseX;
    const dy = item.y - mouseY;
    if (dx * dx + dy * dy <= HIT_RADIUS_PX * HIT_RADIUS_PX) {
      hitEvents.push(item);
    }
  }

  if (hitEvents.length > 0) {
    // Sort events by timestamp ascending
    hitEvents.sort((a, b) => a.event.time_ms - b.event.time_ms);
    hoveredEvents.value = hitEvents;
    hoveredBar.value = null;
    tooltipPos.value = { mouseX, mouseY, scrollLeft, scrollTop };
    return;
  }

  // Priority 2: Actual execution bars (traverse in reverse order to prefer topmost rendered bar)
  for (let i = visibleHitBars.length - 1; i >= 0; i--) {
    const bar = visibleHitBars[i];
    if (
      mouseX >= bar.x &&
      mouseX <= bar.x + bar.width &&
      mouseY >= bar.y &&
      mouseY <= bar.y + bar.height
    ) {
      hoveredBar.value = bar;
      hoveredEvents.value = null;
      tooltipPos.value = { mouseX, mouseY, scrollLeft, scrollTop };
      return;
    }
  }

  // Clear tooltip when mouse is outside of any hit target
  hoveredBar.value = null;
  hoveredEvents.value = null;
};

/**
 * Handle mouse leaving timeline content area.
 */
const onMouseLeave = () => {
  hoveredBar.value = null;
  hoveredEvents.value = null;
  tooltipPos.value = null;
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
  const targetEl = document.querySelector('.app-container') || document.documentElement;
  themeMutationObserver.observe(targetEl, {
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
    <div class="scroll-area timeline-scroll sb-hide-h" :class="{ 'has-hover': hasHoverTarget }" ref="contentScrollEl"
      @scroll="onScroll" @mousemove="onMouseMove" @mouseleave="onMouseLeave">
      <div class="timeline-content" :style="{ width: totalWidth + 'px', height: totalContentHeight + 'px' }">
        <canvas ref="contentCanvasEl" class="canvas-layer"></canvas>

        <!-- Floating Tooltip Overlay -->
        <div v-if="hoveredBar || (hoveredEvents && hoveredEvents.length > 0)" class="timeline-tooltip"
          :style="tooltipStyle">
          <!-- Actual Bar Tooltip -->
          <div v-if="hoveredBar" class="tooltip-card">
            <div class="tooltip-header">
              <div class="tooltip-title">
                <span class="tooltip-process-name">{{ hoveredBar.clientName }}</span>
                <span class="tooltip-cid-badge">CID: {{ hoveredBar.cid }}</span>
              </div>
              <span class="tooltip-bar-badge"
                :class="hoveredBar.actual.log_line_no_end === null ? 'status-incomplete' : 'status-' + hoveredBar.actual.status">
                <template v-if="hoveredBar.actual.log_line_no_end === null">INCOMPLETE</template>
                <template v-else>{{ hoveredBar.actual.status.toUpperCase() }}</template>
              </span>
            </div>
            <div class="tooltip-bar-body">
              <div class="tooltip-bar-row">
                <span class="tooltip-bar-label">Cycle:</span>
                <span class="tooltip-bar-value tooltip-mono">#{{ hoveredBar.actual.cycle }} (Instance #{{
                  hoveredBar.actual.instance_id }})</span>
              </div>
              <div class="tooltip-bar-row">
                <span class="tooltip-bar-label">Duration:</span>
                <span class="tooltip-bar-value tooltip-mono">
                  <template v-if="hoveredBar.actual.log_line_no_end === null">
                    {{ hoveredBar.actual.duration_ms.toFixed(2) }} ms
                    <span class="tooltip-bar-sub">(Cut off at log end)</span>
                  </template>
                  <template v-else-if="hoveredBar.actual.duration_ms === 0">
                    0.00 ms
                    <span class="tooltip-bar-sub">(&lt; 1 ms / Min width)</span>
                  </template>
                  <template v-else>
                    {{ hoveredBar.actual.duration_ms.toFixed(2) }} ms
                    <template v-if="hoveredBar.planDurationMs !== null">
                      <span class="tooltip-bar-sub">(Plan: {{ hoveredBar.planDurationMs.toFixed(2) }} ms / Δ{{
                        formatDelta(hoveredBar.actual.duration_ms - hoveredBar.planDurationMs) }})</span>
                    </template>
                  </template>
                </span>
              </div>
              <div class="tooltip-bar-row">
                <span class="tooltip-bar-label">Start:</span>
                <span class="tooltip-bar-value tooltip-mono">
                  {{ hoveredBar.actual.start_ms.toFixed(2) }} ms
                  <template v-if="hoveredBar.planStartMs !== null">
                    <span class="tooltip-bar-sub">(Plan: {{ hoveredBar.planStartMs.toFixed(2) }} ms / Δ{{
                      formatDelta(hoveredBar.actual.start_ms - hoveredBar.planStartMs) }})</span>
                  </template>
                </span>
              </div>
              <div class="tooltip-bar-row">
                <span class="tooltip-bar-label">End:</span>
                <span class="tooltip-bar-value tooltip-mono">
                  {{ (hoveredBar.actual.start_ms + hoveredBar.actual.duration_ms).toFixed(2) }} ms
                  <template v-if="hoveredBar.planStartMs !== null && hoveredBar.planDurationMs !== null">
                    <span class="tooltip-bar-sub">(Plan: {{ (hoveredBar.planStartMs +
                      hoveredBar.planDurationMs).toFixed(2) }}
                      ms / Δ{{ formatDelta((hoveredBar.actual.start_ms + hoveredBar.actual.duration_ms) -
                        (hoveredBar.planStartMs + hoveredBar.planDurationMs)) }})</span>
                  </template>
                </span>
              </div>
              <div class="tooltip-bar-row">
                <span class="tooltip-bar-label">Log Line:</span>
                <span class="tooltip-bar-value tooltip-mono">
                  L{{ hoveredBar.actual.log_line_no_start }}
                  <template v-if="hoveredBar.actual.log_line_no_end"> - L{{
                    hoveredBar.actual.log_line_no_end }}</template>
                  <template v-else> (Cut off)</template>
                </span>
              </div>
            </div>
          </div>

          <!-- Instant Event(s) Tooltip -->
          <div v-else-if="hoveredEvents && hoveredEvents.length > 0" class="tooltip-card">
            <div class="tooltip-header">
              <div class="tooltip-title">
                <span class="tooltip-process-name">{{ hoveredEvents[0].clientName }}</span>
                <span class="tooltip-cid-badge">CID: {{ hoveredEvents[0].cid }}</span>
              </div>
            </div>
            <div v-if="hoveredEvents.length > 1" class="tooltip-event-nearby-header">
              Nearby Events ({{ hoveredEvents.length }})
            </div>
            <div class="tooltip-event-list">
              <div v-for="item in hoveredEvents.slice(0, 4)" :key="item.event.log_line_no" class="tooltip-event-item">
                <div class="tooltip-event-row-top">
                  <span class="tooltip-event-badge" :class="'event-' + item.event.event_type">
                    {{ item.event.event_type.toUpperCase() }}
                  </span>
                  <span class="tooltip-mono">Cycle #{{ item.cycle }} (Instance #{{ item.event.instance_id }})</span>
                </div>
                <div class="tooltip-event-row-bottom">
                  <span class="tooltip-mono">Time: {{ item.event.time_ms.toFixed(2) }} ms</span>
                  <span class="tooltip-event-divider">|</span>
                  <span class="tooltip-mono">Log Line: L{{ item.event.log_line_no }}</span>
                </div>
              </div>
              <div v-if="hoveredEvents.length > 4" class="tooltip-event-more">
                + {{ hoveredEvents.length - 4 }} more events
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

/* --- Hover State --- */
.timeline-scroll.has-hover .canvas-layer {
  cursor: pointer;
}

/* -----------------------------------------------------------------------------
 * Tooltip Overlay Components
 * ----------------------------------------------------------------------------- */

/* --- Base Tooltip --- */
.timeline-tooltip {
  position: absolute;
  z-index: 30;
  width: max-content;
  max-width: 380px;
  padding: 10px 12px;
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
  gap: 8px;
}

.tooltip-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--rt-color-border);
}

.tooltip-title {
  display: flex;
  align-items: baseline;
  gap: 6px;
}

.tooltip-process-name {
  color: var(--rt-color-text);
  font-size: var(--rt-font-s, 12px);
  font-weight: 600;
}

.tooltip-cid-badge {
  color: var(--rt-color-text-dim);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: var(--rt-font-xs, 11px);
}

.tooltip-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-variant-numeric: tabular-nums;
}

/* --- Actual Bar Tooltip (tooltip-bar-*) --- */
.tooltip-bar-badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: var(--rt-font-xs, 11px);
  font-weight: 700;
  letter-spacing: 0.5px;
}

.tooltip-bar-badge.status-normal {
  background-color: var(--rt-color-status-normal);
  color: var(--rt-color-on-status-normal);
}

.tooltip-bar-badge.status-overrun {
  background-color: var(--rt-color-status-overrun);
  color: var(--rt-color-on-status-overrun);
}

.tooltip-bar-badge.status-skip {
  background-color: var(--rt-color-status-skip);
  color: var(--rt-color-on-status-skip);
}

.tooltip-bar-badge.status-incomplete {
  background-color: var(--rt-color-status-incomplete);
  color: var(--rt-color-on-status-incomplete);
}

.tooltip-bar-body {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.tooltip-bar-row {
  display: flex;
  align-items: baseline;
  gap: 6px;
}

.tooltip-bar-label {
  min-width: 54px;
  color: var(--rt-color-text-dim);
  font-size: var(--rt-font-xs, 11px);
}

.tooltip-bar-value {
  color: var(--rt-color-text);
  font-size: var(--rt-font-xs, 11px);
}

.tooltip-bar-sub {
  margin-left: 4px;
  color: var(--rt-color-text-dim);
}

/* --- Instant Event Tooltip (tooltip-event-*) --- */
.tooltip-event-nearby-header {
  padding: 2px 0;
  color: var(--rt-color-text-dim);
  font-size: var(--rt-font-xs, 11px);
  font-weight: 600;
}

.tooltip-event-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.tooltip-event-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 4px 6px;
  background-color: transparent;
  border: 1px solid var(--rt-color-border);
  border-radius: 4px;
}

.tooltip-event-row-top {
  display: flex;
  align-items: center;
  gap: 8px;
}

.tooltip-event-row-bottom {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--rt-color-text-dim);
}

.tooltip-event-divider {
  opacity: 0.4;
}

.tooltip-event-badge {
  display: inline-flex;
  align-items: center;
  padding: 1px 5px;
  border-radius: 3px;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.5px;
}

.tooltip-event-badge.event-ready {
  background-color: var(--rt-color-event-ready);
  color: var(--rt-color-on-event-ready);
}

.tooltip-event-badge.event-exit {
  background-color: var(--rt-color-event-exit);
  color: var(--rt-color-on-event-exit);
}

.tooltip-event-badge.event-overrun {
  background-color: var(--rt-color-event-overrun);
  color: var(--rt-color-on-event-overrun);
}

.tooltip-event-badge.event-error {
  background-color: var(--rt-color-event-error);
  color: var(--rt-color-on-event-error);
}

.tooltip-event-badge.event-skip {
  background-color: var(--rt-color-event-skip);
  color: var(--rt-color-on-event-skip);
}

.tooltip-event-badge.event-late {
  background-color: var(--rt-color-event-late);
  color: var(--rt-color-on-event-late);
}

.tooltip-event-badge.event-retransmit {
  background-color: var(--rt-color-event-retransmit);
  color: var(--rt-color-on-event-retransmit);
}

.tooltip-event-more {
  padding-top: 2px;
  color: var(--rt-color-text-dim);
  font-size: var(--rt-font-xs, 11px);
  text-align: center;
}
</style>
