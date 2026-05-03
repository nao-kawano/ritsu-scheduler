<script setup lang="ts">
import { ref, computed } from 'vue';
import { useAppState } from '../composables/useAppState';
import { useTimeScale } from '../composables/useTimeScale';
import { useCreateModeLayout } from '../composables/useCreateModeLayout';
import type { PlannedExecution } from '../types/simulation';

// --- State and Composables ---
const { config, plannedExecutions, configErrors, openEdit } = useAppState();
const { cycleTimeMs, getPos } = useTimeScale();
const { totalCycles, gridInfo, totalWidth } = useCreateModeLayout();

// --- Layout Constants ---
const ROW_HEIGHT = 84;   // Fixed height of each process row (px)
const RECT_HEIGHT = 36;  // Height of the execution bar (px)

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

// --- Validation and Error Helpers ---

const getErrors = (cid: number) => {
  return configErrors.value[cid] || [];
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
  return plannedExecutions.value.filter(e => existingCids.has(e.cid));
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
  const target = activeExecutions.value.find(e => e.instanceId === hoveredInstanceId.value);

  if (target) {
    // 1. Add Parents (Ancestors) - instances this one depends on.
    target.dependsInstanceIds.forEach(id => ids.add(id));

    // 2. Add Children (Descendants) - instances that depend on this one.
    activeExecutions.value.forEach(e => {
      if (e.dependsInstanceIds.includes(hoveredInstanceId.value!)) {
        ids.add(e.instanceId);
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
const getY = (cid: number) => {
  const index = config.client_configs.findIndex(c => c.data.client_id === cid);
  if (index === -1) return -1000; // Position off-screen if process is not found
  return (index * ROW_HEIGHT) + (ROW_HEIGHT / 2) - (RECT_HEIGHT / 2);
};

/**
 * Calculate the total height required for the SVG overlay.
 */
const svgHeight = computed(() => {
  return config.client_configs.length * ROW_HEIGHT + ROW_HEIGHT;
});

// --- Path Generation ---

/**
 * Calculate SVG paths for dependency arrows.
 * Uses cubic Bezier curves for a smooth visual connection between execution bars.
 */
const dependencyArrows = computed(() => {
  const arrows = [];
  // Use activeExecutions to ensure arrows are only drawn for visible bars
  const execMap = new Map(activeExecutions.value.map(e => [e.instanceId, e]));

  for (const exec of activeExecutions.value) {
    const toX = getPos(exec.startMs);
    const toY = getY(exec.cid) + RECT_HEIGHT / 2;

    for (const depId of exec.dependsInstanceIds) {
      const depExec = execMap.get(depId);
      if (depExec) {
        const fromX = getPos(depExec.startMs + depExec.durationMs);
        const fromY = getY(depExec.cid) + RECT_HEIGHT / 2;

        // Calculate horizontal control point offset based on distance to avoid "flat" curves on short gaps.
        const dx = Math.abs(toX - fromX);
        const cpOffset = Math.max(30, dx * 0.4);

        // M: MoveTo, C: Cubic Bezier Curve
        const path = `M ${fromX},${fromY} C ${fromX + cpOffset},${fromY} ${toX - cpOffset},${toY} ${toX},${toY}`;

        arrows.push({
          id: `${depId}-${exec.instanceId}`,
          fromId: depId,
          toId: exec.instanceId,
          path
        });
      }
    }
  }
  return arrows;
});

// --- Event Handlers ---

/**
 * Handle clicking on an execution bar to open its process configuration.
 */
const handleExecClick = (exec: PlannedExecution) => {
  const clientWrap = config.client_configs.find(c => c.data.client_id === exec.cid);
  if (clientWrap) {
    openEdit(clientWrap);
  }
};
</script>

<template>
  <main class="timeline-pane" :key="config.sessionId">
    <!-- Time Header (Cycle and ms markers, synced across panes) -->
    <div class="timeline-header hide-scrollbar" ref="headerScrollEl">
      <div class="time-axis" :style="{ width: totalWidth + 'px' }">
        <div v-for="n in totalCycles" :key="n" class="time-tick" :style="{ width: gridInfo.majorPx + 'px' }">
          <span class="cycle-label">Cycle {{ n - 1 }}</span>
          <span class="time-label">{{ (n - 1) * cycleTimeMs }}ms</span>
        </div>
      </div>
    </div>

    <!-- Scrollable Content Area -->
    <div class="scroll-area timeline-scroll" ref="contentScrollEl" @scroll="onScroll">
      <div class="timeline-container" :style="{
        width: totalWidth + 'px',
        backgroundSize: `${gridInfo.majorPx}px 100%, ${gridInfo.minorPx}px 100%`
      }">
        <!-- Row Backgrounds for structural alignment -->
        <div v-for="clientWrap in config.client_configs" :key="clientWrap.configId" class="timeline-row"
          :class="{ 'has-warning': warningCids.has(clientWrap.data.client_id) }"></div>
        <div class="timeline-row add-btn-placeholder"></div>

        <!-- SVG Layer for dynamic content (Arrows and Bars) -->
        <svg class="timeline-svg" :width="totalWidth" :height="svgHeight">
          <defs>
            <marker id="arrowhead" markerWidth="6" markerHeight="6" refX="5" refY="3" orient="auto">
              <polygon points="0 0, 6 3, 0 6" fill="var(--accent-color)" />
            </marker>
          </defs>

          <!-- Dependency Arrows -->
          <path v-for="arrow in dependencyArrows" :key="arrow.id" :d="arrow.path" class="arrow-path" :class="{
            'is-highlighted': hoveredInstanceId !== null && (arrow.fromId === hoveredInstanceId || arrow.toId === hoveredInstanceId),
            'is-dimmed': hoveredInstanceId !== null && !(arrow.fromId === hoveredInstanceId || arrow.toId === hoveredInstanceId)
          }" marker-end="url(#arrowhead)" />

          <!-- Execution Bars Grouped by Instance -->
          <g v-for="exec in activeExecutions" :key="exec.instanceId"
            :transform="`translate(${getPos(exec.startMs)}, ${getY(exec.cid)})`" class="exec-group" :class="{
              'is-highlighted': highlightedIds.has(exec.instanceId),
              'is-dimmed': hoveredInstanceId !== null && !highlightedIds.has(exec.instanceId),
              'is-overrun': exec.status === 'overrun',
              'is-skip': exec.status === 'skip'
            }" @mouseenter="hoveredInstanceId = exec.instanceId" @mouseleave="hoveredInstanceId = null"
            @click="handleExecClick(exec)">
            <rect :width="getPos(exec.durationMs)" :height="RECT_HEIGHT" rx="6" class="exec-rect" />
            <text x="8" :y="RECT_HEIGHT / 2 + 4" font-size="11" font-weight="bold" class="exec-label">
              {{ String(exec.cid).padStart(3, '0') }}
              {{ exec.status === 'overrun' ? ' (Overrun)' : '' }}
              {{ exec.status === 'skip' ? ' (Skip)' : '' }}
            </text>
          </g>

          <!-- Static Configuration Errors -->
          <g v-for="(clientWrap, index) in config.client_configs" :key="'err-' + clientWrap.configId">
            <template v-if="getErrors(clientWrap.data.client_id).length > 0">
              <rect x="0" :y="index * ROW_HEIGHT" :width="totalWidth" :height="ROW_HEIGHT" class="error-row-bg" />
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
   1. Layout and Containers
   ========================================================================== */

.timeline-pane {
  display: flex;
  flex-direction: column;
  background-color: var(--pane-bg);
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  height: 100%;
}

/* --- Header Section --- */
.timeline-header {
  height: var(--header-row-height);
  overflow: hidden;
  padding-right: 10px;
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

.timeline-scroll {
  overflow-y: scroll;
  overflow-x: hidden;
  width: 100%;
  height: 100%;
}

.timeline-container {
  position: relative;
  min-height: 100%;
  /* Visual grid synchronization using CSS linear-gradients */
  background-image:
    linear-gradient(90deg, rgba(128, 128, 128, 0.3) 1px, transparent 1px),
    linear-gradient(90deg, rgba(128, 128, 128, 0.1) 1px, transparent 1px);
  background-position: -1px 0, -1px 0;
}

.timeline-row {
  height: var(--row-height);
  border-bottom: 1px solid var(--border-color);
}

.timeline-row.has-warning {
  background-color: rgba(255, 200, 0, 0.12);
}

/* ==========================================================================
   2. SVG Overlay Components
   ========================================================================== */

.timeline-svg {
  position: absolute;
  top: 0;
  left: 0;
  pointer-events: none;
  /* Let scroll events pass through to container */
}

/* --- Execution Bars (Nodes) --- */
.exec-group {
  cursor: pointer;
  pointer-events: auto;
  /* Re-enable for interactions */
  transition: opacity 0.2s;
}

.exec-rect {
  fill: var(--primary-color);
  stroke: var(--border-color);
  stroke-width: 1;
  transition: fill 0.2s, stroke 0.2s, stroke-width 0.2s;
}

.exec-label {
  fill: white;
  pointer-events: none;
  user-select: none;
}

/* --- Execution Status States (Overrun / Skip) --- */
.exec-group.is-overrun .exec-rect {
  /* Deep red for overrun by default */
  fill: #a61d24;
}

.exec-group.is-skip .exec-rect {
  /* Slight fill to improve visibility against grid */
  fill: rgba(255, 255, 255, 0.5);
  stroke: var(--text-dim);
  stroke-width: 2;
  stroke-dasharray: 4 4;
}

.exec-group.is-skip .exec-label {
  fill: var(--text-dim);
}

/* --- Dependency Arrows (Edges) --- */
.arrow-path {
  fill: none;
  stroke: var(--accent-color);
  stroke-width: 2;
  opacity: 0.6;
  transition: opacity 0.2s, stroke 0.2s, stroke-width 0.2s;
}

/* --- Error Overlays --- */
.error-row-bg {
  fill: rgba(255, 77, 79, 0.08);
  stroke: rgba(255, 77, 79, 0.2);
  stroke-width: 1;
}

.error-text-msg {
  fill: #cf1322;
  font-size: 11px;
}

.error-text-msg.is-summary {
  font-weight: bold;
}

/* ==========================================================================
   3. Interaction and Highlighting States
   ========================================================================== */

/* --- Hover State --- */
.exec-group:hover .exec-rect {
  fill: var(--accent-color);
}

/* --- Active/Highlighted State --- */
.exec-group.is-highlighted {
  z-index: 10;
}

.is-highlighted .exec-rect {
  fill: var(--accent-color);
  stroke: #fff;
  stroke-width: 2;
  filter: drop-shadow(0 0 4px rgba(0, 0, 0, 0.3));
}

.is-highlighted.is-overrun .exec-rect {
  fill: #ff4d4f;
}

.is-highlighted.is-skip .exec-rect {
  /* Opaque white to clear grid interference */
  fill: rgba(255, 255, 255, 0.9);
}

.arrow-path.is-highlighted {
  stroke: var(--accent-color);
  stroke-width: 3;
  opacity: 1;
}

/* --- Dimmed State (Inactive during hover) --- */
.is-dimmed {
  opacity: 0.25 !important;
}
</style>
