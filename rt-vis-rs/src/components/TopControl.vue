<script setup lang="ts">
import { ref, reactive, watch } from 'vue';
import { useAppState } from '../composables/useAppState';

const { mode, config, newConfig, loadConfig, saveConfig } = useAppState();

// -----------------------------------------------------------------------------
// Props and Emits

// -----------------------------------------------------------------------------
// State, Computed, and Logic

/**
 * Local drafting buffer for server configuration.
 * Using a separate buffer allows us to perform "Boundary Validation" before 
 * updating the global Source of Truth (SSOT). This prevents invalid intermediate
 * states (like empty strings during typing) from triggering simulation errors.
 */
const localServerConfig = reactive({ ...config.server_config });

/**
 * Watch local changes and sync to global state ONLY if the values are valid.
 * This ensures the global state remains clean and can be trusted by compute logic.
 */
watch(localServerConfig, (newVal) => {
  // 1. Port Validation (1024 - 65535)
  if (typeof newVal.port === 'number' && newVal.port >= 1024 && newVal.port <= 65535) {
    config.server_config.port = newVal.port;
  }
  // 2. Cycle Validation (>= 1ms)
  if (typeof newVal.cycle_time_ms === 'number' && newVal.cycle_time_ms >= 1) {
    config.server_config.cycle_time_ms = newVal.cycle_time_ms;
  }
  // 3. Stats Interval Validation (>= 0 cycles)
  const stats = newVal.stats_interval_cycle ?? 0;
  if (typeof stats === 'number' && stats >= 0) {
    config.server_config.stats_interval_cycle = stats;
  }
}, { deep: true });

/**
 * Watch global state changes (e.g., from Load Config or New Config)
 * and sync them back to the local buffer.
 */
watch(() => config.server_config, (newVal) => {
  // Object.assign provides a clean way to update all reactive properties at once.
  Object.assign(localServerConfig, newVal);
}, { deep: true });

const isConfirmingNew = ref(false);

const onNew = () => {
  if (!isConfirmingNew.value) {
    isConfirmingNew.value = true;
    return;
  }
  newConfig();
  isConfirmingNew.value = false;
};

const resetNewConfirm = () => {
  isConfirmingNew.value = false;
};

// -----------------------------------------------------------------------------
// Expose

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
          <input type="number" v-model.number="localServerConfig.port" min="1024" max="65535" required />
        </div>
        <div class="input-group">
          <label>Cycle(ms)</label>
          <input type="number" v-model.number="localServerConfig.cycle_time_ms" min="1" required />
        </div>
        <div class="input-group">
          <label>Stats Interval(cycles)</label>
          <input type="number" v-model.number="localServerConfig.stats_interval_cycle" min="0" required />
          <span class="calc-hint"
            v-if="localServerConfig.stats_interval_cycle && localServerConfig.cycle_time_ms">
            (= {{ localServerConfig.stats_interval_cycle * localServerConfig.cycle_time_ms }} ms)
          </span>
        </div>
      </div>
      <div class="actions">
        <div class="config-actions-label">Config:</div>
        <button class="secondary" :class="{ danger: isConfirmingNew }" @click="onNew" @mouseleave="resetNewConfirm">
          {{ isConfirmingNew ? 'Confirm New' : 'New' }}
        </button>
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
  transition: border-color 0.2s, box-shadow 0.2s;
}

/* Focused state for high-signal feedback */
.input-group input:focus {
  outline: none;
  border-color: var(--primary-color);
  box-shadow: 0 0 0 3px rgba(57, 108, 216, 0.2);
}

/* HTML5 Validation Feedback (via required/min/max attributes) */
.input-group input:invalid {
  border-color: #f44336;
  box-shadow: 0 0 0 3px rgba(244, 67, 54, 0.2);
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
  padding: 0 1.2rem;
  height: 38px;
  border-radius: 6px;
  font-weight: bold;
  cursor: pointer;
  transition: background-color 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s, transform 0.2s;
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

/* 2-step confirmation and danger styles */
.actions button.secondary.danger {
  color: #f44336;
  border-color: #f44336;
}

.actions button.secondary.danger:hover {
  background: rgba(244, 67, 54, 0.1);
}

.actions button.secondary.danger:active {
  transform: scale(0.98);
}

.actions button.danger {
  background: #f44336;
  color: white;
  border-color: #f44336;
  box-shadow: 0 4px 12px rgba(244, 67, 54, 0.3);
}
</style>
