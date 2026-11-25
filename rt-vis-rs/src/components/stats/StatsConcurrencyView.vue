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
import { useSimulation } from '../../composables/useSimulation';
import { useExecutionLog } from '../../composables/useExecutionLog';
import { formatRatioAsPercent, formatUsAsMs } from '../../utils/format';
import type { ConcurrencyStats } from '../../types/simulation';
import InfoTooltip from './InfoTooltip.vue';
import StatsProgressBar from './StatsProgressBar.vue';

// -----------------------------------------------------------------------------
// Global State & Composables

const { mode } = useApp();
const { plannedConcurrencyStatsCreateMode } = useSimulation();
const { logSummaryAnalyzeMode } = useExecutionLog();

// -----------------------------------------------------------------------------
// Props & Emits

const emit = defineEmits<{
  (e: 'jump', timeUs: number): void;
}>();

// -----------------------------------------------------------------------------
// Types & Interfaces

// (none)

// -----------------------------------------------------------------------------
// Constants & Layout

// (none)

// -----------------------------------------------------------------------------
// Local State & Computed

const stats = computed<ConcurrencyStats | null>(() => {
  switch (mode.value) {
    case 'Create':
      return plannedConcurrencyStatsCreateMode.value;
    case 'Analyze':
      return logSummaryAnalyzeMode.value?.stats.concurrency ?? null;
  }
});

const scopeLabel = computed<string>(() => {
  switch (mode.value) {
    case 'Create':
      return 'Steady-state 1 period (excl. warmup)';
    case 'Analyze':
      return 'Full log duration';
  }
});

const activeRatioText = computed<string>(() => {
  if (!stats.value || stats.value.total_duration_us === 0) {
    return '-';
  }
  return formatRatioAsPercent(stats.value.active_ratio, 2);
});

// -----------------------------------------------------------------------------
// Methods & Logic

/**
 * Formats a discrete concurrency level count into a human-readable label.
 */
const formatLevelLabel = (level: number): string => {
  if (level === 0) {
    return '0 (Idle)';
  } else if (level === 1) {
    return '1 client';
  } else {
    return `${level} clients`;
  }
};

/**
 * Resolves the bar color modifier class for a given concurrency level.
 */
const getBarToneClass = (level: number): string => {
  if (level === 0) {
    return 'is-idle';
  } else if (stats.value && level === stats.value.peak_concurrency) {
    return 'is-peak';
  } else {
    return 'is-active';
  }
};

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
  <div v-if="!stats" class="stats-empty-state">
    No concurrency statistics available for the current state.
  </div>

  <div v-else class="stats-scroll-body">
    <div class="scope-banner">
      <span>{{ scopeLabel }}</span>
      <span class="scope-window">{{ formatUsAsMs(stats.total_duration_us) }}</span>
    </div>

    <!-- Active Time Ratio -->
    <section class="stats-card">
      <div class="stats-section-header">
        <div class="stats-section-title">
          <span>Active Time Ratio</span>
          <InfoTooltip title="Active Time Ratio"
            description="Percentage of the window with at least one running client (running_count >= 1)."
            tip="Low active ratio indicates idle headroom; very high ratio leaves little margin for workload bursts." />
        </div>
        <span class="stats-primary-value">{{ activeRatioText }}</span>
      </div>
      <StatsProgressBar :ratio="stats.active_ratio" tone="normal" />
      <div class="stats-subtext">
        Active: {{ formatUsAsMs(stats.active_duration_us) }} / {{ formatUsAsMs(stats.total_duration_us) }}
      </div>
    </section>

    <!-- Parallelism Summary -->
    <section class="stats-card">
      <div class="stats-section-header">
        <div class="stats-section-title">
          <span>Parallelism</span>
          <InfoTooltip title="Parallelism Metrics"
            description="Active Avg Concurrency is time-weighted over active periods only (excluding idle). Peak Concurrency is the maximum simultaneous clients."
            tip="Shift client offsets or start_cycle to flatten unnecessary concurrency peaks." />
        </div>
      </div>

      <div class="summary-row">
        <span class="summary-label">Active Avg Concurrency</span>
        <span class="summary-value">{{ stats.active_avg_concurrency.toFixed(2) }}</span>
      </div>

      <div class="summary-row">
        <span class="summary-label">Peak Concurrency</span>
        <div class="peak-actions">
          <span class="summary-value">{{ stats.peak_concurrency }}</span>
          <template v-if="mode === 'Analyze' && stats.peak_time_us !== null">
            <span class="stats-subtext">at {{ formatUsAsMs(stats.peak_time_us) }}</span>
            <button type="button" class="stats-jump-btn" title="Jump to peak concurrency timestamp"
              @click="emit('jump', stats.peak_time_us)">
              Jump
            </button>
          </template>
        </div>
      </div>
    </section>

    <!-- Concurrency Level Distribution -->
    <section class="stats-card">
      <div class="stats-section-header">
        <div class="stats-section-title">
          <span>Concurrency Distribution</span>
          <InfoTooltip title="Concurrency Distribution"
            description="Time share spent at each discrete concurrency level." />
        </div>
      </div>

      <div v-if="stats.distribution.length === 0" class="stats-subtext">
        No distribution samples recorded.
      </div>

      <div v-else class="dist-list">
        <div v-for="item in stats.distribution" :key="item.level" class="dist-row"
          :title="`${formatLevelLabel(item.level)}: ${formatUsAsMs(item.duration_us)} (${formatRatioAsPercent(item.ratio, 2)})`">
          <span class="dist-label">{{ formatLevelLabel(item.level) }}</span>
          <div class="dist-bar-track">
            <div class="dist-bar-fill" :class="getBarToneClass(item.level)"
              :style="{ width: `${Math.min(100, Math.max(0, item.ratio * 100)).toFixed(2)}%` }" />
          </div>
          <span class="dist-pct">{{ formatRatioAsPercent(item.ratio, 1) }}</span>
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

.scope-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 6px 10px;
  border: var(--rt-border-main);
  border-radius: var(--rt-radius-s);
  background-color: var(--rt-color-surface);
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
}

.dist-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

/* -----------------------------------------------------------------------------
 * Components & Elements
 * ----------------------------------------------------------------------------- */

.scope-window {
  font-family: var(--rt-font-mono);
  font-weight: 600;
  color: var(--rt-color-text);
  font-variant-numeric: tabular-nums;
}

.summary-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.summary-label {
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
}

.summary-value {
  font-family: var(--rt-font-mono);
  font-size: var(--rt-font-s);
  font-weight: 700;
  color: var(--rt-color-text);
  font-variant-numeric: tabular-nums;
}

.peak-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.dist-row {
  display: grid;
  grid-template-columns: 72px 1fr 52px;
  align-items: center;
  gap: 8px;
}

.dist-label {
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
  white-space: nowrap;
}

.dist-bar-track {
  height: 8px;
  overflow: hidden;
  border-radius: 4px;
  background-color: var(--rt-color-surface-header);
}

.dist-bar-fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.2s ease, background-color 0.2s ease;
}

.dist-pct {
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

.dist-bar-fill.is-idle {
  background-color: var(--rt-color-outline);
}

.dist-bar-fill.is-active {
  background-color: var(--rt-color-primary);
}

.dist-bar-fill.is-peak {
  background-color: var(--rt-color-accent);
}
</style>
