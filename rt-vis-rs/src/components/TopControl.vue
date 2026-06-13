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

/**
 * Formats a duration in milliseconds to a human-readable string.
 * Automatically scales to 'sec' or 'min' for larger durations.
 */
const formatDuration = (totalMs: number): string => {
  if (totalMs < 1000) {
    return `${totalMs} ms`;
  } else if (totalMs < 60000) {
    const sec = (totalMs / 1000).toFixed(1);
    return `${sec} sec`;
  } else {
    const min = (totalMs / 60000).toFixed(1);
    return `${min} min`;
  }
};

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
      <div class="rt-toggle-container">
        <button :class="{ active: mode === 'Create' }" class="rt-toggle-item" @click="mode = 'Create'">Create</button>
        <button :class="{ active: mode === 'Analyze' }" class="rt-toggle-item"
          :disabled="true" title="Under Development"
          @click="mode = 'Analyze'">Analyze</button>
      </div>
    </div>
    <div class="bottom-row">
      <div class="server-info-inputs">
        <div class="rt-input-group">
          <label class="rt-input-label">Port</label>
          <input type="number" v-model.number="localServerConfig.port" min="1024" max="65535" required
            class="rt-input server-input" />
        </div>
        <div class="rt-input-group">
          <label class="rt-input-label">Cycle(ms)</label>
          <input type="number" v-model.number="localServerConfig.cycle_time_ms" min="1" required
            class="rt-input server-input" />
        </div>
        <div class="rt-input-group">
          <label class="rt-input-label">Stats cycle</label>
          <input type="number" v-model.number="localServerConfig.stats_interval_cycle" min="0" required
            class="rt-input server-input" />
          <span class="rt-input-hint" v-if="localServerConfig.stats_interval_cycle === 0">
            (= Disabled)
          </span>
          <span class="rt-input-hint" v-else-if="localServerConfig.stats_interval_cycle && localServerConfig.cycle_time_ms">
            (= {{ formatDuration(localServerConfig.stats_interval_cycle * localServerConfig.cycle_time_ms) }})
          </span>
        </div>
      </div>
      <div class="actions">
        <div class="rt-input-label">Config:</div>
        <button class="rt-btn rt-btn-secondary" :class="{ 'rt-btn-danger active': isConfirmingNew }" @click="onNew"
          @mouseleave="resetNewConfirm">
          {{ isConfirmingNew ? 'Confirm New' : 'New' }}
        </button>
        <button class="rt-btn rt-btn-secondary" @click="loadConfig">Load</button>
        <button class="rt-btn rt-btn-primary" @click="saveConfig">Save</button>
      </div>
    </div>
  </header>
</template>

<style scoped>
.global-control-pane {
  z-index: 100;
  display: flex;
  flex-direction: column;
  padding: 0 1.5rem;
  background-color: var(--rt-color-surface);
  border-bottom: var(--rt-border-main);
}

.top-row,
.bottom-row {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

.top-row {
  border-bottom: var(--rt-border-main);
}

.brand {
  font-size: var(--rt-font-brand);
  font-weight: 800;
  color: var(--rt-color-primary);
}

.server-info-inputs {
  display: flex;
  gap: 1.5rem;
  align-items: center;
}

.server-input {
  width: 80px;
}

.actions {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}
</style>
