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
import { useApp } from '../../composables/useApp';
import { useConfig } from '../../composables/useConfig';
import { useExecutionLog } from '../../composables/useExecutionLog';
import { useStatsPanel } from '../../composables/useStatsPanel';
import { formatCid } from '../../utils/format';
import { getTotalAnomalyCount } from '../../utils/stats';
import type { StatsPanelTab } from '../../types/stats';
import AppIcon from '../AppIcon.vue';
import StatsSystemView from './StatsSystemView.vue';
import StatsClientView from './StatsClientView.vue';
import StatsConcurrencyView from './StatsConcurrencyView.vue';
import StatsJitterView from './StatsJitterView.vue';

// -----------------------------------------------------------------------------
// Global State & Composables

const { mode } = useApp();
const { configAnalyzeMode } = useConfig();
const { logSummaryAnalyzeMode } = useExecutionLog();
const { target, activeTab, closePanel, setActiveTab } = useStatsPanel();

// -----------------------------------------------------------------------------
// Props & Emits

const emit = defineEmits<{
  (e: 'jump', timeUs: number, cid?: number): void;
}>();

// -----------------------------------------------------------------------------
// Types & Interfaces

interface PanelTabDef {
  id: StatsPanelTab;
  label: string;
}

// -----------------------------------------------------------------------------
// Constants & Layout

// (none)

// -----------------------------------------------------------------------------
// Local State & Computed

/**
 * Resolves the panel header title based on the active inspection target.
 */
const headerTitle = computed<string>(() => {
  const t = target.value;
  if (!t) {
    return 'Statistics';
  }
  switch (t.kind) {
    case 'system':
      return 'System (All Clients)';
    case 'client': {
      const client = configAnalyzeMode.client_configs.find(c => c.data.client_id === t.cid);
      return formatCid(t.cid, client?.data.display_name);
    }
    case 'concurrency':
      return 'Concurrent Clients';
    case 'jitter':
      return 'Cycle Jitter';
  }
});

/**
 * Resolves the short mode badge label displayed beside the concurrency header title.
 */
const modeBadgeLabel = computed<string>(() => {
  switch (mode.value) {
    case 'Create':
      return 'Plan';
    case 'Analyze':
      return 'Log';
  }
});

const systemAnomalyTotal = computed<number>(() => {
  const sys = logSummaryAnalyzeMode.value?.stats.system;
  return sys ? getTotalAnomalyCount(sys) : 0;
});

/**
 * Computes the total anomaly event count for the currently selected client target.
 */
const clientAnomalyTotal = computed<number>(() => {
  const t = target.value;
  if (!t || t.kind !== 'client') {
    return 0;
  }
  const client = logSummaryAnalyzeMode.value?.stats.clients.find(c => c.cid === t.cid);
  return client ? getTotalAnomalyCount(client) : 0;
});

const jitterOutlierCount = computed<number>(() => {
  return logSummaryAnalyzeMode.value?.stats.jitter?.outliers.length ?? 0;
});

/**
 * Defines available inspector tabs per target; concurrency is single-pane and hides the tab bar.
 */
const tabs = computed<PanelTabDef[]>(() => {
  const t = target.value;
  if (!t) {
    return [];
  }
  switch (t.kind) {
    case 'system':
      return [
        { id: 'stats', label: 'Stats' },
        { id: 'events', label: `Events (${systemAnomalyTotal.value})` }
      ];
    case 'client':
      return [
        { id: 'stats', label: 'Stats' },
        { id: 'events', label: `Events (${clientAnomalyTotal.value})` }
      ];
    case 'concurrency':
      return [];
    case 'jitter':
      return [
        { id: 'stats', label: 'Stats' },
        { id: 'outliers', label: `Outliers (${jitterOutlierCount.value})` }
      ];
  }
});

// -----------------------------------------------------------------------------
// Methods & Logic

// (none)

// -----------------------------------------------------------------------------
// Event Handlers

/**
 * Forwards timeline jump requests from child statistics views up to App.vue.
 */
const onJump = (timeUs: number, cid?: number): void => {
  emit('jump', timeUs, cid);
};

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
  <aside class="stats-panel-pane">
    <!-- Panel Header (aligned with --header-row-height) -->
    <div class="stats-panel-header">
      <div class="stats-panel-title-wrap">
        <span class="stats-panel-title" :title="headerTitle">{{ headerTitle }}</span>
        <span v-if="target?.kind === 'concurrency'" class="stats-mode-badge">
          {{ modeBadgeLabel }}
        </span>
      </div>
      <button type="button" class="stats-close-btn" title="Close statistics panel" aria-label="Close statistics panel"
        @click="closePanel">
        <AppIcon name="close" />
      </button>
    </div>

    <!-- Tab Bar -->
    <div v-if="tabs.length > 0" class="stats-tab-bar">
      <button v-for="tab in tabs" :key="tab.id" type="button" class="stats-tab-btn"
        :class="{ 'is-active': activeTab === tab.id }" @click="setActiveTab(tab.id)">
        {{ tab.label }}
      </button>
    </div>

    <!-- Panel Body -->
    <div class="stats-panel-body">
      <StatsSystemView v-if="target?.kind === 'system'" @jump="onJump" />
      <StatsClientView v-else-if="target?.kind === 'client'" :cid="target.cid" @jump="onJump" />
      <StatsConcurrencyView v-else-if="target?.kind === 'concurrency'" @jump="onJump" />
      <StatsJitterView v-else-if="target?.kind === 'jitter'" @jump="onJump" />
    </div>
  </aside>
</template>

<!-- ========================================================================== -->
<!-- Global Style Section                                                       -->
<!-- ========================================================================== -->
<style>
/* -----------------------------------------------------------------------------
 * Layout & Containers
 * ----------------------------------------------------------------------------- */

.stats-panel-pane .stats-tab-fill {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.stats-panel-pane .stats-scroll-body {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  padding: 12px;
  overflow-x: hidden;
  overflow-y: auto;
}

.stats-panel-pane .stats-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px 12px;
  border: var(--rt-border-main);
  border-radius: var(--rt-radius-m);
  background-color: var(--rt-color-surface);
}

.stats-panel-pane .stats-section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.stats-panel-pane .stats-section-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: var(--rt-font-s);
  font-weight: 700;
  color: var(--rt-color-text);
}

.stats-panel-pane .stats-kv-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 6px 16px;
}

/* -----------------------------------------------------------------------------
 * Components & Elements
 * ----------------------------------------------------------------------------- */

.stats-panel-pane .stats-primary-value {
  font-family: var(--rt-font-mono);
  font-size: var(--rt-font-m);
  font-weight: 700;
  color: var(--rt-color-text);
  font-variant-numeric: tabular-nums;
}

.stats-panel-pane .stats-subtext {
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
  font-variant-numeric: tabular-nums;
}

.stats-panel-pane .stats-empty-state {
  padding: 24px 16px;
  font-size: var(--rt-font-s);
  text-align: center;
  color: var(--rt-color-text-dim);
}

.stats-panel-pane .stats-kv-item {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 6px;
  min-width: 0;
}

.stats-panel-pane .stats-kv-label {
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
  white-space: nowrap;
}

.stats-panel-pane .stats-kv-value {
  font-family: var(--rt-font-mono);
  font-size: var(--rt-font-xs);
  font-weight: 600;
  color: var(--rt-color-text);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.stats-panel-pane .stats-jump-btn {
  flex-shrink: 0;
  padding: 2px 8px;
  border: 1px solid var(--rt-color-border);
  border-radius: var(--rt-radius-s);
  background-color: var(--rt-color-bg);
  font-family: inherit;
  font-size: var(--rt-font-xs);
  font-weight: 600;
  color: var(--rt-color-primary);
  cursor: pointer;
  transition: border-color 0.15s, background-color 0.15s, color 0.15s;
}

.stats-panel-pane .stats-event-badge {
  display: inline-flex;
  flex-shrink: 0;
  align-items: center;
  padding: 1px 5px;
  border-radius: 3px;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.3px;
}

/* -----------------------------------------------------------------------------
 * Overlays & Tooltips
 * ----------------------------------------------------------------------------- */

/* (none) */

/* -----------------------------------------------------------------------------
 * States & Modifiers
 * ----------------------------------------------------------------------------- */

.stats-panel-pane .stats-jump-btn:hover {
  border-color: var(--rt-color-primary);
  background-color: var(--rt-color-primary);
  color: var(--rt-color-on-primary);
}

.stats-panel-pane .stats-kv-value.is-negative {
  color: var(--rt-color-error);
}

.stats-panel-pane .stats-event-fill.event-overrun {
  background-color: var(--rt-color-event-overrun);
  color: var(--rt-color-on-event-overrun);
}

.stats-panel-pane .stats-event-fill.event-skip {
  background-color: var(--rt-color-event-skip);
  color: var(--rt-color-on-event-skip);
}

.stats-panel-pane .stats-event-fill.event-late {
  background-color: var(--rt-color-event-late);
  color: var(--rt-color-on-event-late);
}

.stats-panel-pane .stats-event-fill.event-retransmit {
  background-color: var(--rt-color-event-retransmit);
  color: var(--rt-color-on-event-retransmit);
}

.stats-panel-pane .stats-event-fill.event-error {
  background-color: var(--rt-color-event-error);
  color: var(--rt-color-on-event-error);
}

.stats-panel-pane .stats-event-fill.event-exit {
  background-color: var(--rt-color-event-exit);
  color: var(--rt-color-on-event-exit);
}
</style>

<!-- ========================================================================== -->
<!-- Scoped Style Section                                                       -->
<!-- ========================================================================== -->
<style scoped>
/* -----------------------------------------------------------------------------
 * Layout & Containers
 * ----------------------------------------------------------------------------- */

.stats-panel-pane {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  height: 100%;
  border-left: var(--rt-border-main);
  background-color: var(--rt-color-bg);
}

.stats-panel-header {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  height: var(--header-row-height);
  padding: 0 12px;
  border-bottom: var(--rt-border-main);
  background-color: var(--rt-color-surface-header);
}

.stats-panel-title-wrap {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.stats-tab-bar {
  display: flex;
  flex-shrink: 0;
  border-bottom: var(--rt-border-main);
  background-color: var(--rt-color-surface);
}

.stats-panel-body {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

/* -----------------------------------------------------------------------------
 * Components & Elements
 * ----------------------------------------------------------------------------- */

.stats-panel-title {
  overflow: hidden;
  font-size: var(--rt-font-s);
  font-weight: 700;
  color: var(--rt-color-text);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.stats-mode-badge {
  flex-shrink: 0;
  padding: 1px 6px;
  border-radius: var(--rt-radius-s);
  background-color: var(--rt-color-primary-container);
  font-size: 10px;
  font-weight: 700;
  color: var(--rt-color-on-primary-container);
  text-transform: uppercase;
}

.stats-close-btn {
  display: inline-flex;
  flex-shrink: 0;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: none;
  border-radius: var(--rt-radius-s);
  background: transparent;
  color: var(--rt-color-text-dim);
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.stats-tab-btn {
  flex: 1;
  height: 32px;
  padding: 0 10px;
  border: none;
  border-bottom: 2px solid transparent;
  background: transparent;
  font-family: inherit;
  font-size: var(--rt-font-xs);
  font-weight: 600;
  color: var(--rt-color-text-dim);
  cursor: pointer;
  transition: color 0.15s, border-color 0.15s, background-color 0.15s;
}

/* -----------------------------------------------------------------------------
 * Overlays & Tooltips
 * ----------------------------------------------------------------------------- */

/* (none) */

/* -----------------------------------------------------------------------------
 * States & Modifiers
 * ----------------------------------------------------------------------------- */

.stats-close-btn:hover {
  background-color: var(--rt-color-surface);
  color: var(--rt-color-text);
}

.stats-tab-btn:hover:not(.is-active) {
  background-color: var(--rt-color-surface-header);
  color: var(--rt-color-text);
}

.stats-tab-btn.is-active {
  border-bottom-color: var(--rt-color-primary);
  color: var(--rt-color-primary);
}
</style>
