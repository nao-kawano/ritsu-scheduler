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
import { useExecutionLog } from '../../composables/useExecutionLog';
import { useStatsPanel } from '../../composables/useStatsPanel';
import { formatDeltaUsAsMs, formatUsAsMs } from '../../utils/format';
import type { JitterStats } from '../../types/analyze';
import AppIcon from '../AppIcon.vue';
import InfoTooltip from './InfoTooltip.vue';
import StatsHistogram from './StatsHistogram.vue';

// -----------------------------------------------------------------------------
// Global State & Composables

const { logSummaryAnalyzeMode } = useExecutionLog();
const { activeTab } = useStatsPanel();

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

const jitter = computed<JitterStats | null>(() => {
  return logSummaryAnalyzeMode.value?.stats.jitter ?? null;
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
  <div v-if="!jitter" class="stats-empty-state">
    Cycle jitter requires at least 2 cycles (Cycle 0 is excluded).
  </div>

  <!-- Outliers Tab -->
  <div v-else-if="activeTab === 'outliers'" class="stats-tab-fill">
    <div class="outliers-notice">
      Showing Top {{ jitter.outliers.length }} cycles sorted by |jitter| descending
      (out of {{ jitter.sample_count }} sampled cycles).
    </div>

    <div class="outliers-list">
      <button v-for="(item, idx) in jitter.outliers" :key="item.cycle" type="button" class="outlier-row"
        @click="emit('jump', item.time_us)">
        <div class="outlier-row-top">
          <div class="outlier-left">
            <span class="outlier-rank">#{{ idx + 1 }}</span>
            <span class="outlier-cycle">Cycle #{{ item.cycle }}</span>
          </div>
          <div class="outlier-right">
            <span class="outlier-delta">{{ formatDeltaUsAsMs(item.jitter_us) }}</span>
            <AppIcon class="outlier-jump-icon" name="jump" :size="12" />
          </div>
        </div>
        <div class="outlier-row-bottom">
          Time: {{ formatUsAsMs(item.time_us) }}
        </div>
      </button>
    </div>
  </div>

  <!-- Stats Tab -->
  <div v-else-if="activeTab === 'stats'" class="stats-scroll-body">
    <!-- Signed Jitter -->
    <section class="stats-card">
      <div class="stats-section-header">
        <div class="stats-section-title">
          <span>Signed Jitter (Bias)</span>
          <InfoTooltip title="Signed Cycle Jitter"
            description="Cycle-to-cycle start timing delta relative to cycle_time_ms (Cycle 0 excluded). Positive = late, negative = early."
            tip="A consistently positive average indicates OS timer delay or host CPU contention." />
        </div>
        <span class="stats-subtext">N = {{ jitter.sample_count }}</span>
      </div>

      <div class="signed-row">
        <span class="stats-kv-label">Avg</span>
        <span class="stats-kv-value">{{ formatDeltaUsAsMs(jitter.avg_us) }}</span>
      </div>

      <div class="signed-row">
        <span class="stats-kv-label">Max</span>
        <div class="signed-actions">
          <span class="stats-kv-value">{{ formatDeltaUsAsMs(jitter.max_us) }}</span>
          <span class="stats-subtext">Cycle #{{ jitter.max_cycle }}</span>
          <button type="button" class="stats-jump-btn" title="Jump to maximum jitter cycle"
            @click="emit('jump', jitter.max_time_us)">
            Jump
          </button>
        </div>
      </div>

      <div class="signed-row">
        <span class="stats-kv-label">Min</span>
        <div class="signed-actions">
          <span class="stats-kv-value">{{ formatDeltaUsAsMs(jitter.min_us) }}</span>
          <span class="stats-subtext">Cycle #{{ jitter.min_cycle }}</span>
          <button type="button" class="stats-jump-btn" title="Jump to minimum jitter cycle"
            @click="emit('jump', jitter.min_time_us)">
            Jump
          </button>
        </div>
      </div>
    </section>

    <!-- Absolute Jitter -->
    <section class="stats-card">
      <div class="stats-section-header">
        <div class="stats-section-title">
          <span>Absolute |Jitter| (Spread)</span>
          <InfoTooltip title="Absolute Jitter Magnitude"
            description="Magnitude-only jitter metrics preventing early and late cycles from canceling out." />
        </div>
      </div>

      <div class="stats-kv-grid">
        <div class="stats-kv-item">
          <span class="stats-kv-label">Avg (MAE)</span>
          <span class="stats-kv-value">{{ formatUsAsMs(jitter.abs_avg_us) }}</span>
        </div>
        <div class="stats-kv-item">
          <span class="stats-kv-label">StdDev</span>
          <span class="stats-kv-value">{{ formatUsAsMs(jitter.abs_std_dev_us) }}</span>
        </div>
        <div class="stats-kv-item">
          <span class="stats-kv-label">P95</span>
          <span class="stats-kv-value">{{ formatUsAsMs(jitter.abs_p95_us) }}</span>
        </div>
        <div class="stats-kv-item">
          <span class="stats-kv-label">P99</span>
          <span class="stats-kv-value">{{ formatUsAsMs(jitter.abs_p99_us) }}</span>
        </div>
      </div>
    </section>

    <!-- Jitter Distribution Histogram -->
    <section class="stats-card">
      <div class="stats-section-header">
        <div class="stats-section-title">
          <span>Jitter Distribution</span>
          <InfoTooltip title="Jitter Distribution"
            description="Signed jitter histogram anchored at 0 ms. Extreme tails are grouped into underflow/overflow edge bins." />
        </div>
      </div>
      <StatsHistogram mode="jitter" :histogram="jitter.histogram" />
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

.outliers-list {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: none;
}

/* -----------------------------------------------------------------------------
 * Components & Elements
 * ----------------------------------------------------------------------------- */

.signed-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.signed-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

.outliers-notice {
  flex-shrink: 0;
  padding: 8px 12px;
  border-bottom: var(--rt-border-main);
  background-color: var(--rt-color-surface-header);
  font-size: var(--rt-font-xs);
  line-height: 1.35;
  color: var(--rt-color-text-dim);
}

.outlier-row {
  display: flex;
  flex-direction: column;
  gap: 3px;
  width: 100%;
  padding: 8px 12px;
  border: none;
  border-bottom: var(--rt-border-main);
  background-color: transparent;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
  transition: background-color 0.12s;
}

.outlier-row-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.outlier-left,
.outlier-right {
  display: flex;
  align-items: center;
  gap: 6px;
}

.outlier-rank {
  font-family: var(--rt-font-mono);
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
  font-variant-numeric: tabular-nums;
}

.outlier-cycle {
  font-size: var(--rt-font-s);
  font-weight: 600;
  color: var(--rt-color-text);
  font-variant-numeric: tabular-nums;
}

.outlier-delta {
  font-family: var(--rt-font-mono);
  font-size: var(--rt-font-s);
  font-weight: 700;
  color: var(--rt-color-text);
  font-variant-numeric: tabular-nums;
}

.outlier-jump-icon {
  color: var(--rt-color-text-dim);
  opacity: 0.6;
  transition: opacity 0.12s, color 0.12s;
}

.outlier-row-bottom {
  font-family: var(--rt-font-mono);
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
  font-variant-numeric: tabular-nums;
}

/* -----------------------------------------------------------------------------
 * Overlays & Tooltips
 * ----------------------------------------------------------------------------- */

/* (none) */

/* -----------------------------------------------------------------------------
 * States & Modifiers
 * ----------------------------------------------------------------------------- */

.outlier-row:hover {
  background-color: var(--rt-color-surface-header);
}

.outlier-row:hover .outlier-jump-icon {
  color: var(--rt-color-primary);
  opacity: 1;
}
</style>
