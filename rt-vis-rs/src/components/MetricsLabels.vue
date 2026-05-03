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
  border-right: 1px solid var(--border-color);
  background-color: var(--pane-bg);
  height: 100%;
  display: flex;
  flex-direction: column;
}

.pane-header {
  height: var(--header-row-height);
  padding: 0 1rem;
  display: flex;
  align-items: center;
  font-weight: bold;
  font-size: 0.75rem;
  color: var(--text-dim);
  text-transform: uppercase;
  border-bottom: 1px solid var(--border-color);
  background: rgba(0, 0, 0, 0.02);
  flex-shrink: 0;
}

.metrics-labels {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.metric-label {
  /* Matching ROW_HEIGHT and METRICS_HEIGHT for UI consistency */
  height: var(--row-height);
  display: flex;
  align-items: center;
  padding: 0 1rem;
  font-size: 0.75rem;
  color: var(--text-dim);
}
</style>
