<script setup lang="ts">
import { ref } from 'vue';
import { useAppState } from '../composables/useAppState';

// --- State and Composables ---
const { config } = useAppState();

// -----------------------------------------------------------------------------
// Props and Emits

const emit = defineEmits<{
  (e: 'scroll', event: Event): void
}>();

// -----------------------------------------------------------------------------
// State, Computed, and Logic

// --- Layout Constants ---

const CYCLE_MS = 50;
const TOTAL_CYCLE = 4;
const PIX_PER_CYCLE = 400;
const PIX_TOTAL_WIDTH = PIX_PER_CYCLE * TOTAL_CYCLE;
const PIX_GRID_MAJOR = PIX_PER_CYCLE;
const PIX_GRID_MINOR = PIX_PER_CYCLE / 10;

// --- Viewport and Scrolling ---

const headerScrollEl = ref<HTMLElement | null>(null);
const contentScrollEl = ref<HTMLElement | null>(null);

const onScroll = (e: Event) => {
  emit('scroll', e);
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
      <div class="time-axis" :style="{ width: PIX_TOTAL_WIDTH + 'px' }">
        <div v-for="n in TOTAL_CYCLE" :key="n" class="time-tick" :style="{ width: PIX_GRID_MAJOR + 'px' }">
          <span class="cycle-label">Cycle {{ n - 1 }}</span>
          <span class="time-label">{{ (n - 1) * CYCLE_MS }}ms</span>
        </div>
      </div>
    </div>

    <!-- Scrollable Content Area -->
    <div class="scroll-area timeline-scroll sb-hide-h" ref="contentScrollEl" @scroll="onScroll">
      <div class="timeline-content" :style="{
        width: PIX_TOTAL_WIDTH + 'px',
        backgroundSize: `${PIX_GRID_MAJOR}px 100%, ${PIX_GRID_MINOR}px 100%`
      }">
        <div v-for="clientWrap in config.client_configs" :key="clientWrap.configId" class="timeline-row">
          <div class="plan-preview">Timeline Row for CID {{ clientWrap.data.client_id }}</div>
        </div>
        <div class="timeline-row add-btn-placeholder"></div>
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
  display: flex;
  align-items: center;
  height: var(--row-height);
  padding: 0 1rem;
  border-bottom: var(--rt-border-main);
}

/* ==========================================================================
   Informational UI
   ========================================================================== */

.plan-preview {
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
  opacity: 0.4;
}
</style>
