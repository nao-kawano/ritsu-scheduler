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

<!-- ========================================================================== -->
<!-- Script Section                                                             -->
<!-- ========================================================================== -->
<script setup lang="ts">
// -----------------------------------------------------------------------------
// Imports

import { ref, reactive, computed, watch } from 'vue';
import { useApp } from '../composables/useApp';
import { useConfig } from '../composables/useConfig';
import { useExecutionLog } from '../composables/useExecutionLog';

// -----------------------------------------------------------------------------
// Global State & Composables

const { appVersion, mode } = useApp();
const { activeConfig, currentConfigPathCreateMode, newConfig, loadConfig, saveConfig, openServerAdvanced } = useConfig();
const { currentLogPathAnalyzeMode, loadLog, isLogLoading } = useExecutionLog();

// -----------------------------------------------------------------------------
// Props & Emits

// (none)

// -----------------------------------------------------------------------------
// Types & Interfaces

// (none)

// -----------------------------------------------------------------------------
// Constants & Layout

// (none)

// -----------------------------------------------------------------------------
// Local State & Computed

/**
 * Active file path currently displayed or edited.
 * Resolves to the config path in Create mode, or the log path in Analyze mode.
 */
const activeFilePath = computed<string>(() => {
  return mode.value === 'Create' ? currentConfigPathCreateMode.value : currentLogPathAnalyzeMode.value;
});

/**
 * Basename extracted from activeFilePath.
 * Returns an empty string if no file is loaded or saved yet.
 */
const activeFileName = computed<string>(() => {
  if (!activeFilePath.value) return '';
  const parts = activeFilePath.value.split(/[/\\]/);
  return parts[parts.length - 1] || '';
});

/**
 * Local drafting buffer for server configuration.
 * Using a separate buffer allows us to perform "Boundary Validation" before
 * updating the global Source of Truth (SSOT). This prevents invalid intermediate
 * states (like empty strings during typing) from triggering simulation errors.
 */
const localServerConfig = reactive({ ...activeConfig.value.server_config });

const isConfirmingNew = ref(false);

// -----------------------------------------------------------------------------
// Methods & Logic

// (none)

// -----------------------------------------------------------------------------
// Event Handlers

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
// Watchers & Reactive Triggers

/**
 * Watch local changes and sync to global state ONLY if the values are valid.
 * This ensures the global state remains clean and can be trusted by compute logic.
 */
watch(localServerConfig, (newVal) => {
  // 1. Port Validation (1024 - 65535)
  if (typeof newVal.port === 'number' && newVal.port >= 1024 && newVal.port <= 65535) {
    activeConfig.value.server_config.port = newVal.port;
  }
  // 2. Cycle Validation (>= 1ms)
  if (typeof newVal.cycle_time_ms === 'number' && newVal.cycle_time_ms >= 1) {
    activeConfig.value.server_config.cycle_time_ms = newVal.cycle_time_ms;
  }
}, { deep: true });

/**
 * Watch global state changes (e.g., from Load Config or New Config)
 * and sync them back to the local buffer.
 */
watch(() => activeConfig.value.server_config, (newVal) => {
  // Object.assign provides a clean way to update all reactive properties at once.
  Object.assign(localServerConfig, newVal);
}, { deep: true });

// -----------------------------------------------------------------------------
// Lifecycle Hooks & Observers

// (none)

// -----------------------------------------------------------------------------
// Expose & Exports

// (none)
</script>

<!-- ========================================================================== -->
<!-- Template Section                                                           -->
<!-- ========================================================================== -->
<template>
  <header class="global-control-pane">
    <div class="top-row">
      <div class="brand-container">
        <div class="brand">
          Ritsu Vis <span class="version-label" v-if="appVersion">v{{ appVersion }}</span>
        </div>
        <div class="brand-divider" />
        <div class="active-file-indicator">
          <span class="indicator-label">Active File:</span>
          <span
            class="indicator-value"
            :class="{ 'none-value': !activeFileName }"
            :title="activeFilePath || 'No active file'"
          >
            {{ activeFileName || '(None)' }}
          </span>
        </div>
      </div>
      <div class="rt-toggle-container">
        <button :class="{ active: mode === 'Create' }" class="rt-toggle-item" @click="mode = 'Create'">Create</button>
        <button :class="{ active: mode === 'Analyze' }" class="rt-toggle-item"
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
          <label class="rt-input-label">Advanced</label>
          <button type="button" class="rt-btn rt-btn-secondary server-btn"
            @click="openServerAdvanced">Settings...</button>
        </div>
      </div>
      <div class="actions">
        <template v-if="mode === 'Create'">
          <div class="rt-input-label">Config:</div>
          <button class="rt-btn rt-btn-secondary" :class="{ 'rt-btn-danger active': isConfirmingNew }" @click="onNew"
            @mouseleave="resetNewConfirm">
            {{ isConfirmingNew ? 'Confirm New' : 'New' }}
          </button>
          <button class="rt-btn rt-btn-secondary" @click="loadConfig">Load</button>
          <button class="rt-btn rt-btn-primary" @click="saveConfig">Save</button>
        </template>
        <template v-else>
          <div class="rt-input-label">Log:</div>
          <button class="rt-btn rt-btn-primary" :disabled="isLogLoading" @click="loadLog">
            {{ isLogLoading ? 'Loading...' : 'Load' }}
          </button>
        </template>
      </div>
    </div>
  </header>
</template>

<!-- ========================================================================== -->
<!-- Style Section                                                              -->
<!-- ========================================================================== -->
<style scoped>
/* -----------------------------------------------------------------------------
 * Layout & Containers
 * ----------------------------------------------------------------------------- */

.global-control-pane {
  z-index: 100;
  display: flex;
  flex-direction: column;
  padding: 0 24px;
  background-color: var(--rt-color-surface);
  border-bottom: var(--rt-border-main);
}

/* --- Row Layouts --- */

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

/* --- Section Groups --- */

.brand-container {
  display: flex;
  align-items: baseline;
  min-width: 0;
}

.server-info-inputs {
  display: flex;
  gap: 24px;
  align-items: center;
}

.actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

/* -----------------------------------------------------------------------------
 * Components & Elements
 * ----------------------------------------------------------------------------- */

.brand {
  display: inline-flex;
  align-items: baseline;
  font-size: var(--rt-font-brand);
  font-weight: 800;
  color: var(--rt-color-primary);
}

.server-input {
  width: 80px;
}

.server-btn {
  height: 34px;
  padding: 0 14px;
  font-size: var(--rt-font-s);
}

.version-label {
  margin-left: 8px;
  font-size: var(--rt-font-m);
  font-weight: normal;
  color: var(--rt-color-text-dim);
}

.brand-divider {
  align-self: center;
  flex-shrink: 0;
  width: 1px;
  height: 16px;
  margin: 0 16px;
  background-color: var(--rt-color-border);
}

.active-file-indicator {
  display: flex;
  align-items: baseline;
  gap: 6px;
  min-width: 0;
}

.indicator-label {
  font-size: var(--rt-font-m);
  color: var(--rt-color-text-dim);
  white-space: nowrap;
  user-select: none;
}

.indicator-value {
  max-width: 380px;
  overflow: hidden;
  font-size: var(--rt-font-m);
  font-weight: 600;
  color: var(--rt-color-text);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.indicator-value.none-value {
  font-weight: normal;
  color: var(--rt-color-text-dim);
}

/* -----------------------------------------------------------------------------
 * Overlays & Tooltips
 * ----------------------------------------------------------------------------- */

/* (none) */

/* -----------------------------------------------------------------------------
 * States & Modifiers
 * ----------------------------------------------------------------------------- */

/* (none) */
</style>
