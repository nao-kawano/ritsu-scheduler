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

// Theme management based on system preference
const isDark = ref(window.matchMedia('(prefers-color-scheme: dark)').matches);
const themeClass = computed(() => isDark.value ? 'dark' : 'light');

// Listen for system theme changes
window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
  isDark.value = e.matches;
});

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
  <div class="app-container" :class="[mode.toLowerCase(), themeClass]">
    <!-- Top Pane -->
    <TopControl />

    <div class="process-section">
      <!-- Global Floating Error Display -->
      <GlobalError :message="currentErrorMessage" class="floating-error" />

      <!-- Left Pane -->
      <ProcessList :key="mode" ref="processListRef" @scroll="onProcessListScroll" />

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
@import "./assets/styles/light.css";
@import "./assets/styles/dark.css";

/* =========================================
   Global Styles & CSS Variables
   ========================================= */
:root {
  /* Layout Dimensions */
  --top-height: 100px;
  --bottom-height: 193px;
  --left-width: 280px;
  --row-height: 70px;
  --header-row-height: 36px;
  --sb-size: 16px;

  /* Design System Tokens (General) */
  --rt-radius-s: 4px;
  --rt-radius-m: 8px;
  --rt-radius-l: 12px;

  --rt-spacing-xs: 4px;
  --rt-spacing-s: 8px;
  --rt-spacing-m: 16px;

  /* Typography Scale */
  --rt-font-xs: 0.7rem;
  --rt-font-s: 0.8rem;
  --rt-font-m: 0.9rem;
  --rt-font-l: 1.1rem;
  --rt-font-brand: 1.4rem;
}

/* --- Semantic Color Mapping (Binds M3 to RT) --- */
.app-container {
  /* Backgrounds */
  --rt-color-bg: var(--md-sys-color-surface-container-low);
  --rt-color-surface: var(--md-sys-color-surface-container-lowest);
  --rt-color-surface-header: var(--md-sys-color-surface-container-high);
  --rt-color-surface-input: var(--md-sys-color-surface-container-low);

  /* Text */
  --rt-color-text: var(--md-sys-color-on-surface);
  --rt-color-text-dim: var(--md-sys-color-on-surface-variant);

  /* Primary */
  --rt-color-primary: var(--md-sys-color-primary);
  --rt-color-on-primary: var(--md-sys-color-on-primary);

  /* Secondary */
  --rt-color-secondary: var(--md-sys-color-secondary);
  --rt-color-on-secondary: var(--md-sys-color-on-secondary);

  /* Accent (Tertiary in M3) */
  --rt-color-accent: var(--md-sys-color-tertiary);
  --rt-color-on-accent: var(--md-sys-color-on-tertiary);

  /* Feedback */
  --rt-color-error: var(--md-sys-color-error);
  --rt-color-on-error: var(--md-sys-color-on-error);
  --rt-color-error-container: var(--md-sys-color-error-container);
  --rt-color-on-error-container: var(--md-sys-color-on-error-container);

  --rt-color-warning-container: var(--md-extended-color-warning-color-container);
  --rt-color-on-warning-container: var(--md-extended-color-warning-on-color-container);

  /* Borders & Grid */
  --rt-color-border: var(--md-sys-color-outline-variant);
  --rt-color-outline: var(--md-sys-color-outline);

  /* Derived Tokens */
  --rt-border-main: 1px solid var(--rt-color-border);
  --rt-dshadow-pop: 3px 3px 3px var(--md-sys-color-outline);
  --rt-bshadow-pop: 3px 3px 3px 0px var(--md-sys-color-outline-variant);
  --rt-bshadow-pop-error: 3px 3px 3px 0px var(--rt-color-error-container);
  --rt-bshadow-focus: 0 0 3px 2px var(--md-sys-color-primary-fixed-dim);
  --rt-bshadow-invalid: 0 0 3px 2px var(--rt-color-error-container);
  --rt-grid-major: color-mix(in srgb, var(--rt-color-text) 30%, transparent);
  --rt-grid-minor: color-mix(in srgb, var(--rt-color-text) 10%, transparent);
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
  background-color: var(--rt-color-bg);
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
  font-size: var(--rt-font-m);
  color: var(--rt-color-text);
  cursor: default;
  user-select: none;
  -webkit-user-select: none;
}

input,
textarea {
  user-select: text;
  -webkit-user-select: text;
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
  transition: background-color 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s, filter 0.2s;
}

.rt-btn-primary {
  background: var(--rt-color-primary);
  color: var(--rt-color-on-primary);
}

.rt-btn-primary:hover {
  box-shadow: 0 2px 8px color-mix(in srgb, var(--rt-color-primary) 40%, transparent);
  filter: brightness(1.1);
}

.rt-btn-secondary {
  border-color: var(--rt-color-border);
  background: var(--rt-color-bg);
  color: var(--rt-color-text);
}

.rt-btn-secondary:hover {
  border-color: var(--rt-color-primary);
  background: var(--rt-color-surface);
  color: var(--rt-color-primary);
}

.rt-btn-danger {
  border-color: var(--rt-color-error);
  background: transparent;
  color: var(--rt-color-error);
}

.rt-btn-danger:hover {
  background: var(--rt-color-error-container);
  color: var(--rt-color-on-error-container);
}

.rt-btn-danger.active {
  border-color: var(--rt-color-error);
  background: var(--rt-color-error);
  box-shadow: 0 4px 12px color-mix(in srgb, var(--rt-color-error) 30%, transparent);
  color: var(--rt-color-on-error);
}

.rt-btn-ghost {
  padding: 0 var(--rt-spacing-s);
  border: none;
  background: transparent;
  color: var(--rt-color-text-dim);
}

.rt-btn-ghost:hover {
  background: var(--rt-color-surface-header);
  color: var(--rt-color-text);
}

/* --- Toggle Switch --- */
.rt-toggle-container {
  display: flex;
  height: 34px;
  padding: 3px;
  border-radius: var(--rt-radius-m);
  background: var(--rt-color-surface-header);
  box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.1);
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
  color: var(--rt-color-text-dim);
  cursor: pointer;
  transition: background-color 0.2s, color 0.2s, box-shadow 0.2s;
}

.rt-toggle-item:hover:not(.active) {
  color: var(--rt-color-text);
}

.rt-toggle-item.active {
  background: var(--rt-color-surface);
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
  color: var(--rt-color-primary);
}

.rt-toggle-item:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* --- Form Inputs --- */
.rt-input {
  height: 34px;
  padding: 0 12px;
  border: 1px solid var(--rt-color-border);
  border-radius: var(--rt-radius-s);
  background: var(--rt-color-surface-input);
  font-family: inherit;
  font-weight: 500;
  color: var(--rt-color-text);
  transition: border-color 0.2s, box-shadow 0.2s;
}

/* Explicitly set color-scheme based on our theme class to ensure OS/Browser UI matches */
.app-container.light .rt-input {
  color-scheme: light;
}

/* Explicitly set color-scheme based on our theme class to ensure OS/Browser UI matches */
.app-container.dark .rt-input {
  color-scheme: dark;
}

.rt-input::placeholder {
  color: var(--rt-color-text-dim);
  opacity: 0.6;
}

.rt-input:focus {
  outline: none;
  border-color: var(--rt-color-primary);
  box-shadow: var(--rt-bshadow-focus);
}

.rt-input:invalid {
  border-color: var(--rt-color-error);
  box-shadow: var(--rt-bshadow-invalid);
}

.rt-input-group {
  display: flex;
  gap: var(--rt-spacing-s);
  align-items: center;
}

.rt-input-label {
  font-size: var(--rt-font-m);
  font-weight: bold;
  color: var(--rt-color-text-dim);
}

.rt-input-hint {
  margin-left: var(--rt-spacing-xs);
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
}

/* App-wide Dragging Styles */
body.is-dragging-move {
  cursor: grabbing !important;
}

body.is-dragging-resize {
  cursor: ew-resize !important;
}

/* Prevent cursor jitter and hover effects on other elements during drag */
body.is-dragging-move *,
body.is-dragging-resize * {
  pointer-events: none !important;
  user-select: none !important;
}

/* --- Execution Elements (Timeline) --- */
.rt-exec-bar {
  pointer-events: auto;
}

/* Interaction states for interactive scheduling */
.rt-exec-bar.is-editable {
  cursor: grab;
}

.rt-exec-bar.is-readonly {
  opacity: 0.8;
  cursor: default;
  pointer-events: none;
}

.rt-exec-bar-rect {
  fill: var(--rt-color-primary);
  stroke: var(--rt-color-border);
  stroke-width: 1;
  transition: filter 0.2s ease-out;
}

.rt-exec-bar-label {
  fill: var(--rt-color-on-primary);
  font-size: var(--rt-font-xs);
  pointer-events: none;
}

/* Drag handles for duration adjustment */
.rt-exec-handle-duration {
  fill: transparent;
  cursor: ew-resize;
  pointer-events: all;
}

.rt-exec-bar.is-editable:hover .rt-exec-handle-duration {
  fill: rgba(255, 255, 255, 0.2);
}

/* Status: Overrun */
.rt-exec-bar.rt-exec-overrun .rt-exec-bar-rect {
  fill: var(--rt-color-error);
}

.rt-exec-bar.rt-exec-overrun .rt-exec-bar-label {
  fill: var(--rt-color-on-error);
}

/* Status: Skip */
.rt-exec-bar.rt-exec-skip .rt-exec-bar-rect {
  stroke: var(--rt-color-text-dim);
  stroke-width: 2;
  stroke-dasharray: 4 4;
  fill: color-mix(in srgb, var(--rt-color-surface) 20%, transparent);
}

.rt-exec-bar.rt-exec-skip .rt-exec-bar-label {
  fill: var(--rt-color-text-dim);
}

/* Dependency Arrows */
.rt-exec-arrow {
  stroke: var(--rt-color-accent);
  stroke-width: 2;
  fill: none;
  opacity: 1.0;
  transition: stroke-width 0.2s, filter 0.2s ease-out;
}

.rt-exec-arrow.rt-exec-highlight {
  stroke-width: 3;
  opacity: 1;
  filter: drop-shadow(var(--rt-dshadow-pop));
}

/* Interaction: Highlight */
.rt-exec-bar.rt-exec-highlight .rt-exec-bar-rect,
.rt-exec-bar:hover .rt-exec-bar-rect {
  stroke: var(--rt-color-surface);
  stroke-width: 2;
  filter: drop-shadow(var(--rt-dshadow-pop));
}

.rt-exec-bar.rt-exec-skip.rt-exec-highlight .rt-exec-bar-rect,
.rt-exec-bar.rt-exec-skip:hover .rt-exec-bar-rect {
  /* Make fill opaque to block shadow bleed, but keep original dashed stroke color */
  fill: var(--rt-color-surface);
  stroke: var(--rt-color-text-dim);
  stroke-width: 2;
  filter: drop-shadow(var(--rt-dshadow-pop));
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
  background: var(--rt-color-surface-header);
}

::-webkit-scrollbar-thumb {
  border: 5px solid transparent;
  background: var(--rt-color-outline);
  background-clip: content-box;
  opacity: 0.5;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--rt-color-primary);
  background-clip: content-box;
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
  background-color: var(--rt-color-bg);
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
  border-top: var(--rt-border-main);
  background-color: var(--rt-color-surface);
}
</style>
