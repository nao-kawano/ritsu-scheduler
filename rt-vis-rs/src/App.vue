<script setup lang="ts">
import { ref, computed } from "vue";
import { useScrollSync } from "./composables/useScrollSync";
import { useAppState } from "./composables/useAppState";

import TopControl from "./components/TopControl.vue";
import ProcessList from "./components/ProcessList.vue";
import MetricsLabels from "./components/MetricsLabels.vue";
import ZoomControl from "./components/ZoomControl.vue";
import GlobalError from "./components/GlobalError.vue";

// Mode-specific components
import TimelineViewGeneric from "./components/TimelineView.vue";
import MetricsChartGeneric from "./components/MetricsChart.vue";
import TimelineViewCreate from "./components/TimelineViewCreate.vue";
import MetricsChartCreate from "./components/MetricsChartCreate.vue";

const {
  mode,
  simulation_error,
  editingClient,
  editingDependsStr,
  closeEdit,
  deleteProcess,
  isConfirmingDelete,
  resetDeleteConfirm,
} = useAppState();

// Derived state for common components
const currentErrorMessage = computed(() => {
  if (mode.value === 'Create') return simulation_error.value;
  return null;
});

// Component Refs to extract actual elements
const processListRef = ref<InstanceType<typeof ProcessList> | null>(null);
const timelineViewRef = ref<any>(null);
const metricsChartRef = ref<any>(null);

// Computed Refs for Scroll Sync (extracting the exposed HTML elements)
const processListScroll = computed(() => processListRef.value?.scrollEl || null);
const timelineHeaderScroll = computed(() => timelineViewRef.value?.headerScrollEl || null);
const timelineScroll = computed(() => timelineViewRef.value?.contentScrollEl || null);
const metricsHeaderScroll = computed(() => metricsChartRef.value?.headerScrollEl || null);
const metricsChartScroll = computed(() => metricsChartRef.value?.contentScrollEl || null);

// Initialize Scroll Sync
const { onProcessListScroll, onTimelineScroll, onMetricsScroll } = useScrollSync(
  processListScroll,
  timelineHeaderScroll,
  timelineScroll,
  metricsHeaderScroll,
  metricsChartScroll
);
</script>

<template>
  <div class="app-container" :class="mode.toLowerCase()">
    <!-- Top Pane -->
    <TopControl />

    <div class="process-section">
      <!-- Global Floating Error Display -->
      <GlobalError :message="currentErrorMessage" class="floating-error" />

      <!-- Left Pane -->
      <ProcessList ref="processListRef" @scroll="onProcessListScroll" />

      <!-- Right Pane -->
      <component :is="mode === 'Create' ? TimelineViewCreate : TimelineViewGeneric" ref="timelineViewRef"
        @scroll="onTimelineScroll" />

      <!-- Floating Zoom Control -->
      <ZoomControl class="floating-zoom" />
    </div>

    <!-- Bottom Pane -->
    <footer class="metrics-section">
      <MetricsLabels />
      <component :is="mode === 'Create' ? MetricsChartCreate : MetricsChartGeneric" ref="metricsChartRef"
        @scroll="onMetricsScroll" />
    </footer>

    <!-- Edit Popup -->
    <div v-if="editingClient" class="overlay" @click.self="closeEdit(false)">
      <form class="popup" @submit.prevent="closeEdit(true)">
        <h3>Edit Process: CID {{ String(editingClient.client_id).padStart(3, '0') }}</h3>
        <div class="form-grid">
          <label>Name</label><input type="text" v-model="editingClient.display_name" maxlength="20"
            @input="editingClient.display_name = editingClient.display_name.replace(/[^a-zA-Z0-9_-]/g, '')"
            placeholder="e.g. Camera" />
          <label>CID</label><input type="number" v-model="editingClient.client_id" min="0" required />
          <label>Cycle</label><input type="number" v-model="editingClient.cycle" min="1" required />
          <label>Offset</label><input type="number" v-model="editingClient.cycle_offset" min="0" required />
          <label>Duration (ms)</label><input type="number" v-model="editingClient.expected_duration_ms" min="0"
            required />
          <label>Depends</label><input type="text" v-model="editingDependsStr" placeholder="e.g. 10, 20" />
        </div>
        <div class="popup-actions">
          <button type="button" class="danger" :class="{ confirming: isConfirmingDelete }" @click="deleteProcess"
            @mouseleave="resetDeleteConfirm">
            {{ isConfirmingDelete ? 'Confirm Delete' : 'Delete' }}
          </button>
          <div style="flex: 1"></div>
          <button type="button" @click="closeEdit(false)">Cancel</button>
          <button type="submit" class="primary">Apply</button>
        </div>
      </form>
    </div>
  </div>
</template>

<style>
/* =========================================
   Global Styles & CSS Variables
   ========================================= */
:root {
  --top-height: 100px;
  --bottom-height: 220px;
  --left-width: 280px;
  --row-height: 84px;
  --header-row-height: 36px;
  --sb-size: 16px;

  --bg-color: #f0f2f5;
  --pane-bg: #ffffff;
  --border-color: #dcdfe6;
  --primary-color: #396cd8;
  --accent-color: #ff4081;
  --text-main: #2c3e50;
  --text-dim: #909399;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg-color: #121212;
    --pane-bg: #1e1e1e;
    --border-color: #333;
    --text-main: #e0e0e0;
    --text-dim: #888;
  }
}

* {
  box-sizing: border-box;
  overscroll-behavior: none;
}

body,
html {
  margin: 0;
  padding: 0;
  height: 100%;
  overflow: hidden;
}

/* Scrollbar Utility Classes */

/* Set fixed size for all scrollbars globally within the app */
::-webkit-scrollbar {
  width: var(--sb-size);
  height: var(--sb-size);
}

::-webkit-scrollbar-track {
  background: rgba(0, 0, 0, 0.05);
}

::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.2);
  border: 4px solid transparent;
  background-clip: content-box;
}

@media (prefers-color-scheme: dark) {
  ::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.05);
  }

  ::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.2);
  }
}

/* Utilities to hide scrollbars and remove their gutter space */
.sb-hide-all::-webkit-scrollbar {
  display: none;
}

.sb-hide-h::-webkit-scrollbar:horizontal {
  display: none;
}

.sb-hide-v::-webkit-scrollbar:vertical {
  display: none;
}

/* Utility to compensate for a missing vertical scrollbar gutter */
.sb-pad-v {
  padding-right: var(--sb-size) !important;
}
</style>

<style scoped>
/* =========================================
   App Component Scoped Styles (Layout only)
   ========================================= */
.app-container {
  display: grid;
  grid-template-rows: var(--top-height) 1fr var(--bottom-height);
  height: 100vh;
  width: 100vw;
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
  background-color: var(--bg-color);
  overflow: hidden;
}

.process-section {
  display: grid;
  grid-template-columns: var(--left-width) 1fr;
  overflow: hidden;
  min-height: 0;
  position: relative;
}

.floating-error {
  position: absolute;
  top: 20px;
  right: 40px;
  z-index: 100;
}

.floating-zoom {
  position: absolute;
  bottom: 24px;
  right: 24px;
  z-index: 50;
}

.metrics-section {
  display: grid;
  grid-template-columns: var(--left-width) 1fr;
  background-color: var(--pane-bg);
  border-top: 2px solid var(--border-color);
  height: var(--bottom-height);
}

/* Edit Popup Styles */
.overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.popup {
  background: var(--pane-bg);
  padding: 2rem;
  border-radius: 12px;
  width: 400px;
  box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
}

.form-grid {
  display: grid;
  grid-template-columns: 1fr 2fr;
  gap: 1rem;
  align-items: center;
  margin-top: 1.5rem;
}

.form-grid input {
  width: 100%;
  padding: 8px 12px;
  border-radius: 4px;
  border: 1px solid var(--border-color);
  background: var(--bg-color);
  color: var(--text-main);
  font-weight: 500;
  font-family: inherit;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.form-grid input:focus {
  outline: none;
  border-color: var(--primary-color);
  box-shadow: 0 0 0 3px rgba(57, 108, 216, 0.2);
}

.form-grid input:invalid {
  border-color: #f44336;
  box-shadow: 0 0 0 3px rgba(244, 67, 54, 0.2);
}

.popup-actions {
  margin-top: 2rem;
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 1rem;
}

.popup-actions button {
  background: transparent;
  color: var(--text-main);
  border: 1px solid var(--border-color);
  padding: 0 1.2rem;
  height: 38px;
  border-radius: 6px;
  font-weight: bold;
  cursor: pointer;
  transition: background-color 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s, transform 0.2s;
}

.popup-actions button:hover {
  background: var(--border-color);
}

button.primary {
  background: var(--primary-color);
  color: white;
  border: none;
  /* padding and height inherited from .popup-actions button */
}

button.danger {
  color: #f44336;
  border-color: #f44336;
}

button.danger:hover {
  background: rgba(244, 67, 54, 0.1);
}

button.danger.confirming {
  background: #f44336;
  color: white;
  border-color: #f44336;
  box-shadow: 0 4px 12px rgba(244, 67, 54, 0.3);
  transform: scale(1.05);
}
</style>
