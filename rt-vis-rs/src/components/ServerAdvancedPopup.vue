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

import { ref, computed, watch, onMounted } from 'vue';
import { useConfig } from '../composables/useConfig';
import { LOG_LEVELS, getLogLevelPriority } from '../types/config';
import type { ServerConfig } from '../types/config';

// -----------------------------------------------------------------------------
// Global State & Composables

const { activeConfig, updateServerConfig, closeServerAdvanced } = useConfig();

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

// --- Dialog Management ---

const dialogRef = ref<HTMLDialogElement | null>(null);

// --- Edit State ---

/**
 * Working draft of the server configuration to protect SSOT until Apply is pressed.
 */
const draft = ref<ServerConfig>({ ...activeConfig.value.server_config });

/**
 * Whether file logging is enabled based on directory input.
 */
const isFileLogEnabled = computed(() => draft.value.log_file_dir.trim() !== '');

// --- Hints with Fixed Height Reservation (Layout Immobility) ---

const hintStats = computed(() => {
  const stats = draft.value.stats_interval_cycle;
  if (typeof stats !== 'number' || isNaN(stats) || stats <= 0) {
    return '(= Disabled: Statistics log output is disabled)';
  }
  const cycleTimeMs = activeConfig.value.server_config.cycle_time_ms;
  return `(= ${formatDuration(stats * cycleTimeMs)})`;
});

const hintConsoleLevel = computed(() => {
  if (isFileLogEnabled.value && draft.value.log_file_level !== 'trace') {
    return `* Limited to file log level (${draft.value.log_file_level}) or lower`;
  }
  return '';
});

const hintLogDir = computed(() => {
  if (!isFileLogEnabled.value) {
    return '(= Disabled: No directory specified)';
  }
  return '';
});

const hintFileLevel = computed(() => {
  if (!isFileLogEnabled.value) {
    return 'Disabled because log directory is not specified';
  }
  return '';
});

// -----------------------------------------------------------------------------
// Methods & Logic

/**
 * Formats a duration in milliseconds to a human-readable string.
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

/**
 * Determines whether a console log level option should be disabled.
 */
const isConsoleLevelDisabled = (level: string): boolean => {
  if (!isFileLogEnabled.value) return false;
  const filePriority = getLogLevelPriority(draft.value.log_file_level);
  const targetPriority = getLogLevelPriority(level);
  return filePriority !== -1 && targetPriority > filePriority;
};

// -----------------------------------------------------------------------------
// Event Handlers

const onCancel = () => {
  closeServerAdvanced();
};

const onApply = () => {
  let stats = draft.value.stats_interval_cycle;
  if (typeof stats !== 'number' || isNaN(stats) || stats < 0) {
    stats = 0;
  }

  updateServerConfig({
    stats_interval_cycle: stats,
    log_level: draft.value.log_level,
    log_file_dir: draft.value.log_file_dir.trim(),
    log_file_level: draft.value.log_file_level,
  });
  closeServerAdvanced();
};

// -----------------------------------------------------------------------------
// Watchers & Reactive Triggers

// Automatically clamp console log level when file log level is reduced
watch(
  () => [draft.value.log_file_dir, draft.value.log_file_level],
  () => {
    if (!isFileLogEnabled.value) return;
    const filePriority = getLogLevelPriority(draft.value.log_file_level);
    const consolePriority = getLogLevelPriority(draft.value.log_level);
    if (filePriority !== -1 && consolePriority > filePriority) {
      draft.value.log_level = draft.value.log_file_level;
    }
  }
);

// -----------------------------------------------------------------------------
// Lifecycle Hooks & Observers

onMounted(() => {
  dialogRef.value?.showModal();
});

// -----------------------------------------------------------------------------
// Expose & Exports

// (none)
</script>

<!-- ========================================================================== -->
<!-- Template Section                                                           -->
<!-- ========================================================================== -->
<template>
  <dialog ref="dialogRef" class="popup-dialog" @click.self="onCancel" @close="onCancel">
    <div class="dialog-content">
      <h3>Server Advanced Settings</h3>
      <form @submit.prevent="onApply">
        <div class="dialog-body">
          <div class="form-group">
            <label class="rt-input-label">Stats cycle</label>
            <div class="input-wrapper">
              <input type="number" v-model.number="draft.stats_interval_cycle" min="0" required class="rt-input" />
              <div class="field-hint">{{ hintStats }}</div>
            </div>
          </div>

          <div class="form-group">
            <label class="rt-input-label">Console level</label>
            <div class="input-wrapper">
              <select v-model="draft.log_level" class="rt-input">
                <option v-for="lvl in LOG_LEVELS" :key="lvl" :value="lvl" :disabled="isConsoleLevelDisabled(lvl)">
                  {{ lvl }}
                </option>
              </select>
              <div class="field-hint">{{ hintConsoleLevel }}</div>
            </div>
          </div>

          <div class="form-group">
            <label class="rt-input-label">Log directory</label>
            <div class="input-wrapper">
              <input type="text" v-model="draft.log_file_dir" placeholder="e.g. ./logs" class="rt-input" />
              <div class="field-hint">{{ hintLogDir }}</div>
            </div>
          </div>

          <div class="form-group">
            <label class="rt-input-label">File level</label>
            <div class="input-wrapper">
              <select v-model="draft.log_file_level" :disabled="!isFileLogEnabled" class="rt-input">
                <option v-for="lvl in LOG_LEVELS" :key="lvl" :value="lvl">
                  {{ lvl }}
                </option>
              </select>
              <div class="field-hint">{{ hintFileLevel }}</div>
            </div>
          </div>
        </div>

        <div class="dialog-actions">
          <button type="button" class="rt-btn rt-btn-secondary" @click="onCancel">Cancel</button>
          <button type="submit" class="rt-btn rt-btn-primary">Apply</button>
        </div>
      </form>
    </div>
  </dialog>
</template>

<!-- ========================================================================== -->
<!-- Style Section                                                              -->
<!-- ========================================================================== -->
<style scoped>
/* -----------------------------------------------------------------------------
 * Layout & Containers
 * ----------------------------------------------------------------------------- */

.dialog-content {
  width: 480px;
}

.dialog-body {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.form-group {
  display: grid;
  grid-template-columns: 120px 1fr;
  align-items: start;
  gap: 16px;
}

.form-group .rt-input-label {
  padding-top: 8px;
}

.input-wrapper {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field-hint {
  min-height: 18px;
  font-size: var(--rt-font-xs);
  line-height: 18px;
  color: var(--rt-color-text-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* -----------------------------------------------------------------------------
 * Components & Elements
 * ----------------------------------------------------------------------------- */

/* (none) */

/* -----------------------------------------------------------------------------
 * Overlays & Tooltips
 * ----------------------------------------------------------------------------- */

/* (none) */

/* -----------------------------------------------------------------------------
 * States & Modifiers
 * ----------------------------------------------------------------------------- */

/* (none) */
</style>
