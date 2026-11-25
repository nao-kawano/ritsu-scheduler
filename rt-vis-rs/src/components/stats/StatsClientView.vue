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
import {
  formatCid,
  formatDeltaUsAsMs,
  formatRatioAsPercent,
  formatUsAsMs
} from '../../utils/format';
import { EMPTY_ANOMALY_EVENT_COUNTS, toAnomalyEventCounts } from '../../utils/stats';
import {
  ANOMALY_EVENT_LABELS,
  ANOMALY_EVENT_TYPES,
  type AnomalyEventType,
  type ClientStats
} from '../../types/analyze';
import type { ProgressSegment, ProgressTone } from '../../types/stats';
import InfoTooltip from './InfoTooltip.vue';
import StatsProgressBar from './StatsProgressBar.vue';
import StatsHistogram from './StatsHistogram.vue';
import StatsEventsList from './StatsEventsList.vue';

// -----------------------------------------------------------------------------
// Global State & Composables

const { configAnalyzeMode } = useConfig();
const { logSummaryAnalyzeMode, getEffectiveClientBudget } = useExecutionLog();
const { activeTab, openClient } = useStatsPanel();

// -----------------------------------------------------------------------------
// Props & Emits

const props = defineProps<{
  cid: number;
}>();

const emit = defineEmits<{
  (e: 'jump', timeUs: number, cid: number): void;
}>();

// -----------------------------------------------------------------------------
// Types & Interfaces

// (none)

// -----------------------------------------------------------------------------
// Constants & Layout

// (none)

// -----------------------------------------------------------------------------
// Local State & Computed

// --- Client Lookup & Events ---

const clientStats = computed<ClientStats | null>(() => {
  return logSummaryAnalyzeMode.value?.stats.clients.find(c => c.cid === props.cid) ?? null;
});

const clientEventCounts = computed<Readonly<Record<AnomalyEventType, number>>>(() => {
  const c = clientStats.value;
  return c ? toAnomalyEventCounts(c) : EMPTY_ANOMALY_EVENT_COUNTS;
});

// --- Execution Status ---

const totalCompletedRuns = computed<number>(() => {
  const c = clientStats.value;
  return c ? c.normal_count + c.overrun_count : 0;
});

const normalRatio = computed<number>(() => {
  if (!clientStats.value || totalCompletedRuns.value === 0) {
    return 0;
  }
  return clientStats.value.normal_count / totalCompletedRuns.value;
});

const overrunRatio = computed<number>(() => {
  if (!clientStats.value || totalCompletedRuns.value === 0) {
    return 0;
  }
  return clientStats.value.overrun_count / totalCompletedRuns.value;
});

/**
 * Splits completed runs into stacked normal and overrun progress bar segments.
 */
const completionSegments = computed<ProgressSegment[]>(() => {
  if (totalCompletedRuns.value === 0) {
    return [];
  }
  return [
    { ratio: normalRatio.value, tone: 'normal' },
    { ratio: overrunRatio.value, tone: 'error' }
  ];
});

const completionText = computed<string>(() => {
  if (totalCompletedRuns.value === 0) {
    return '-';
  }
  return formatRatioAsPercent(normalRatio.value, 2);
});

// --- Expected Budget ---

const effectiveBudget = computed(() => getEffectiveClientBudget(props.cid));

const coverageText = computed<string>(() => {
  const budget = effectiveBudget.value;
  if (!budget) {
    return '-';
  }
  return formatRatioAsPercent(budget.coverage_ratio, 2);
});

/**
 * Maps budget coverage onto severity tone (>= 99% normal, >= 90% warning, otherwise error).
 */
const coverageBarTone = computed<ProgressTone>(() => {
  const budget = effectiveBudget.value;
  if (!budget) {
    return 'none';
  }
  if (budget.coverage_ratio >= 0.99) {
    return 'normal';
  } else if (budget.coverage_ratio >= 0.9) {
    return 'warning';
  } else {
    return 'error';
  }
});

// --- Duty Cycle ---

/**
 * Computes average and WCET execution share relative to the client's scheduled period (interval * cycle_time_ms).
 */
const dutyCycle = computed<{ avgPct: string; wcetPct: string } | null>(() => {
  const duration = clientStats.value?.duration;
  const clientConfig = configAnalyzeMode.client_configs.find(c => c.data.client_id === props.cid);
  const cycleTimeMs = configAnalyzeMode.server_config.cycle_time_ms;
  if (!duration || !clientConfig || clientConfig.data.interval <= 0 || cycleTimeMs <= 0) {
    return null;
  }

  const periodUs = clientConfig.data.interval * cycleTimeMs * 1000;
  return {
    avgPct: formatRatioAsPercent(duration.avg_us / periodUs, 1),
    wcetPct: formatRatioAsPercent(duration.max_us / periodUs, 1)
  };
});

// -----------------------------------------------------------------------------
// Methods & Logic

// (none)

// -----------------------------------------------------------------------------
// Event Handlers

/**
 * Forwards timeline jump requests from the embedded anomaly events list.
 */
const onJump = (timeUs: number, targetCid: number): void => {
  emit('jump', timeUs, targetCid);
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
  <div v-if="!clientStats" class="stats-empty-state">
    No statistics available for {{ formatCid(cid) }}.
  </div>

  <!-- Events Tab -->
  <div v-else-if="activeTab === 'events'" class="stats-tab-fill">
    <StatsEventsList :cid="cid" :event-counts="clientEventCounts" @jump="onJump" />
  </div>

  <!-- Stats Tab -->
  <div v-else-if="activeTab === 'stats'" class="stats-scroll-body">
    <!-- Execution Status & Event Breakdown -->
    <section class="stats-card">
      <div class="stats-section-header">
        <div class="stats-section-title">
          <span>Execution Status</span>
          <InfoTooltip title="Execution Status"
            description="Completion rate across finalized executions: Normal / (Normal + Overrun)."
            tip="Incomplete executions cut off at log end are excluded from completion rates and duration statistics." />
        </div>
        <span class="stats-primary-value">{{ completionText }}</span>
      </div>

      <StatsProgressBar :segments="completionSegments" />

      <div class="status-counts-row">
        <span>Runs: <strong>{{ totalCompletedRuns }}</strong></span>
        <span>Normal: <strong>{{ clientStats.normal_count }}</strong></span>
        <span>
          Overrun:
          <strong>{{ clientStats.overrun_count }}</strong>
          <template v-if="totalCompletedRuns > 0"> ({{ formatRatioAsPercent(overrunRatio, 1) }})</template>
        </span>
      </div>

      <div v-if="clientStats.incomplete_count > 0" class="stats-subtext">
        Incomplete (excluded): {{ clientStats.incomplete_count }}
      </div>

      <div class="event-strip">
        <button v-for="type in ANOMALY_EVENT_TYPES" :key="type" type="button" class="event-strip-btn"
          @click="openClient(cid, 'events', [type])">
          <span class="stats-event-badge stats-event-fill" :class="`event-${type}`">
            {{ ANOMALY_EVENT_LABELS[type] }}
          </span>
          <span class="event-strip-count">{{ clientEventCounts[type] }}</span>
        </button>
        <div class="event-strip-lifecycle" title="Lifecycle exits (including normal shutdowns)">
          <span class="stats-event-badge stats-event-fill event-exit">Exit</span>
          <span class="event-strip-count">{{ clientStats.exit_count }}</span>
        </div>
      </div>
    </section>

    <!-- Expected Duration Alignment (Budget) -->
    <section class="stats-card">
      <div class="stats-section-header">
        <div class="stats-section-title">
          <span>Expected Budget</span>
          <InfoTooltip title="Expected Budget Alignment"
            description="Share of completed runs finishing within expected_duration_ms, with exceedances and safety margins."
            tip="If coverage is low or P99/WCET margin is negative, increase expected_duration_ms or reduce workload variance." />
        </div>
        <span class="stats-primary-value">{{ coverageText }}</span>
      </div>

      <template v-if="effectiveBudget">
        <StatsProgressBar :ratio="effectiveBudget.coverage_ratio" :tone="coverageBarTone" />
        <div class="stats-subtext">
          Target: {{ formatUsAsMs(effectiveBudget.expected_duration_us) }}
        </div>
        <div class="stats-kv-grid">
          <div class="stats-kv-item">
            <span class="stats-kv-label">Exceed Count</span>
            <span class="stats-kv-value">{{ effectiveBudget.exceed_count }}</span>
          </div>
          <div class="stats-kv-item">
            <!-- empty -->
          </div>
          <div class="stats-kv-item">
            <span class="stats-kv-label">Avg Exceed</span>
            <span class="stats-kv-value">
              {{ effectiveBudget.exceed_count > 0 ? formatUsAsMs(effectiveBudget.avg_exceed_us) : '-' }}
            </span>
          </div>
          <div class="stats-kv-item">
            <span class="stats-kv-label">Max Exceed</span>
            <span class="stats-kv-value">
              {{ effectiveBudget.exceed_count > 0 ? formatUsAsMs(effectiveBudget.max_exceed_us) : '-' }}
            </span>
          </div>
          <div class="stats-kv-item">
            <span class="stats-kv-label">P99 Margin</span>
            <span class="stats-kv-value" :class="{ 'is-negative': effectiveBudget.margin_p99_us < 0 }">
              {{ formatDeltaUsAsMs(effectiveBudget.margin_p99_us) }}
            </span>
          </div>
          <div class="stats-kv-item">
            <span class="stats-kv-label">WCET Margin</span>
            <span class="stats-kv-value" :class="{ 'is-negative': effectiveBudget.margin_wcet_us < 0 }">
              {{ formatDeltaUsAsMs(effectiveBudget.margin_wcet_us) }}
            </span>
          </div>
        </div>
      </template>
      <div v-else class="stats-subtext">No completed executions</div>
    </section>

    <!-- Execution Duration & Histogram -->
    <section class="stats-card">
      <div class="stats-section-header">
        <div class="stats-section-title">
          <span>Execution Duration</span>
          <InfoTooltip title="Execution Duration"
            description="Completed execution duration percentiles (Nearest-Rank) and population standard deviation."
            tip="Compare P99 against Max (WCET) to separate typical cost from rare tail spikes." />
        </div>
      </div>

      <template v-if="clientStats.duration">
        <StatsHistogram v-if="effectiveBudget" mode="duration" :histogram="effectiveBudget.duration_histogram"
          :threshold-us="effectiveBudget.expected_duration_us" />

        <div class="stats-kv-grid">
          <div class="stats-kv-item">
            <span class="stats-kv-label">Avg</span>
            <span class="stats-kv-value">{{ formatUsAsMs(clientStats.duration.avg_us) }}</span>
          </div>
          <div class="stats-kv-item">
            <span class="stats-kv-label">StdDev</span>
            <span class="stats-kv-value">{{ formatUsAsMs(clientStats.duration.std_dev_us) }}</span>
          </div>
          <div class="stats-kv-item">
            <span class="stats-kv-label">Min</span>
            <span class="stats-kv-value">{{ formatUsAsMs(clientStats.duration.min_us) }}</span>
          </div>
          <div class="stats-kv-item">
            <span class="stats-kv-label">P50</span>
            <span class="stats-kv-value">{{ formatUsAsMs(clientStats.duration.p50_us) }}</span>
          </div>
          <div class="stats-kv-item">
            <span class="stats-kv-label">P95</span>
            <span class="stats-kv-value">{{ formatUsAsMs(clientStats.duration.p95_us) }}</span>
          </div>
          <div class="stats-kv-item">
            <span class="stats-kv-label">P99</span>
            <span class="stats-kv-value">{{ formatUsAsMs(clientStats.duration.p99_us) }}</span>
          </div>
        </div>

        <div class="wcet-row">
          <div class="wcet-info">
            <span class="stats-kv-label">Max (WCET)</span>
            <span class="stats-kv-value">{{ formatUsAsMs(clientStats.duration.max_us) }}</span>
            <span class="stats-subtext">Cycle #{{ clientStats.duration.wcet_cycle }}</span>
          </div>
          <button type="button" class="stats-jump-btn" title="Jump to WCET occurrence on timeline"
            @click="emit('jump', clientStats.duration.wcet_time_us, cid)">
            Jump
          </button>
        </div>
      </template>
      <div v-else class="stats-subtext">No completed executions</div>
    </section>

    <!-- Duty Cycle (Auxiliary) -->
    <section v-if="dutyCycle" class="stats-card">
      <div class="stats-section-header">
        <div class="stats-section-title">
          <span>Duty Cycle</span>
          <InfoTooltip title="Duty Cycle"
            description="Execution time relative to the client's scheduled period (interval x cycle_time_ms)." />
        </div>
      </div>
      <div class="stats-kv-grid">
        <div class="stats-kv-item">
          <span class="stats-kv-label">Avg</span>
          <span class="stats-kv-value">{{ dutyCycle.avgPct }}</span>
        </div>
        <div class="stats-kv-item">
          <span class="stats-kv-label">WCET</span>
          <span class="stats-kv-value">{{ dutyCycle.wcetPct }}</span>
        </div>
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

.wcet-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding-top: 6px;
  border-top: 1px solid var(--rt-color-border);
}

.wcet-info {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 6px;
}

/* -----------------------------------------------------------------------------
 * Components & Elements
 * ----------------------------------------------------------------------------- */

.status-counts-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px 12px;
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
  font-variant-numeric: tabular-nums;
}

.status-counts-row strong {
  color: var(--rt-color-text);
}

.event-strip {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding-top: 2px;
}

.event-strip-btn,
.event-strip-lifecycle {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 6px;
  border: var(--rt-border-main);
  border-radius: var(--rt-radius-s);
  background-color: var(--rt-color-bg);
  font-family: inherit;
}

.event-strip-btn {
  cursor: pointer;
  transition: border-color 0.15s, background-color 0.15s;
}

.event-strip-lifecycle {
  opacity: 0.75;
}

.event-strip-count {
  font-family: var(--rt-font-mono);
  font-size: var(--rt-font-xs);
  font-weight: 600;
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

.event-strip-btn:hover {
  border-color: var(--rt-color-primary);
  background-color: var(--rt-color-surface-header);
}
</style>
