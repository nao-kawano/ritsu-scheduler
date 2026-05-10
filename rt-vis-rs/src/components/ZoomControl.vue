<script setup lang="ts">
import { ref } from 'vue';
import { useTimeScale } from '../composables/useTimeScale';

// --- State and Composables ---
const { pxPerCycle, zoomPercent, minZoom, maxZoom, zoom, resetZoom } = useTimeScale();

// -----------------------------------------------------------------------------
// Props and Emits

// -----------------------------------------------------------------------------
// State, Computed, and Logic

const isExpanded = ref(false);

// -----------------------------------------------------------------------------
// Expose

</script>

<template>
  <div class="zoom-control-container" :class="{ expanded: isExpanded }" @mouseenter="isExpanded = true"
    @mouseleave="isExpanded = false">
    <button class="rt-btn rt-btn-ghost zoom-btn-icon" @click="zoom('out')" title="Zoom Out">
      <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="3"
        stroke-linecap="round">
        <line x1="5" y1="12" x2="19" y2="12" />
      </svg>
    </button>

    <div class="zoom-info">
      <span class="zoom-text" @click="resetZoom" title="Reset to 100%">{{ zoomPercent }}%</span>
      <div class="slider-wrapper">
        <input type="range" :min="minZoom" :max="maxZoom" :step="25" v-model.number="pxPerCycle" class="zoom-slider" />
      </div>
    </div>

    <button class="rt-btn rt-btn-ghost zoom-btn-icon" @click="zoom('in')" title="Zoom In">
      <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="3"
        stroke-linecap="round">
        <line x1="12" y1="5" x2="12" y2="19" />
        <line x1="5" y1="12" x2="19" y2="12" />
      </svg>
    </button>
  </div>
</template>

<style scoped>
.zoom-control-container {
  display: flex;
  align-items: center;
  height: 42px;
  padding: 4px;
  background-color: var(--rt-color-surface);
  border: var(--rt-border-main);
  border-radius: var(--rt-radius-m);
  box-shadow: var(--rt-bshadow-pop);
  overflow: hidden;
  user-select: none;
  /* Fixed height to prevent vertical jitter during expansion */
  transition: background-color 0.2s, border-color 0.2s, box-shadow 0.2s;
}

.zoom-btn-icon {
  flex-shrink: 0;
  width: 32px;
  height: 32px;
  padding: 0;
}

.zoom-info {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 50px;
  padding: 0 4px;
  /* Maintain a stable minimum width to prevent UI jitter when text changes */
  overflow: hidden;
}

.zoom-text {
  padding: 4px 8px;
  border-radius: var(--rt-radius-s);
  font-size: var(--rt-font-s);
  font-weight: 800;
  color: var(--rt-color-text);
  font-variant-numeric: tabular-nums;
  cursor: pointer;
  transition: background-color 0.2s, color 0.2s;
}

.zoom-text:hover {
  background-color: var(--rt-color-bg);
  color: var(--rt-color-primary);
}

/* Slider transition logic */
.slider-wrapper {
  display: flex;
  align-items: center;
  width: 0;
  margin-left: 0;
  opacity: 0;
  /* Disable mouse events when collapsed to stabilize hover */
  pointer-events: none;
  transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.2s ease;
}

.zoom-control-container.expanded .slider-wrapper {
  width: 120px;
  margin-left: var(--rt-spacing-s);
  margin-right: var(--rt-spacing-s);
  opacity: 1;
  /* Enable mouse events when expanded */
  pointer-events: auto;
}

.zoom-slider {
  width: 100%;
  cursor: pointer;
  accent-color: var(--rt-color-primary);
}

/* Webkit slider styling for better look */
.zoom-slider::-webkit-slider-runnable-track {
  height: 4px;
  background: var(--rt-color-border);
  border-radius: 2px;
}

.zoom-slider::-webkit-slider-thumb {
  appearance: none;
  width: 12px;
  height: 12px;
  background: var(--rt-color-primary);
  border-radius: 50%;
  margin-top: -4px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}
</style>
