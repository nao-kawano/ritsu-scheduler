<script setup lang="ts">
import { useAppState } from '../composables/useAppState';

// --- State and Composables ---
const { config } = useAppState();
</script>

<template>
  <div class="metrics-labels-pane" :key="config.sessionId">
    <div class="pane-header">Metrics Labels</div>
    <div class="metrics-labels">
      <div class="metric-label">Concurrent Processes</div>
      <div class="metric-label">Cycle Jitter</div>
    </div>
  </div>
</template>

<style scoped>
.metrics-labels-pane {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  height: 100%;
  background-color: var(--rt-color-surface);
  border-right: var(--rt-border-main);
}

.pane-header {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  height: var(--header-row-height);
  padding: 0 1rem;
  background: var(--rt-color-surface-header);
  border-bottom: var(--rt-border-main);
  font-size: var(--rt-font-xs);
  font-weight: bold;
  color: var(--rt-color-text-dim);
  text-transform: uppercase;
}

.metrics-labels {
  display: flex;
  flex: 1;
  flex-direction: column;
}

.metric-label {
  /* Matching ROW_HEIGHT and METRICS_HEIGHT for UI consistency */
  display: flex;
  align-items: center;
  height: var(--row-height);
  padding: 0 1rem;
  border-bottom: var(--rt-border-main);
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
}
</style>
