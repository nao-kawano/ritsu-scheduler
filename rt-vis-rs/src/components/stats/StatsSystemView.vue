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
import { useConfig } from '../../composables/useConfig';
import { useExecutionLog } from '../../composables/useExecutionLog';
import { useStatsPanel } from '../../composables/useStatsPanel';
import { formatCid, formatRatioAsPercent } from '../../utils/format';
import {
  EMPTY_ANOMALY_EVENT_COUNTS,
  getAnomalySeverity,
  getTotalAnomalyCount,
  toAnomalyEventCounts
} from '../../utils/stats';
import {
  ANOMALY_EVENT_LABELS,
  ANOMALY_EVENT_TYPES,
  type AnomalyEventType
} from '../../types/analyze';
import type { AnomalySeverity, ProgressTone } from '../../types/stats';
import InfoTooltip from './InfoTooltip.vue';
import StatsProgressBar from './StatsProgressBar.vue';
import StatsEventsList from './StatsEventsList.vue';

// -----------------------------------------------------------------------------
// Global State & Composables

const { configAnalyzeMode } = useConfig();
const { logSummaryAnalyzeMode } = useExecutionLog();
const { activeTab, openSystem, openClient } = useStatsPanel();

// -----------------------------------------------------------------------------
// Props & Emits

const emit = defineEmits<{
  (e: 'jump', timeUs: number, cid: number): void;
}>();

// -----------------------------------------------------------------------------
// Types & Interfaces

interface ClientHealthRow {
  cid: number;
  cidLabel: string;
  name: string;
  severity: AnomalySeverity;
  totalAnomalies: number;
  completionText: string;
}

// -----------------------------------------------------------------------------
// Constants & Layout

// (none)

// -----------------------------------------------------------------------------
// Local State & Computed

const systemStats = computed(() => logSummaryAnalyzeMode.value?.stats.system ?? null);
const clientStatsList = computed(() => logSummaryAnalyzeMode.value?.stats.clients ?? []);

/**
 * Maps configured client IDs to their display names for the health summary list.
 */
const cidToDisplayName = computed(() => {
  const map = new Map<number, string>();
  for (const c of configAnalyzeMode.client_configs) {
    map.set(c.data.client_id, c.data.display_name);
  }
  return map;
});

const systemEventCounts = computed<Readonly<Record<AnomalyEventType, number>>>(() => {
  const sys = systemStats.value;
  return sys ? toAnomalyEventCounts(sys) : EMPTY_ANOMALY_EVENT_COUNTS;
});

const cleanCycleRatioText = computed<string>(() => {
  const sys = systemStats.value;
  if (!sys || sys.total_cycles === 0) {
    return '-';
  }
  return formatRatioAsPercent(sys.clean_cycle_ratio, 2);
});

/**
 * Maps clean-cycle ratio onto severity tone (strict 100% normal, >= 95% warning, otherwise error).
 */
const cleanCycleBarTone = computed<ProgressTone>(() => {
  const sys = systemStats.value;
  if (!sys || sys.total_cycles === 0) {
    return 'none';
  }
  if (sys.clean_cycle_ratio >= 1) {
    return 'normal';
  } else if (sys.clean_cycle_ratio >= 0.95) {
    return 'warning';
  } else {
    return 'error';
  }
});

/**
 * Projects per-client statistics into clickable health summary rows, excluding incomplete runs from completion rate.
 */
const clientHealthRows = computed<ClientHealthRow[]>(() => {
  return clientStatsList.value.map(client => {
    const totalRuns = client.normal_count + client.overrun_count;
    return {
      cid: client.cid,
      cidLabel: formatCid(client.cid),
      name: cidToDisplayName.value.get(client.cid) || '',
      severity: getAnomalySeverity(client),
      totalAnomalies: getTotalAnomalyCount(client),
      completionText: totalRuns === 0 ? '-' : formatRatioAsPercent(client.normal_count / totalRuns, 1)
    };
  });
});

// -----------------------------------------------------------------------------
// Methods & Logic

// (none)

// -----------------------------------------------------------------------------
// Event Handlers

/**
 * Forwards timeline jump requests from the embedded anomaly events list.
 */
const onJump = (timeUs: number, cid: number): void => {
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
  <div v-if="!systemStats" class="stats-empty-state">
    Load a log file to inspect system statistics.
  </div>

  <!-- Events Tab -->
  <div v-else-if="activeTab === 'events'" class="stats-tab-fill">
    <StatsEventsList :cid="null" :event-counts="systemEventCounts" @jump="onJump" />
  </div>

  <!-- Stats Tab -->
  <div v-else-if="activeTab === 'stats'" class="stats-scroll-body">
    <!-- Clean Cycle Rate -->
    <section class="stats-card">
      <div class="stats-section-header">
        <div class="stats-section-title">
          <span>Clean Cycle Rate</span>
          <InfoTooltip title="Clean Cycle Rate"
            description="Percentage of cycles with zero critical anomalies (Overrun, Skip, Late, Error) across all clients."
            tip="Retransmit and Exit are excluded because recovered UDP retries and normal exits do not break deterministic execution; check Retransmit counts below or in the Events tab." />
        </div>
        <span class="stats-primary-value">{{ cleanCycleRatioText }}</span>
      </div>
      <StatsProgressBar :ratio="systemStats.clean_cycle_ratio" :tone="cleanCycleBarTone" />
      <div class="stats-subtext">
        {{ systemStats.clean_cycles }} / {{ systemStats.total_cycles }} cycles clean
      </div>
    </section>

    <!-- Anomaly Summary -->
    <section class="stats-card">
      <div class="stats-section-header">
        <div class="stats-section-title">
          <span>Anomaly Summary</span>
          <InfoTooltip title="Anomaly Summary"
            description="Total anomaly events across all clients. Click any category to open filtered Events."
            tip="Overrun is typically the root cause, while Skip is often downstream collateral. Retransmit indicates UDP packet loss or OS load." />
        </div>
      </div>
      <div class="anomaly-grid">
        <button v-for="type in ANOMALY_EVENT_TYPES" :key="type" type="button" class="anomaly-tile"
          @click="openSystem('events', [type])">
          <span class="stats-event-badge stats-event-fill" :class="`event-${type}`">
            {{ ANOMALY_EVENT_LABELS[type] }}
          </span>
          <span class="anomaly-count">{{ systemEventCounts[type] }}</span>
        </button>
      </div>
    </section>

    <!-- Client Health -->
    <section class="stats-card">
      <div class="stats-section-header">
        <div class="stats-section-title">
          <span>Client Health</span>
          <InfoTooltip title="Client Health"
            description="Per-client completion rate and anomaly summary. Click a client to open its detailed statistics." />
        </div>
        <span class="stats-subtext">{{ clientHealthRows.length }} clients</span>
      </div>

      <div v-if="clientHealthRows.length === 0" class="stats-subtext">
        No client statistics recorded.
      </div>

      <div v-else class="client-health-list">
        <button v-for="row in clientHealthRows" :key="row.cid" type="button" class="client-health-row"
          @click="openClient(row.cid, 'stats')">
          <div class="client-health-left">
            <span class="health-dot" :class="`is-${row.severity}`" />
            <span class="client-health-cid">{{ row.cidLabel }}</span>
            <span v-if="row.name" class="client-health-name" :title="row.name">
              {{ row.name }}
            </span>
          </div>
          <div class="client-health-right">
            <span v-if="row.totalAnomalies > 0" class="client-anomaly-pill" :class="`is-${row.severity}`">
              {{ row.totalAnomalies }}
            </span>
            <span class="client-completion-text">{{ row.completionText }}</span>
          </div>
        </button>
      </div>
    </section>
  </div>
</template>

<!-- ========================================================================== -->
<!-- Style Section                                                              -->
<!-- ========================================================================== -->
<style scoped>
/* -----------------------------------------------------------------------------
 * Layout & Containers
 * ----------------------------------------------------------------------------- */

.anomaly-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 6px;
}

.client-health-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

/* -----------------------------------------------------------------------------
 * Components & Elements
 * ----------------------------------------------------------------------------- */

.anomaly-tile {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  padding: 6px 8px;
  border: var(--rt-border-main);
  border-radius: var(--rt-radius-s);
  background-color: var(--rt-color-bg);
  font-family: inherit;
  cursor: pointer;
  transition: border-color 0.15s, background-color 0.15s;
}

.anomaly-count {
  font-family: var(--rt-font-mono);
  font-size: var(--rt-font-s);
  font-weight: 700;
  color: var(--rt-color-text);
  font-variant-numeric: tabular-nums;
}

.client-health-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  width: 100%;
  padding: 6px 8px;
  border: 1px solid transparent;
  border-radius: var(--rt-radius-s);
  background-color: var(--rt-color-bg);
  font-family: inherit;
  text-align: left;
  cursor: pointer;
  transition: border-color 0.15s, background-color 0.15s;
}

.client-health-left {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.health-dot {
  flex-shrink: 0;
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.client-health-cid {
  flex-shrink: 0;
  font-family: var(--rt-font-mono);
  font-size: var(--rt-font-xs);
  font-weight: 600;
  color: var(--rt-color-text);
  font-variant-numeric: tabular-nums;
}

.client-health-name {
  overflow: hidden;
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.client-health-right {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  gap: 6px;
}

.client-anomaly-pill {
  padding: 1px 5px;
  border-radius: 3px;
  font-family: var(--rt-font-mono);
  font-size: 10px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.client-completion-text {
  min-width: 52px;
  font-family: var(--rt-font-mono);
  font-size: var(--rt-font-xs);
  font-weight: 600;
  text-align: right;
  color: var(--rt-color-text);
  font-variant-numeric: tabular-nums;
}

/* -----------------------------------------------------------------------------
 * Overlays & Tooltips
 * ----------------------------------------------------------------------------- */

/* (none) */

/* -----------------------------------------------------------------------------
 * States & Modifiers
 * ----------------------------------------------------------------------------- */

.anomaly-tile:hover,
.client-health-row:hover {
  border-color: var(--rt-color-primary);
  background-color: var(--rt-color-surface-header);
}

.health-dot.is-none {
  background-color: var(--rt-color-status-normal);
}

.health-dot.is-warning {
  background-color: var(--rt-color-warning);
}

.health-dot.is-error {
  background-color: var(--rt-color-error);
}

.client-anomaly-pill.is-warning {
  background-color: var(--rt-color-warning-container);
  color: var(--rt-color-on-warning-container);
}

.client-anomaly-pill.is-error {
  background-color: var(--rt-color-error-container);
  color: var(--rt-color-on-error-container);
}
</style>
