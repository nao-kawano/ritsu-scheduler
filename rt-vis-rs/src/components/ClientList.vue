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

<!-- ========================================================================== -->
<!-- Script Section                                                             -->
<!-- ========================================================================== -->
<script setup lang="ts">
// -----------------------------------------------------------------------------
// Imports

import { computed, ref } from 'vue';
import { useApp } from '../composables/useApp';
import { useConfig } from '../composables/useConfig';
import { useSimulation } from '../composables/useSimulation';
import { useExecutionLog } from '../composables/useExecutionLog';
import { useStatsPanel } from '../composables/useStatsPanel';
import { formatCid } from '../utils/format';
import { formatAnomalySummary, getAnomalySeverity } from '../utils/stats';
import type { ClientStats } from '../types/analyze';
import type { ClientConfigUI } from '../types/config';
import type { AnomalySeverity } from '../types/stats';
import AppIcon from './AppIcon.vue';

// -----------------------------------------------------------------------------
// Global State & Composables

const { mode } = useApp();
const { activeConfig, openClientEdit, addClient, moveClientConfig } = useConfig();
const { configErrors } = useSimulation();
const { logSummaryAnalyzeMode } = useExecutionLog();
const { activeTab, openSystem, openClient, isTargetActive } = useStatsPanel();

// -----------------------------------------------------------------------------
// Props & Emits

// (none)

// -----------------------------------------------------------------------------
// Types & Interfaces

interface ClientCardMeta {
  severity: AnomalySeverity;
  anomalySummary: string;
}

interface ClientCardRow {
  clientWrap: ClientConfigUI;
  errors: string[];
  meta: ClientCardMeta | null;
}

// -----------------------------------------------------------------------------
// Constants & Layout

// (none)

// -----------------------------------------------------------------------------
// Local State & Computed

// --- Viewport and Scrolling ---

const scrollEl = ref<HTMLElement | null>(null);

let scrollAnimationFrameId: number | null = null;
const scrollSpeed = ref(0);
let lastMouseY = 0;

// --- Drag and Drop State ---

const draggingIndex = ref<number | null>(null);
const targetIndex = ref<number | null>(null);

// --- Statistics & Row Lookup ---

const clientRows = computed<ClientCardRow[]>(() => {
  const statsByCid = new Map<number, ClientStats>();
  const clients = logSummaryAnalyzeMode.value?.stats.clients;
  if (clients) {
    for (const c of clients) {
      statsByCid.set(c.cid, c);
    }
  }

  return activeConfig.value.client_configs.map(clientWrap => {
    const cid = clientWrap.data.client_id;
    const stats = statsByCid.get(cid);
    return {
      clientWrap,
      errors: configErrors.value[cid] || [],
      meta: stats
        ? {
          severity: getAnomalySeverity(stats),
          anomalySummary: formatAnomalySummary(stats)
        }
        : null
    };
  });
});

// -----------------------------------------------------------------------------
// Methods & Logic

// --- Drag and Drop Calculation Helpers ---

/**
 * Update the target drop index based on current mouse coordinates.
 */
const updateTargetIndex = (clientY: number) => {
  const container = scrollEl.value;
  if (!container) return;

  const rect = container.getBoundingClientRect();
  const relativeY = clientY - rect.top + container.scrollTop;

  const rows = container.querySelectorAll('.client-row-wrapper:not(.add-btn-row)');
  let computedIndex = 0;

  for (let i = 0; i < rows.length; i++) {
    const row = rows[i] as HTMLElement;
    const rowTop = row.offsetTop;
    const rowHeight = row.offsetHeight;
    const rowMiddle = rowTop + rowHeight / 2;

    if (relativeY > rowMiddle) {
      computedIndex = i + 1;
    } else {
      break;
    }
  }

  // Boundary Guard: prevent inserting below the add client button
  const boundedIndex = Math.min(activeConfig.value.client_configs.length, Math.max(0, computedIndex));

  if (targetIndex.value !== boundedIndex) {
    targetIndex.value = boundedIndex;
  }
};

/**
 * Update the scrolling speed of the container when dragging near top/bottom boundaries.
 */
const updateScrollSpeed = (clientY: number) => {
  const container = scrollEl.value;
  if (!container) return;

  const rect = container.getBoundingClientRect();
  const localY = clientY - rect.top;

  const threshold = 40;
  const maxSpeed = 10;

  if (localY < threshold) {
    const speed = ((threshold - localY) / threshold) * maxSpeed;
    scrollSpeed.value = -speed;
  } else if (localY > rect.height - threshold) {
    const speed = ((localY - (rect.height - threshold)) / threshold) * maxSpeed;
    scrollSpeed.value = speed;
  } else {
    scrollSpeed.value = 0;
  }
};

/**
 * Start the requestAnimationFrame animation loop for autoscrolling.
 */
const startScrollLoop = () => {
  if (scrollAnimationFrameId !== null) return;

  const loop = () => {
    if (scrollSpeed.value !== 0 && scrollEl.value) {
      scrollEl.value.scrollTop += scrollSpeed.value;
      updateTargetIndex(lastMouseY);
    }
    scrollAnimationFrameId = requestAnimationFrame(loop);
  };
  scrollAnimationFrameId = requestAnimationFrame(loop);
};

/**
 * Stop the autoscrolling animation loop.
 */
const stopScrollLoop = () => {
  if (scrollAnimationFrameId !== null) {
    cancelAnimationFrame(scrollAnimationFrameId);
    scrollAnimationFrameId = null;
  }
  scrollSpeed.value = 0;
};

// --- Statistics Helpers ---

/**
 * Returns whether the per-client statistics icon should be highlighted as active.
 */
const isClientStatsIconActive = (cid: number, hasAnomaly: boolean): boolean => {
  return isTargetActive('client', cid) && (activeTab.value === 'stats' || !hasAnomaly);
};

/**
 * Returns whether the per-client anomaly events icon should be highlighted as active.
 */
const isClientAnomalyIconActive = (cid: number): boolean => {
  return isTargetActive('client', cid) && activeTab.value === 'events';
};

// -----------------------------------------------------------------------------
// Event Handlers

// --- Drag and Drop Handlers ---

/**
 * Handle the start of dragging a client card.
 */
const startDrag = (event: MouseEvent, index: number) => {
  draggingIndex.value = index;
  targetIndex.value = index;
  lastMouseY = event.clientY;

  document.body.classList.add('is-dragging-move');

  window.addEventListener('mousemove', onMouseMove);
  window.addEventListener('mouseup', onMouseUp);

  startScrollLoop();
};

/**
 * Handle mouse movement during dragging.
 */
const onMouseMove = (event: MouseEvent) => {
  lastMouseY = event.clientY;
  updateTargetIndex(event.clientY);
  updateScrollSpeed(event.clientY);
};

/**
 * Handle mouseup event to complete or cancel dragging.
 */
const onMouseUp = () => {
  if (draggingIndex.value !== null) {
    document.body.classList.remove('is-dragging-move');
  }

  if (draggingIndex.value !== null && targetIndex.value !== null) {
    moveClientConfig(draggingIndex.value, targetIndex.value);
  }

  draggingIndex.value = null;
  targetIndex.value = null;

  window.removeEventListener('mousemove', onMouseMove);
  window.removeEventListener('mouseup', onMouseUp);

  stopScrollLoop();
};

// -----------------------------------------------------------------------------
// Watchers & Reactive Triggers

// (none)

// -----------------------------------------------------------------------------
// Lifecycle Hooks & Observers

// (none)

// -----------------------------------------------------------------------------
// Expose & Exports

defineExpose({ scrollEl });
</script>

<!-- ========================================================================== -->
<!-- Template Section                                                           -->
<!-- ========================================================================== -->
<template>
  <aside class="client-list-pane" :key="activeConfig.sessionId">
    <div class="pane-header">
      <span>Clients</span>
      <button v-if="mode === 'Analyze'" type="button" class="rt-icon-btn"
        :class="{ 'is-active': isTargetActive('system') }" :disabled="!logSummaryAnalyzeMode"
        :title="logSummaryAnalyzeMode ? 'System Statistics (All Clients)' : 'Load a log file to view statistics'"
        aria-label="System Statistics" @click="openSystem('stats')">
        <AppIcon name="stats" />
      </button>
    </div>
    <div class="scroll-area client-list-scroll sb-hide-all" ref="scrollEl">
      <div class="client-list-content" style="position: relative;">
        <template v-for="(row, index) in clientRows" :key="row.clientWrap.configId">
          <div class="drop-indicator" :class="{ 'is-active': targetIndex === index }"></div>
          <div class="client-row-wrapper" :class="{ 'is-dragging': draggingIndex === index }">
            <div class="drag-handle" @mousedown.prevent.stop="startDrag($event, index)">
              <svg viewBox="0 0 14 24" width="14" height="20" fill="currentColor">
                <circle cx="4" cy="6" r="1.5" />
                <circle cx="4" cy="12" r="1.5" />
                <circle cx="4" cy="18" r="1.5" />
                <circle cx="10" cy="6" r="1.5" />
                <circle cx="10" cy="12" r="1.5" />
                <circle cx="10" cy="18" r="1.5" />
              </svg>
            </div>
            <div class="client-card" :class="{ 'has-error': row.errors.length > 0 }"
              @click="openClientEdit(row.clientWrap)" :title="row.errors.join('\n')">
              <div class="card-header">
                <div class="cid">
                  {{ formatCid(row.clientWrap.data.client_id) }}
                  <span v-if="row.clientWrap.data.display_name" class="display-name">
                    ({{ row.clientWrap.data.display_name }})
                  </span>
                </div>
              </div>
              <div class="card-body">
                <div class="card-meta">
                  <div class="details">S: {{ row.clientWrap.data.start_cycle }}, I: {{ row.clientWrap.data.interval }},
                    O: {{
                      row.clientWrap.data.offset }}, D: {{
                      row.clientWrap.data.expected_duration_ms }}ms</div>
                  <div class="depends" :class="{ 'no-deps': row.clientWrap.data.depends.length === 0 }">Deps: {{
                    row.clientWrap.data.depends.length > 0 ? row.clientWrap.data.depends.join(', ') : '-' }}</div>
                </div>
                <div v-if="mode === 'Analyze'" class="card-actions" @click.stop>
                  <!-- Optional left slot: anomaly warning/error icon -->
                  <button v-if="row.meta && row.meta.severity !== 'none'" type="button"
                    class="rt-icon-btn card-anomaly-btn" :class="[
                      `is-anomaly-${row.meta.severity}`,
                      { 'is-active': isClientAnomalyIconActive(row.clientWrap.data.client_id) }
                    ]" :title="`${row.meta.anomalySummary} (Click to view Events)`" aria-label="Client Anomaly Events"
                    @click.stop="openClient(row.clientWrap.data.client_id, 'events')">
                    <AppIcon name="anomaly" />
                  </button>
                  <!-- Fixed rightmost slot: client statistics icon -->
                  <button type="button" class="rt-icon-btn" :class="{
                    'is-active': isClientStatsIconActive(
                      row.clientWrap.data.client_id,
                      row.meta !== null && row.meta.severity !== 'none'
                    )
                  }" :disabled="!row.meta"
                    :title="row.meta ? 'Client Statistics' : 'No log statistics available for this client'"
                    aria-label="Client Statistics" @click.stop="openClient(row.clientWrap.data.client_id, 'stats')">
                    <AppIcon name="stats" />
                  </button>
                </div>
              </div>
            </div>
          </div>
        </template>
        <div class="drop-indicator" :class="{ 'is-active': targetIndex === activeConfig.client_configs.length }"></div>
        <div class="client-row-wrapper add-btn-row">
          <button class="add-btn" @click="addClient">+ Add Client</button>
        </div>
      </div>
    </div>
  </aside>
</template>

<!-- ========================================================================== -->
<!-- Style Section                                                              -->
<!-- ========================================================================== -->
<style scoped>
/* -----------------------------------------------------------------------------
 * Layout & Containers
 * ----------------------------------------------------------------------------- */

.client-list-pane {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  height: 100%;
  border-right: var(--rt-border-main);
  background-color: var(--rt-color-surface);
}

/* --- Header Section --- */

.pane-header {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  justify-content: space-between;
  height: var(--header-row-height);
  padding: 0 12px 0 16px;
  border-bottom: var(--rt-border-main);
  background: var(--rt-color-surface-header);
  font-size: var(--rt-font-xs);
  font-weight: bold;
  color: var(--rt-color-text-dim);
  text-transform: uppercase;
}

/* --- Content Section --- */

.scroll-area {
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.client-list-scroll {
  overflow-x: hidden;
  overflow-y: scroll;
  overscroll-behavior: none;
}

.client-row-wrapper {
  /* Box Model */
  display: flex;
  align-items: center;
  height: var(--row-height);
  padding: 5px 12px;
  gap: 4px;

  /* Border/Background */
  border-bottom: var(--rt-border-main);
}

/* --- Client Card Layout --- */

.client-card {
  display: flex;
  flex-direction: column;
  justify-content: center;
  width: 100%;
  height: 100%;
  padding: 4px 12px;
  background-color: var(--rt-color-surface);
  border: 1px solid var(--rt-color-border);
  border-radius: var(--rt-radius-m);
  cursor: pointer;
  transition: border-color 0.2s, box-shadow 0.2s, background-color 0.2s;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.card-header .cid {
  width: 100%;
  overflow: hidden;
  font-size: var(--rt-font-m);
  font-weight: 600;
  color: var(--rt-color-text);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cid .display-name {
  margin-left: 4px;
  font-size: var(--rt-font-s);
  font-weight: normal;
  color: var(--rt-color-text-dim);
}

.card-body {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  min-width: 0;
  margin-top: 1px;
}

.card-meta {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 0px;
  min-width: 0;
}

.card-meta .details {
  overflow: hidden;
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.card-meta .depends {
  overflow: hidden;
  font-size: var(--rt-font-xs);
  font-weight: bold;
  color: var(--rt-color-accent);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.card-meta .depends.no-deps {
  font-weight: normal;
  color: var(--rt-color-text-dim);
}

.card-actions {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
}

/* -----------------------------------------------------------------------------
 * Components & Elements
 * ----------------------------------------------------------------------------- */

/* --- Drag Handle & Action Buttons --- */

.drag-handle {
  /* Positioning */
  flex-shrink: 0;

  /* Box Model */
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 24px;

  /* Typography/Color */
  color: var(--rt-color-text-dim);

  /* Misc */
  cursor: grab;
  transition: color 0.2s;
}

.add-btn {
  width: 100%;
  height: 40px;
  border: 2px dashed var(--rt-color-border);
  border-radius: var(--rt-radius-m);
  background: transparent;
  font-size: var(--rt-font-s);
  font-weight: bold;
  color: var(--rt-color-text-dim);
  cursor: pointer;
  transition: border-color 0.2s, color 0.2s, background-color 0.2s;
}

.drop-indicator {
  /* Box Model */
  height: 4px;
  margin: -2px 12px;

  /* Border/Background */
  background-color: transparent;

  /* Misc */
  transition: background-color 0.15s, box-shadow 0.15s;
}

/* -----------------------------------------------------------------------------
 * Overlays & Tooltips
 * ----------------------------------------------------------------------------- */

/* (none) */

/* -----------------------------------------------------------------------------
 * States & Modifiers
 * ----------------------------------------------------------------------------- */

/* --- Client Card States --- */

.client-card:hover {
  border-color: var(--rt-color-primary);
  box-shadow: var(--rt-bshadow-pop);
}

.client-card.has-error {
  background-color: var(--rt-color-error-container);
  border-color: var(--rt-color-error);
  color: var(--rt-color-on-error-container);
}

.client-card.has-error:hover {
  box-shadow: var(--rt-bshadow-pop-error);
}

.client-card.has-error .cid,
.client-card.has-error .display-name,
.client-card.has-error .details,
.client-card.has-error .depends,
.client-card.has-error .depends.no-deps,
.client-card.has-error .rt-icon-btn:not(.is-active):not(:hover) {
  color: var(--rt-color-on-error-container);
}

/* --- Statistics & Anomaly Icon States --- */

.card-anomaly-btn.is-anomaly-error {
  color: var(--rt-color-error);
}

.card-anomaly-btn.is-anomaly-error:not(.is-active):hover {
  background-color: var(--rt-color-error-container);
  color: var(--rt-color-on-error-container);
}

.card-anomaly-btn.is-anomaly-error.is-active {
  background-color: var(--rt-color-error);
  box-shadow: var(--rt-bshadow-focus);
  color: var(--rt-color-on-error);
}

.card-anomaly-btn.is-anomaly-warning {
  color: var(--rt-color-warning);
}

.card-anomaly-btn.is-anomaly-warning:not(.is-active):hover {
  background-color: var(--rt-color-warning-container);
  color: var(--rt-color-on-warning-container);
}

.card-anomaly-btn.is-anomaly-warning.is-active {
  background-color: var(--rt-color-warning);
  box-shadow: var(--rt-bshadow-focus);
  color: var(--rt-color-on-warning);
}

/* --- Interactive Element States --- */

.drag-handle:hover {
  color: var(--rt-color-primary);
}

.drag-handle:active {
  cursor: grabbing;
}

.add-btn:hover {
  border-color: var(--rt-color-primary);
  background-color: var(--rt-color-surface-header);
  color: var(--rt-color-primary);
}

.client-row-wrapper.is-dragging {
  opacity: 0.5;
}

.drop-indicator.is-active {
  /* Border/Background */
  background-color: var(--rt-color-primary);
  border-radius: var(--rt-radius-s);
  box-shadow: 0 0 8px var(--rt-color-primary);
}
</style>
