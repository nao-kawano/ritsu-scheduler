<script setup lang="ts">
import { useAppState } from '../composables/useAppState';

const { mode, config, loadConfig, saveConfig } = useAppState();
</script>

<template>
  <header class="global-control-pane">
    <div class="top-row">
      <div class="brand">Ritsu Vis</div>
      <div class="mode-selector">
        <button :class="{ active: mode === 'Create' }" @click="mode = 'Create'">Create</button>
        <button :class="{ active: mode === 'Analyze' }" @click="mode = 'Analyze'">Analyze</button>
      </div>
    </div>
    <div class="bottom-row">
      <div class="server-info-inputs">
        <div class="input-group">
          <label>Port</label>
          <input type="number" v-model="config.server_config.port" />
        </div>
        <div class="input-group">
          <label>Cycle(ms)</label>
          <input type="number" v-model="config.server_config.cycle_time_ms" />
        </div>
        <div class="input-group">
          <label>Stats Interval(cycles)</label>
          <input type="number" v-model="config.server_config.stats_interval_cycle" />
          <span class="calc-hint"
            v-if="config.server_config.stats_interval_cycle && config.server_config.cycle_time_ms">
            (= {{ config.server_config.stats_interval_cycle * config.server_config.cycle_time_ms }} ms)
          </span>
        </div>
      </div>
      <div class="actions">
        <div class="config-actions-label">Config:</div>
        <button class="secondary" @click="loadConfig">Load</button>
        <button class="primary" @click="saveConfig">Save</button>
      </div>
    </div>
  </header>
</template>

<style scoped>
.global-control-pane {
  background-color: var(--pane-bg);
  border-bottom: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  padding: 0 1.5rem;
  z-index: 100;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
}

.top-row,
.bottom-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  flex: 1;
}

.top-row {
  border-bottom: 1px solid var(--border-color);
}

.brand {
  font-size: 1.4rem;
  font-weight: 800;
  color: var(--primary-color);
}

.server-info-inputs {
  display: flex;
  align-items: center;
  gap: 1.5rem;
}

.input-group {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.input-group label {
  font-size: 0.7rem;
  font-weight: bold;
  color: var(--text-dim);
}

.input-group input {
  width: 70px;
  padding: 4px 8px;
  border-radius: 4px;
  border: 1px solid var(--border-color);
  background: var(--bg-color);
  color: var(--text-main);
  font-weight: bold;
}

.calc-hint {
  font-size: 0.7rem;
  color: var(--text-dim);
  margin-left: 0.25rem;
}

.mode-selector {
  display: flex;
  background: var(--bg-color);
  padding: 3px;
  border-radius: 6px;
}

.mode-selector button {
  border: none;
  background: transparent;
  padding: 0.4rem 1rem;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 600;
  color: var(--text-dim);
}

.mode-selector button.active {
  background: var(--pane-bg);
  color: var(--primary-color);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.actions {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.config-actions-label {
  font-size: 0.8rem;
  font-weight: bold;
  color: var(--text-dim);
  margin-right: 0.25rem;
}

.actions button {
  border: none;
  padding: 0.6rem 1.2rem;
  border-radius: 6px;
  font-weight: bold;
  cursor: pointer;
}

.actions button.primary {
  background: var(--primary-color);
  color: white;
}

.actions button.secondary {
  background: var(--bg-color);
  color: var(--text-main);
  border: 1px solid var(--border-color);
}
</style>
