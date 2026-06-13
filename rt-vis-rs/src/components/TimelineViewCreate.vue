<script setup lang="ts">
import { ref, computed } from 'vue';
import { useAppState } from '../composables/useAppState';
import { useTimeScale } from '../composables/useTimeScale';
import { useCreateModeLayout } from '../composables/useCreateModeLayout';
import type { PlannedExecution } from '../types/simulation';

// --- State and Composables ---
const { config, planned_executions, config_errors } = useAppState();
const { cycleTimeMs, getPos, getMs } = useTimeScale();
const { totalCycles, gridInfo, totalWidth } = useCreateModeLayout();

// -----------------------------------------------------------------------------
// Props and Emits

const emit = defineEmits<{
  (e: 'scroll', event: Event): void
}>();

// -----------------------------------------------------------------------------
// State, Computed, and Logic

// --- Layout Constants ---

const ROW_HEIGHT = 70;   // Fixed height of each process row (px)
const RECT_HEIGHT = 36;  // Height of the execution bar (px)

// --- Viewport and Scrolling ---

const headerScrollEl = ref<HTMLElement | null>(null);
const contentScrollEl = ref<HTMLElement | null>(null);

const onScroll = (e: Event) => {
  emit('scroll', e);
};

// --- Validation and Error Helpers ---

const getErrors = (cid: number) => {
  return config_errors.value[cid] || [];
};

/**
 * Limit and format configuration errors for display in the timeline.
 * Ensures that errors do not overflow the row height by capping at 4 lines.
 */
const getDisplayErrors = (cid: number) => {
  const allErrors = getErrors(cid);
  if (allErrors.length <= 4) {
    return allErrors.map(text => ({ text: `• ${text}`, isSummary: false }));
  }
  const display = allErrors.slice(0, 3).map(text => ({ text: `• ${text}`, isSummary: false }));
  display.push({ text: `+ ${allErrors.length - 3} more errors...`, isSummary: true });
  return display;
};

// --- Data Filtering ---

/**
 * Filter planned executions to only those whose processes still exist in the configuration.
 * This prevents "ghost bars" from jumping to the top of the chart during the
 * debounce period after a process deletion or CID change.
 */
const activeExecutions = computed(() => {
  const existingCids = new Set(config.client_configs.map(c => c.data.client_id));
  return planned_executions.value.filter(e => existingCids.has(e.cid));
});

/**
 * Identify the first execution instance for each process (CID).
 * These instances are the primary targets for interactive editing (D&D).
 */
const firstInstances = computed(() => {
  const seen = new Set<number>();
  const firsts = new Set<number>();
  for (const exec of activeExecutions.value) {
    if (!seen.has(exec.cid)) {
      seen.add(exec.cid);
      firsts.add(exec.instance_id);
    }
  }
  return firsts;
});

/**
 * Identify CIDs that have any execution with a warning status (Overrun or Skip).
 * Used to highlight the entire row for better visibility.
 */
const warningCids = computed(() => {
  const cids = new Set<number>();
  activeExecutions.value.forEach(exec => {
    if (exec.status !== 'normal') {
      cids.add(exec.cid);
    }
  });
  return cids;
});

// --- Highlighting Logic ---

const hoveredInstanceId = ref<number | null>(null);

/**
 * Identify all execution instances related to the currently hovered instance.
 * Includes the instance itself, its immediate ancestors (depends), and immediate descendants.
 */
const highlightedIds = computed(() => {
  if (hoveredInstanceId.value === null) return new Set<number>();

  const ids = new Set<number>([hoveredInstanceId.value]);
  // Use activeExecutions to ensure we don't highlight stale/deleted data
  const target = activeExecutions.value.find(e => e.instance_id === hoveredInstanceId.value);

  if (target) {
    // 1. Add Parents (Ancestors) - instances this one depends on.
    target.depends_instance_ids.forEach(id => ids.add(id));

    // 2. Add Children (Descendants) - instances that depend on this one.
    activeExecutions.value.forEach(e => {
      if (e.depends_instance_ids.includes(hoveredInstanceId.value!)) {
        ids.add(e.instance_id);
      }
    });
  }

  return ids;
});

// --- Drag & Drop Editing ---

type DragMode = 'offset' | 'duration';

const dragState = ref<{
  mode: DragMode;
  startX: number;
  initialValue: number;
  clientWrap: any;
} | null>(null);

/**
 * Initialize drag operation for either Offset or Duration change.
 */
const startDrag = (event: MouseEvent, exec: PlannedExecution, mode: DragMode) => {
  const clientWrap = config.client_configs.find(c => c.data.client_id === exec.cid);
  if (!clientWrap) return;

  // Prevent text selection or other default browser behaviors during drag.
  event.preventDefault();

  dragState.value = {
    mode,
    startX: event.clientX,
    initialValue: mode === 'offset' ? clientWrap.data.cycle_offset : (clientWrap.data.expected_duration_ms ?? 0),
    clientWrap
  };

  document.body.classList.add(mode === 'offset' ? 'is-dragging-move' : 'is-dragging-resize');

  window.addEventListener('mousemove', onDrag);
  window.addEventListener('mouseup', endDrag);
};

/**
 * Handle mouse movement during drag.
 * Updates the configuration in real-time with snapping and boundary guards.
 */
const onDrag = (event: MouseEvent) => {
  if (!dragState.value) return;

  const { mode, startX, initialValue, clientWrap } = dragState.value;
  const deltaX = event.clientX - startX;
  const deltaMs = getMs(deltaX);

  if (mode === 'duration') {
    // Snap to the visual Minor Grid (synced with zoom/layout)
    const minorGridMs = getMs(gridInfo.value.minorPx);
    let newDuration = initialValue + deltaMs;
    newDuration = Math.round(newDuration / minorGridMs) * minorGridMs;

    // Boundary Guards: [MinorGrid, TotalCycleDuration]
    const minDuration = minorGridMs;
    const maxDuration = clientWrap.data.cycle * cycleTimeMs.value;
    newDuration = Math.max(minDuration, Math.min(maxDuration, newDuration));

    // Update SSOT only if the value actually changed to minimize reactivity churn.
    newDuration = Math.max(1, Math.round(newDuration));
    if (clientWrap.data.expected_duration_ms !== newDuration) {
      clientWrap.data.expected_duration_ms = newDuration;
    }
  } else if (mode === 'offset') {
    // Snap to the visual Major Grid (synced with zoom/layout)
    const majorGridMs = getMs(gridInfo.value.majorPx);
    const deltaCycles = Math.round(deltaMs / majorGridMs);
    let newOffset = initialValue + deltaCycles;

    // Boundary Guards: [0, cycle - 1]
    newOffset = Math.max(0, Math.min(clientWrap.data.cycle - 1, newOffset));

    // Update SSOT only if the value actually changed.
    if (clientWrap.data.cycle_offset !== newOffset) {
      clientWrap.data.cycle_offset = newOffset;
    }
  }
};

/**
 * Clean up drag state and event listeners.
 */
const endDrag = () => {
  if (dragState.value) {
    document.body.classList.remove('is-dragging-move', 'is-dragging-resize');
  }
  dragState.value = null;
  window.removeEventListener('mousemove', onDrag);
  window.removeEventListener('mouseup', endDrag);
};

// --- Guide Region ---

/**
 * Calculate the boundaries of the allowed area (Guide Region) during a drag operation.
 * Provides visual feedback to the user on how far they can move/resize a bar.
 */
const guideRegion = computed(() => {
  if (!dragState.value) return null;

  const { mode, clientWrap } = dragState.value;
  const cycleMs = cycleTimeMs.value;
  const processCycle = clientWrap.data.cycle;

  let startMs = 0;
  let durationMs = 0;

  if (mode === 'duration') {
    // For Duration: Shows the valid duration range within the current scheduled cycle.
    startMs = clientWrap.data.cycle_offset * cycleMs;
    durationMs = processCycle * cycleMs;
  } else if (mode === 'offset') {
    // For Offset: Shows all valid offset slots.
    startMs = 0;
    durationMs = processCycle * cycleMs;
  }

  return {
    cid: clientWrap.data.client_id,
    startMs,
    durationMs,
  };
});

// --- Coordinate Transformations ---

/**
 * Calculate the vertical Y coordinate for a specific Client ID.
 * Aligns the bar to the vertical center of its corresponding row.
 */
const getBarY = (cid: number) => {
  const index = config.client_configs.findIndex(c => c.data.client_id === cid);
  if (index === -1) return -1000; // Position off-screen if process is not found
  return (index * ROW_HEIGHT) + (ROW_HEIGHT / 2) - (RECT_HEIGHT / 2);
};

/**
 * Calculate the total height required for the SVG overlay.
 */
const svgHeight = computed(() => {
  return config.client_configs.length * ROW_HEIGHT;
});

// --- Path Generation ---

/**
 * Calculate SVG paths for dependency arrows.
 * Uses cubic Bezier curves for a smooth visual connection between execution bars.
 */
const dependencyArrows = computed(() => {
  const arrows = [];
  // Use activeExecutions to ensure arrows are only drawn for visible bars
  const execMap = new Map(activeExecutions.value.map(e => [e.instance_id, e]));

  for (const exec of activeExecutions.value) {
    const toX = getPos(exec.start_ms);
    const toY = getBarY(exec.cid) + RECT_HEIGHT / 2;

    for (const depId of exec.depends_instance_ids) {
      const depExec = execMap.get(depId);
      if (depExec) {
        const fromX = getPos(depExec.start_ms + depExec.duration_ms);
        const fromY = getBarY(depExec.cid) + RECT_HEIGHT / 2;

        // Calculate horizontal control point offset based on distance to avoid "flat" curves on short gaps.
        const dx = Math.abs(toX - fromX);
        const cpOffset = Math.max(30, dx * 0.4);

        // M: MoveTo, C: Cubic Bezier Curve
        const path = `M ${fromX},${fromY} C ${fromX + cpOffset},${fromY} ${toX - cpOffset},${toY} ${toX},${toY}`;

        arrows.push({
          id: `${depId}-${exec.instance_id}`,
          fromId: depId,
          toId: exec.instance_id,
          path
        });
      }
    }
  }
  return arrows;
});

// -----------------------------------------------------------------------------
// Expose

defineExpose({
  headerScrollEl,
  contentScrollEl
});
</script>

<template>
  <main class="timeline-pane" :key="config.sessionId">
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
    <div class="scroll-area timeline-scroll sb-hide-h" ref="contentScrollEl" @scroll="onScroll">
      <div class="timeline-content" :style="{
        width: totalWidth + 'px',
        backgroundSize: `${gridInfo.majorPx}px 100%, ${gridInfo.minorPx}px 100%`
      }">
        <!-- Row Backgrounds for structural alignment -->
        <div v-for="clientWrap in config.client_configs" :key="clientWrap.configId" class="timeline-row" :class="{
          'has-warning': warningCids.has(clientWrap.data.client_id),
          'has-error': getErrors(clientWrap.data.client_id).length > 0
        }"></div>
        <div class="timeline-row add-btn-placeholder"></div>

        <!-- SVG Layer for dynamic content (Arrows and Bars) -->
        <svg class="timeline-svg" :width="totalWidth" :height="svgHeight">
          <defs>
            <marker id="arrowhead" markerWidth="6" markerHeight="6" refX="5" refY="3" orient="auto">
              <polygon points="0 0, 6 3, 0 6" fill="var(--rt-color-accent)" />
            </marker>
            <!-- Diagonal hatching pattern for non-editable instances -->
            <pattern id="hatch" width="8" height="8" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
              <line x1="0" y1="0" x2="0" y2="8" stroke="rgba(255,255,255,0.2)" stroke-width="2" />
            </pattern>
          </defs>

          <!-- Guide Region (Visible only during drag) -->
          <rect v-if="guideRegion" :x="getPos(guideRegion.startMs)" :y="getBarY(guideRegion.cid) - 8"
            :width="getPos(guideRegion.durationMs)" :height="RECT_HEIGHT + 16" class="rt-guide-region" />

          <!-- Dependency Arrows -->
          <path v-for="arrow in dependencyArrows" :key="arrow.id" :d="arrow.path" class="rt-exec-arrow" :class="{
            'rt-exec-highlight': hoveredInstanceId !== null && (arrow.fromId === hoveredInstanceId || arrow.toId === hoveredInstanceId),
            'rt-exec-dimmed': hoveredInstanceId !== null && !(arrow.fromId === hoveredInstanceId || arrow.toId === hoveredInstanceId)
          }" marker-end="url(#arrowhead)" />

          <!-- Execution Bars Grouped by Instance -->
          <g v-for="exec in activeExecutions" :key="exec.instance_id"
            :transform="`translate(${getPos(exec.start_ms)}, ${getBarY(exec.cid)})`" class="rt-exec-bar" :class="{
              'rt-exec-highlight': highlightedIds.has(exec.instance_id),
              'rt-exec-dimmed': hoveredInstanceId !== null && !highlightedIds.has(exec.instance_id),
              'rt-exec-overrun': exec.status === 'overrun',
              'rt-exec-skip': exec.status === 'skip',
              'is-editable': firstInstances.has(exec.instance_id),
              'is-readonly': !firstInstances.has(exec.instance_id)
            }" @mouseenter="hoveredInstanceId = exec.instance_id" @mouseleave="hoveredInstanceId = null"
            @mousedown="firstInstances.has(exec.instance_id) && startDrag($event, exec, 'offset')">

            <!-- Main Bar Body -->
            <rect :width="getPos(exec.duration_ms)" :height="RECT_HEIGHT" rx="6" class="rt-exec-bar-rect" />

            <!-- Hatching overlay for read-only instances -->
            <rect v-if="!firstInstances.has(exec.instance_id)" :width="getPos(exec.duration_ms)" :height="RECT_HEIGHT"
              rx="6" fill="url(#hatch)" pointer-events="none" />

            <!-- Duration Handle (Visible only for editable instances) -->
            <rect v-if="firstInstances.has(exec.instance_id)" :x="getPos(exec.duration_ms) - 10" y="0" width="12"
              :height="RECT_HEIGHT" class="rt-exec-handle-duration"
              @mousedown.stop="startDrag($event, exec, 'duration')" />

            <text x="8" :y="RECT_HEIGHT / 2 + 4" font-weight="bold" class="rt-exec-bar-label">
              {{ exec.status === 'overrun' ? 'Overrun' : '' }}
              {{ exec.status === 'skip' ? 'Skip' : '' }}
            </text>
          </g>

          <!-- Static Configuration Errors -->
          <g v-for="(clientWrap, index) in config.client_configs" :key="'err-' + clientWrap.configId">
            <template v-if="getErrors(clientWrap.data.client_id).length > 0">
              <text v-for="(errObj, i) in getDisplayErrors(clientWrap.data.client_id)" :key="i" x="12"
                :y="index * ROW_HEIGHT + 20 + (i * 16)" class="error-text-msg"
                :class="{ 'is-summary': errObj.isSummary }">
                {{ errObj.text }}
              </text>
            </template>
          </g>
        </svg>
      </div>
    </div>
  </main>
</template>

<style scoped>
/* ==========================================================================
   Layout and Containers
   ========================================================================== */

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
  padding: 0 0.5rem;
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

.timeline-scroll {
  width: 100%;
  height: 100%;
  overflow-x: scroll;
  overflow-y: scroll;
}

.timeline-content {
  position: relative;
  min-height: 100%;
  /* Visual grid synchronization using CSS linear-gradients */
  background-image:
    linear-gradient(90deg, var(--rt-grid-major) 1px, transparent 1px),
    linear-gradient(90deg, var(--rt-grid-minor) 1px, transparent 1px);
  background-position: -1px 0, -1px 0;
}

.timeline-row {
  height: var(--row-height);
  border-bottom: var(--rt-border-main);
}

.timeline-row.has-warning {
  background-color: color-mix(in srgb, var(--rt-color-warning-container) 40%, transparent);
}

.timeline-row.has-error {
  background-color: color-mix(in srgb, var(--rt-color-error-container) 40%, transparent);
}

/* ==========================================================================
   SVG Overlay Components
   ========================================================================== */

.timeline-svg {
  position: absolute;
  top: 0;
  left: 0;
  /* Let scroll events pass through to container */
  pointer-events: none;
}

/* --- Error Overlays --- */
.error-text-msg {
  fill: var(--rt-color-on-error-container);
  font-size: var(--rt-font-xs);
}

.error-text-msg.is-summary {
  font-weight: bold;
}

/* --- Drag Guide Region --- */
.rt-guide-region {
  fill: color-mix(in srgb, var(--rt-color-accent) 20%, transparent);
  stroke: var(--rt-color-accent);
  stroke-width: 0.6;
  stroke-dasharray: 4 4;
  pointer-events: none;
}
</style>
