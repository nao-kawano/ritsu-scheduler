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
  overflow-x: scroll;
  width: 100%;
  height: 100%;
}

.timeline-content {
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

  display: flex;
  align-items: center;
  padding: 0 1rem;
}

/* ==========================================================================
   Informational UI
   ========================================================================== */

.plan-preview {
  font-size: 0.75rem;
  color: var(--text-dim);
  opacity: 0.4;
}
</style>
