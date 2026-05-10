<script setup lang="ts">
import { ref, computed } from 'vue';
import { useAppState } from '../composables/useAppState';
import { useTimeScale } from '../composables/useTimeScale';
import { useCreateModeLayout } from '../composables/useCreateModeLayout';
import type { PlannedExecution } from '../types/simulation';

// --- State and Composables ---
const { config, planned_executions, config_errors, openEdit } = useAppState();
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

const ROW_HEIGHT = 84;   // Fixed height of each process row (px)
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

// --- Edit Process ---

/**
 * Handle clicking on an execution bar to open its process configuration.
 */
const handleExecClick = (exec: PlannedExecution) => {
  const clientWrap = config.client_configs.find(c => c.data.client_id === exec.cid);
  if (clientWrap) {
    openEdit(clientWrap);
  }
};

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
          </defs>

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
              'rt-exec-skip': exec.status === 'skip'
            }" @mouseenter="hoveredInstanceId = exec.instance_id" @mouseleave="hoveredInstanceId = null"
            @click="handleExecClick(exec)">
            <rect :width="getPos(exec.duration_ms)" :height="RECT_HEIGHT" rx="6" class="rt-exec-bar-rect" />
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
</style>
