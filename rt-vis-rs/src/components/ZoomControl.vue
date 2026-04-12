<script setup lang="ts">
import { ref } from 'vue';
import { useTimeScale } from '../composables/useTimeScale';

const { pxPerCycle, zoomPercent, minZoom, maxZoom, zoom, resetZoom } = useTimeScale();
const isExpanded = ref(false);
</script>

<template>
  <div class="zoom-control-container" :class="{ expanded: isExpanded }" @mouseenter="isExpanded = true"
    @mouseleave="isExpanded = false">
    <button class="zoom-btn" @click="zoom('out')" title="Zoom Out">
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

    <button class="zoom-btn" @click="zoom('in')" title="Zoom In">
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
  background-color: var(--pane-bg);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 4px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  user-select: none;
  overflow: hidden;
  /* Fixed height to prevent vertical jitter during expansion */
  height: 42px;
  transition: background-color 0.2s, border-color 0.2s, box-shadow 0.2s;
}

.zoom-btn {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: 6px;
  color: var(--text-main);
  cursor: pointer;
  transition: background-color 0.2s, color 0.2s;
  flex-shrink: 0;
}

.zoom-btn:hover {
  background-color: var(--bg-color);
  color: var(--primary-color);
}

.zoom-info {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 4px;
  /* Maintain a stable minimum width to prevent UI jitter when text changes */
  min-width: 50px;
  overflow: hidden;
}

.zoom-text {
  font-size: 0.8rem;
  font-weight: 800;
  color: var(--text-main);
  font-variant-numeric: tabular-nums;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  transition: background-color 0.2s, color 0.2s;
}

.zoom-text:hover {
  background-color: var(--bg-color);
  color: var(--primary-color);
}

/* Slider transition logic */
.slider-wrapper {
  width: 0;
  opacity: 0;
  display: flex;
  align-items: center;
  margin-left: 0;
  transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.2s ease;
  pointer-events: none;
  /* Disable mouse events when collapsed to stabilize hover */
}

.zoom-control-container.expanded .slider-wrapper {
  width: 120px;
  opacity: 1;
  margin-left: 8px;
  margin-right: 8px;
  pointer-events: auto;
  /* Enable mouse events when expanded */
}

.zoom-slider {
  width: 100%;
  cursor: pointer;
  accent-color: var(--primary-color);
}

/* Webkit slider styling for better look */
.zoom-slider::-webkit-slider-runnable-track {
  height: 4px;
  background: var(--border-color);
  border-radius: 2px;
}

.zoom-slider::-webkit-slider-thumb {
  appearance: none;
  width: 12px;
  height: 12px;
  background: var(--primary-color);
  border-radius: 50%;
  margin-top: -4px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}
</style>
