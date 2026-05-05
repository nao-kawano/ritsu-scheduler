<script setup lang="ts">
import { ref, computed } from "vue";
import { useScrollSync } from "./composables/useScrollSync";
import { useAppState } from "./composables/useAppState";

import TopControl from "./components/TopControl.vue";
import ProcessList from "./components/ProcessList.vue";
import MetricsLabels from "./components/MetricsLabels.vue";
import ZoomControl from "./components/ZoomControl.vue";
import GlobalError from "./components/GlobalError.vue";
import EditPopup from "./components/EditPopup.vue";

// Mode-specific components
import TimelineViewGeneric from "./components/TimelineView.vue";
import MetricsChartGeneric from "./components/MetricsChart.vue";
import TimelineViewCreate from "./components/TimelineViewCreate.vue";
import MetricsChartCreate from "./components/MetricsChartCreate.vue";

const {
  mode,
  simulation_error,
  selectedClientWrap,
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

    <!-- Edit Popup (Conditional) -->
    <EditPopup v-if="selectedClientWrap" />
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
</style>
