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

import { computed } from 'vue';
import { useApp } from '../composables/useApp';
import { useConfig } from '../composables/useConfig';
import { useExecutionLog } from '../composables/useExecutionLog';
import { useSimulation } from '../composables/useSimulation';
import { useStatsPanel } from '../composables/useStatsPanel';
import AppIcon from './AppIcon.vue';

// -----------------------------------------------------------------------------
// Global State & Composables

const { mode } = useApp();
const { activeConfig } = useConfig();
const { plannedConcurrencyStatsCreateMode } = useSimulation();
const { logSummaryAnalyzeMode } = useExecutionLog();
const { openConcurrency, openJitter, isTargetActive } = useStatsPanel();

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

const hasConcurrencyStats = computed<boolean>(() => {
  return mode.value === 'Create'
    ? plannedConcurrencyStatsCreateMode.value !== null
    : logSummaryAnalyzeMode.value !== null;
});

const hasJitterStats = computed<boolean>(() => {
  return mode.value === 'Analyze' && logSummaryAnalyzeMode.value !== null;
});

const concurrencyTooltip = computed<string>(() => {
  if (hasConcurrencyStats.value) {
    return 'Concurrency Statistics';
  }
  return mode.value === 'Create'
    ? 'Fix configuration errors to view statistics'
    : 'Load a log file to view statistics';
});

const jitterTooltip = computed<string>(() => {
  return hasJitterStats.value
    ? 'Cycle Jitter Statistics'
    : 'Load a log file to view statistics';
});

// -----------------------------------------------------------------------------
// Methods & Logic

// (none)

// -----------------------------------------------------------------------------
// Event Handlers

// (none)

// -----------------------------------------------------------------------------
// Watchers & Reactive Triggers

// (none)

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
  <div class="metrics-labels-pane" :key="activeConfig.sessionId">
    <div class="pane-header">Metrics Labels</div>
    <div class="metrics-labels">
      <div class="metric-label">
        <span>Concurrent Clients</span>
        <button type="button" class="rt-icon-btn" :class="{ 'is-active': isTargetActive('concurrency') }"
          :disabled="!hasConcurrencyStats" :title="concurrencyTooltip" aria-label="Concurrency Statistics"
          @click="openConcurrency">
          <AppIcon name="stats" />
        </button>
      </div>
      <div class="metric-label">
        <span>Cycle Jitter</span>
        <button v-if="mode === 'Analyze'" type="button" class="rt-icon-btn"
          :class="{ 'is-active': isTargetActive('jitter') }" :disabled="!hasJitterStats" :title="jitterTooltip"
          aria-label="Cycle Jitter Statistics" @click="openJitter('stats')">
          <AppIcon name="stats" />
        </button>
      </div>
    </div>
  </div>
</template>

<!-- ========================================================================== -->
<!-- Style Section                                                              -->
<!-- ========================================================================== -->
<style scoped>
/* -----------------------------------------------------------------------------
 * Layout & Containers
 * ----------------------------------------------------------------------------- */

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
  padding: 0 16px;
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
  justify-content: space-between;
  gap: 8px;
  height: var(--row-height);
  padding: 0 12px 0 16px;
  border-bottom: var(--rt-border-main);
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
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
