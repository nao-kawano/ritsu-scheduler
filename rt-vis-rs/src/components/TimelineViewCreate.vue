<script setup lang="ts">
import { ref, computed } from 'vue';
import { useAppState } from '../composables/useAppState';
import { useTimeScale } from '../composables/useTimeScale';
import { useCreateModeLayout } from '../composables/useCreateModeLayout';
import type { PlannedExecution } from '../types/simulation';

// --- State and Composables ---
const { config, plannedExecutions, openEdit } = useAppState();
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

// --- Data Filtering ---

/**
 * Filter planned executions to only those whose processes still exist in the configuration.
 * This prevents "ghost bars" from jumping to the top of the chart during the
 * debounce period after a process deletion or CID change.
 */
const activeExecutions = computed(() => {
  const existingCids = new Set(config.client_configs.map(c => c.client_id));
  return plannedExecutions.value.filter(e => existingCids.has(e.cid));
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
  const index = config.client_configs.findIndex(c => c.client_id === cid);
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
  const client = config.client_configs.find(c => c.client_id === exec.cid);
  if (client) {
    openEdit(client);
  }
};
</script>

<template>
  <main class="timeline-pane">
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
        <div v-for="client in config.client_configs" :key="client.client_id" class="timeline-row"></div>
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
              'is-dimmed': hoveredInstanceId !== null && !highlightedIds.has(exec.instanceId)
            }" @mouseenter="hoveredInstanceId = exec.instanceId" @mouseleave="hoveredInstanceId = null"
            @click="handleExecClick(exec)">
            <rect :width="getPos(exec.durationMs)" :height="RECT_HEIGHT" rx="6" class="exec-rect" />
            <text x="8" :y="RECT_HEIGHT / 2 + 4" fill="white" font-size="11" font-weight="bold" class="exec-label">
              {{ String(exec.cid).padStart(3, '0') }}
            </text>
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
  pointer-events: none;
  user-select: none;
}

/* --- Dependency Arrows (Edges) --- */
.arrow-path {
  fill: none;
  stroke: var(--accent-color);
  stroke-width: 2;
  opacity: 0.6;
  transition: opacity 0.2s, stroke 0.2s, stroke-width 0.2s;
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
