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
import type { ProgressSegment, ProgressTone } from '../../types/stats';

// -----------------------------------------------------------------------------
// Global State & Composables

// (none)

// -----------------------------------------------------------------------------
// Props & Emits

const props = withDefaults(
  defineProps<{
    ratio?: number;
    tone?: ProgressTone;
    segments?: ProgressSegment[];
  }>(),
  {
    ratio: 0,
    tone: 'none',
    segments: undefined
  }
);

// -----------------------------------------------------------------------------
// Types & Interfaces

// (none)

// -----------------------------------------------------------------------------
// Constants & Layout

// (none)

// -----------------------------------------------------------------------------
// Local State & Computed

/**
 * Normalizes multi-segment or single-ratio props into clamped [0, 1] visible segments.
 */
const clampedSegments = computed<ProgressSegment[]>(() => {
  if (props.segments && props.segments.length > 0) {
    return props.segments
      .map(seg => ({
        tone: seg.tone,
        ratio: clampRatio(seg.ratio)
      }))
      .filter(seg => seg.ratio > 0);
  } else {
    const clamped = clampRatio(props.ratio);
    return clamped > 0 ? [{ ratio: clamped, tone: props.tone }] : [];
  }
});

// -----------------------------------------------------------------------------
// Methods & Logic

/**
 * Clamps a progress ratio to [0, 1], treating non-finite values as 0.
 */
const clampRatio = (ratio: number): number => {
  return Number.isFinite(ratio) ? Math.min(1, Math.max(0, ratio)) : 0;
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
  <div class="progress-track">
    <div v-for="(seg, idx) in clampedSegments" :key="idx" class="progress-fill" :class="`tone-${seg.tone}`"
      :style="{ width: `${(seg.ratio * 100).toFixed(3)}%` }" />
  </div>
</template>

<!-- ========================================================================== -->
<!-- Style Section                                                              -->
<!-- ========================================================================== -->
<style scoped>
/* -----------------------------------------------------------------------------
 * Layout & Containers
 * ----------------------------------------------------------------------------- */

.progress-track {
  display: flex;
  width: 100%;
  height: 6px;
  overflow: hidden;
  border-radius: 3px;
  background-color: var(--rt-color-surface-header);
}

/* -----------------------------------------------------------------------------
 * Components & Elements
 * ----------------------------------------------------------------------------- */

.progress-fill {
  height: 100%;
  transition: width 0.2s ease, background-color 0.2s ease;
}

/* -----------------------------------------------------------------------------
 * Overlays & Tooltips
 * ----------------------------------------------------------------------------- */

/* (none) */

/* -----------------------------------------------------------------------------
 * States & Modifiers
 * ----------------------------------------------------------------------------- */

.progress-fill.tone-none {
  background-color: transparent;
}

.progress-fill.tone-normal {
  background-color: var(--rt-color-status-normal);
}

.progress-fill.tone-warning {
  background-color: var(--rt-color-warning);
}

.progress-fill.tone-error {
  background-color: var(--rt-color-error);
}
</style>
