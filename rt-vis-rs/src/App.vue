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
  /* Layout Dimensions */
  --top-height: 100px;
  --bottom-height: 222px;
  --left-width: 280px;
  --row-height: 84px;
  --header-row-height: 36px;
  --sb-size: 16px;

  /* Basic Colors */
  --bg-color: #f0f2f5;
  --pane-bg: #ffffff;
  --border-color: #dcdfe6;
  --primary-color: #396cd8;
  --accent-color: #ff4081;
  --text-main: #2c3e50;
  --text-dim: #909399;

  /* Design System Tokens */
  --rt-radius-s: 4px;
  --rt-radius-m: 8px;
  --rt-radius-l: 12px;

  --rt-spacing-xs: 4px;
  --rt-spacing-s: 8px;
  --rt-spacing-m: 16px;

  --rt-shadow-pop: 0 4px 12px rgba(0, 0, 0, 0.15);
  --rt-shadow-focus: 0 0 0 3px rgba(57, 108, 216, 0.3);

  --rt-border-main: 1px solid var(--border-color);
  --rt-bg-header: rgba(0, 0, 0, 0.02);

  --rt-grid-major: rgba(128, 128, 128, 0.3);
  --rt-grid-minor: rgba(128, 128, 128, 0.1);

  /* Typography Scale */
  --rt-font-xs: 0.7rem;
  --rt-font-s: 0.8rem;
  --rt-font-m: 0.9rem;
  --rt-font-l: 1.1rem;
  --rt-font-brand: 1.4rem;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg-color: #121212;
    --pane-bg: #1e1e1e;
    --border-color: #333;
    --text-main: #e0e0e0;
    --text-dim: #888;

    --rt-bg-header: rgba(255, 255, 255, 0.03);
    --rt-grid-major: rgba(255, 255, 255, 0.15);
    --rt-grid-minor: rgba(255, 255, 255, 0.05);
  }
}

* {
  box-sizing: border-box;
  overscroll-behavior: none;
}

body,
html {
  height: 100%;
  margin: 0;
  padding: 0;
  overflow: hidden;
  background-color: var(--bg-color);
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
  font-size: var(--rt-font-m);
  color: var(--text-main);
}

/* =========================================
   Common Components (Global Classes)
   ========================================= */

/* --- Buttons --- */
.rt-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 38px;
  padding: 0 1.2rem;
  border: 1px solid transparent;
  border-radius: var(--rt-radius-m);
  font-family: inherit;
  font-size: var(--rt-font-m);
  font-weight: bold;
  cursor: pointer;
  user-select: none;
  transition: background-color 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s, filter 0.2s;
}

.rt-btn-primary {
  background: var(--primary-color);
  color: white;
}

.rt-btn-primary:hover {
  box-shadow: 0 2px 8px rgba(57, 108, 216, 0.4);
  filter: brightness(1.1);
}

.rt-btn-secondary {
  border-color: var(--border-color);
  background: var(--bg-color);
  color: var(--text-main);
}

.rt-btn-secondary:hover {
  border-color: var(--primary-color);
  background: var(--pane-bg);
  color: var(--primary-color);
}

.rt-btn-danger {
  border-color: #ff4d4f;
  background: transparent;
  color: #ff4d4f;
}

.rt-btn-danger:hover {
  background: rgba(255, 77, 79, 0.1);
}

.rt-btn-danger.active {
  border-color: #ff4d4f;
  background: #ff4d4f;
  box-shadow: 0 4px 12px rgba(255, 77, 79, 0.3);
  color: white;
}

.rt-btn-ghost {
  padding: 0 var(--rt-spacing-s);
  border: none;
  background: transparent;
  color: var(--text-dim);
}

.rt-btn-ghost:hover {
  background: rgba(0, 0, 0, 0.05);
  color: var(--text-main);
}

@media (prefers-color-scheme: dark) {
  .rt-btn-ghost:hover {
    background: rgba(255, 255, 255, 0.1);
  }
}

/* --- Toggle Switch --- */
.rt-toggle-container {
  display: flex;
  height: 34px;
  padding: 3px;
  border-radius: var(--rt-radius-m);
  background: rgba(0, 0, 0, 0.05);
  box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.1);
}

@media (prefers-color-scheme: dark) {
  .rt-toggle-container {
    background: rgba(255, 255, 255, 0.05);
  }
}

.rt-toggle-item {
  display: flex;
  align-items: center;
  padding: 0 1rem;
  border: none;
  border-radius: calc(var(--rt-radius-m) - 2px);
  background: transparent;
  font-size: var(--rt-font-m);
  font-weight: 600;
  color: var(--text-dim);
  cursor: pointer;
  transition: background-color 0.2s, color 0.2s, box-shadow 0.2s;
}

.rt-toggle-item:hover:not(.active) {
  color: var(--text-main);
}

.rt-toggle-item.active {
  background: var(--pane-bg);
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
  color: var(--primary-color);
}

/* --- Form Inputs --- */
.rt-input {
  height: 34px;
  padding: 0 12px;
  border: 1px solid var(--border-color);
  border-radius: var(--rt-radius-s);
  background: var(--bg-color);
  font-family: inherit;
  font-weight: 500;
  color: var(--text-main);
  transition: border-color 0.2s, box-shadow 0.2s;
}

.rt-input:focus {
  outline: none;
  border-color: var(--primary-color);
  box-shadow: var(--rt-shadow-focus);
}

.rt-input:invalid {
  border-color: #ff4d4f;
  box-shadow: 0 0 0 3px rgba(255, 77, 79, 0.2);
}

.rt-input-group {
  display: flex;
  gap: var(--rt-spacing-s);
  align-items: center;
}

.rt-input-label {
  font-size: var(--rt-font-m);
  font-weight: bold;
  color: var(--text-dim);
}

.rt-input-hint {
  margin-left: var(--rt-spacing-xs);
  font-size: var(--rt-font-xs);
  color: var(--text-dim);
}

/* --- Execution Elements (Timeline) --- */
.rt-exec-bar {
  cursor: pointer;
  pointer-events: auto;
}

.rt-exec-bar-rect {
  stroke: var(--border-color);
  stroke-width: 1;
  fill: var(--primary-color);
  transition: fill 0.2s, stroke 0.2s, stroke-width 0.2s, filter 0.2s;
}

.rt-exec-bar-label {
  fill: white;
  font-size: var(--rt-font-xs);
  pointer-events: none;
  user-select: none;
}

/* Status: Overrun */
.rt-exec-bar.rt-exec-overrun .rt-exec-bar-rect {
  fill: #a61d24;
}

/* Status: Skip */
.rt-exec-bar.rt-exec-skip .rt-exec-bar-rect {
  stroke: var(--text-dim);
  stroke-width: 2;
  stroke-dasharray: 4 4;
  fill: rgba(255, 255, 255, 0.5);
}

.rt-exec-bar.rt-exec-skip .rt-exec-bar-label {
  fill: var(--text-dim);
}

@media (prefers-color-scheme: dark) {
  .rt-exec-bar.rt-exec-skip .rt-exec-bar-rect {
    fill: rgba(255, 255, 255, 0.1);
  }
}

/* Dependency Arrows */
.rt-exec-arrow {
  stroke: var(--accent-color);
  stroke-width: 2;
  fill: none;
  opacity: 0.5;
  transition: stroke-width 0.2s, opacity 0.2s;
}

.rt-exec-arrow.rt-exec-highlight {
  stroke-width: 3;
  opacity: 1;
}

/* Interaction: Highlight */
.rt-exec-bar.rt-exec-highlight .rt-exec-bar-rect,
.rt-exec-bar:hover .rt-exec-bar-rect {
  stroke: #fff;
  stroke-width: 2;
  filter: drop-shadow(var(--rt-shadow-pop));
}

/* Interaction: Dimmed */
.rt-exec-dimmed {
  opacity: 0.25 !important;
}

/* --- Scrollbar Utility Classes --- */

/* Set fixed size for all scrollbars globally within the app */
::-webkit-scrollbar {
  width: var(--sb-size);
  height: var(--sb-size);
}

::-webkit-scrollbar-track {
  background: rgba(0, 0, 0, 0.05);
}

::-webkit-scrollbar-thumb {
  border: 4px solid transparent;
  background: rgba(0, 0, 0, 0.2);
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
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  background-color: var(--bg-color);
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
}

.process-section {
  position: relative;
  display: grid;
  grid-template-columns: var(--left-width) 1fr;
  min-height: 0;
  overflow: hidden;
}

.floating-error {
  position: absolute;
  z-index: 100;
  top: 20px;
  right: 40px;
}

.floating-zoom {
  position: absolute;
  z-index: 50;
  bottom: 24px;
  right: 24px;
}

.metrics-section {
  display: grid;
  grid-template-columns: var(--left-width) 1fr;
  height: var(--bottom-height);
  border-top: 2px solid var(--border-color);
  background-color: var(--pane-bg);
}
</style>
