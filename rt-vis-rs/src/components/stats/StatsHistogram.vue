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

import { computed, ref } from 'vue';
import { formatRatioAsPercent, formatUsAsMs, formatUsAsMsInt } from '../../utils/format';
import type { Histogram } from '../../types/analyze';

// -----------------------------------------------------------------------------
// Global State & Composables

// (none)

// -----------------------------------------------------------------------------
// Props & Emits

const props = withDefaults(
  defineProps<{
    histogram: Histogram;
    mode: 'duration' | 'jitter';
    thresholdUs?: number | null;
  }>(),
  {
    thresholdUs: null
  }
);

// -----------------------------------------------------------------------------
// Types & Interfaces

type DisplayBarKind = 'underflow' | 'bin' | 'overflow';

interface RawBarEntry {
  key: string;
  kind: DisplayBarKind;
  startUs: number;
  endUs: number;
  count: number;
  label: string;
  fill: string;
}

interface DisplayBar extends RawBarEntry {
  x: number;
  y: number;
  width: number;
  height: number;
  slotX: number;
  slotWidth: number;
}

// -----------------------------------------------------------------------------
// Constants & Layout

const SVG_WIDTH = 320;
const SVG_HEIGHT = 96;
const PLOT_TOP = 6;
const PLOT_BOTTOM = 90;
const PLOT_HEIGHT = PLOT_BOTTOM - PLOT_TOP;
const MIN_NONZERO_BAR_HEIGHT = 3;

// -----------------------------------------------------------------------------
// Local State & Computed

const hoveredIndex = ref<number | null>(null);

/**
 * Upper bound of regular histogram bins, falling back to a single step when empty.
 */
const upperBoundUs = computed<number>(() => {
  const bins = props.histogram.bins;
  if (bins.length === 0) {
    return props.histogram.origin_us + props.histogram.step_us;
  }
  return bins[bins.length - 1].end_us;
});

/**
 * Flattens optional underflow tail, regular bins, and optional overflow tail into bar descriptors.
 */
const rawBarEntries = computed<RawBarEntry[]>(() => {
  const hist = props.histogram;
  const entries: RawBarEntry[] = [];

  // Prepend left tail bucket when extreme early/low outliers exist
  if (hist.underflow_count > 0) {
    entries.push({
      key: 'underflow',
      kind: 'underflow',
      startUs: Number.NEGATIVE_INFINITY,
      endUs: hist.origin_us,
      count: hist.underflow_count,
      label: `< ${formatUsAsMs(hist.origin_us)}`,
      fill: resolveBarColor('underflow', Number.NEGATIVE_INFINITY, hist.origin_us)
    });
  }

  // Emit uniformly spaced regular bins with semantic coloring
  for (let i = 0; i < hist.bins.length; i++) {
    const bin = hist.bins[i];
    entries.push({
      key: `bin-${i}`,
      kind: 'bin',
      startUs: bin.start_us,
      endUs: bin.end_us,
      count: bin.count,
      label: `[${formatUsAsMs(bin.start_us)}, ${formatUsAsMs(bin.end_us)}]`,
      fill: resolveBarColor('bin', bin.start_us, bin.end_us)
    });
  }

  // Append right tail bucket so extreme spikes do not compress the main distribution
  if (hist.overflow_count > 0) {
    const upper = upperBoundUs.value;
    entries.push({
      key: 'overflow',
      kind: 'overflow',
      startUs: upper,
      endUs: Number.POSITIVE_INFINITY,
      count: hist.overflow_count,
      label: `>= ${formatUsAsMs(upper)}`,
      fill: resolveBarColor('overflow', upper, Number.POSITIVE_INFINITY)
    });
  }

  return entries;
});

/**
 * Maps raw bar entries onto normalized SVG geometry while enforcing a minimum height for rare spikes.
 */
const displayBars = computed<DisplayBar[]>(() => {
  const entries = rawBarEntries.value;
  if (entries.length === 0) {
    return [];
  }

  // Derive uniform slot geometry and scale factor across all visible buckets
  const maxCount = Math.max(1, ...entries.map(e => e.count));
  const slotWidth = SVG_WIDTH / entries.length;
  const gap = Math.min(2, slotWidth * 0.15);
  const barWidth = Math.max(1, slotWidth - gap);

  // Project each entry to SVG rect coordinates with guaranteed minimum bar visibility when non-zero
  return entries.map((entry, idx) => {
    const slotX = idx * slotWidth;
    const x = slotX + gap / 2;
    const height = entry.count > 0 ? Math.max(MIN_NONZERO_BAR_HEIGHT, (entry.count / maxCount) * PLOT_HEIGHT) : 0;
    const y = PLOT_BOTTOM - height;

    return {
      ...entry,
      x,
      y,
      width: barWidth,
      height,
      slotX,
      slotWidth
    };
  });
});

/**
 * Projects expected-duration threshold onto SVG X coordinates across regular bins only.
 */
const thresholdLineX = computed<number | null>(() => {
  if (props.mode !== 'duration' || props.thresholdUs === null) {
    return null;
  }

  // Ensure regular bins and non-degenerate span exist before interpolation
  const bins = props.histogram.bins;
  const totalSlots = displayBars.value.length;
  if (bins.length === 0 || totalSlots === 0) {
    return null;
  }
  const span = upperBoundUs.value - props.histogram.origin_us;
  if (span <= 0) {
    return null;
  }

  // Interpolate linearly within regular bins, omitting line when threshold lies in tail buckets
  const ratio = (props.thresholdUs - props.histogram.origin_us) / span;
  if (ratio < 0 || ratio > 1) {
    return null;
  }

  // Offset past optional underflow slot so threshold aligns with regular bin boundaries
  const slotWidth = SVG_WIDTH / totalSlots;
  const regularStartSlot = props.histogram.underflow_count > 0 ? 1 : 0;
  return clampSvgX((regularStartSlot + ratio * bins.length) * slotWidth);
});

/**
 * Anchors the 0 ms reference line in jitter mode at the boundary between early and late bins.
 */
const zeroLineX = computed<number | null>(() => {
  if (props.mode !== 'jitter' || displayBars.value.length === 0) {
    return null;
  }

  // Prefer the left edge of the bin starting at 0 ms when positive bins exist
  const zeroIndex = displayBars.value.findIndex(b => b.kind === 'bin' && b.startUs === 0);
  if (zeroIndex >= 0) {
    return clampSvgX(displayBars.value[zeroIndex].slotX);
  }

  // Fall back to the right edge of the last negative bin when only early cycles exist
  for (let idx = displayBars.value.length - 1; idx >= 0; idx--) {
    const bar = displayBars.value[idx];
    if (bar.kind === 'bin' && bar.endUs <= 0) {
      return clampSvgX(bar.slotX + bar.slotWidth);
    }
  }

  return null;
});

/**
 * Determines whether jitter bins span both negative and positive sides around 0 ms.
 */
const hasNegAndPosBins = computed<boolean>(() => {
  if (props.mode !== 'jitter') {
    return false;
  }
  const hasNeg = props.histogram.bins.some(b => b.end_us <= 0);
  const hasPos = props.histogram.bins.some(b => b.start_us >= 0);
  return hasNeg && hasPos;
});

/**
 * Computes percentage position for the centered 0 ms axis tick, hiding it when too close to edge ticks.
 */
const zeroLabelPercent = computed<number | null>(() => {
  if (!hasNegAndPosBins.value || zeroLineX.value === null) {
    return null;
  }
  const pct = (zeroLineX.value / SVG_WIDTH) * 100;
  return pct >= 20 && pct <= 80 ? pct : null;
});

const leftTickLabel = computed<string>(() => {
  const originLabel = formatTickUs(props.histogram.origin_us);
  return props.histogram.underflow_count > 0 ? `< ${originLabel}` : originLabel;
});

const rightTickLabel = computed<string>(() => {
  const upperLabel = formatTickUs(upperBoundUs.value);
  return props.histogram.overflow_count > 0 ? `>= ${upperLabel}` : upperLabel;
});

/**
 * Formats the fixed single-line status bar with either the hovered bin breakdown or dataset summary.
 */
const hoveredStatusText = computed<string>(() => {
  const idx = hoveredIndex.value;
  if (idx !== null && idx >= 0 && idx < displayBars.value.length) {
    const bar = displayBars.value[idx];
    const ratio = props.histogram.total_count > 0 ? bar.count / props.histogram.total_count : 0;
    return `${bar.label}: ${bar.count} (${formatRatioAsPercent(ratio, 1)})`;
  } else {
    return `N = ${props.histogram.total_count}  |  Bin: ${formatUsAsMs(props.histogram.step_us)}  (hover bar)`;
  }
});

// -----------------------------------------------------------------------------
// Methods & Logic

/**
 * Clamps a horizontal coordinate inside the SVG viewport by 1 px to prevent stroke clipping.
 */
const clampSvgX = (x: number): number => {
  return Math.min(SVG_WIDTH - 1, Math.max(1, x));
};

/**
 * Resolves semantic bar fill color for any histogram bucket (underflow, regular bin, or overflow).
 */
const resolveBarColor = (kind: DisplayBarKind, startUs: number, endUs: number): string => {
  switch (props.mode) {
    case 'jitter': {
      // Highlight extreme tail buckets in warning; separate early (<= 0) vs late (> 0) regular bins
      switch (kind) {
        case 'underflow':
        case 'overflow':
          return 'var(--rt-color-warning)';
        case 'bin':
          return endUs <= 0 ? 'var(--rt-color-accent)' : 'var(--rt-color-primary)';
      }
    }
    case 'duration': {
      // Without a configured threshold, render all bars in the default primary color
      const threshold = props.thresholdUs;
      if (threshold === null) {
        return 'var(--rt-color-primary)';
      }

      // Classify bucket position relative to threshold: within, exceeding, or straddling
      if (endUs <= threshold) {
        return 'var(--rt-color-primary)';
      } else if (startUs >= threshold) {
        return 'var(--rt-color-error)';
      } else {
        return 'var(--rt-color-warning)';
      }
    }
  }
};

/**
 * Formats an axis tick using sub-ms precision only when step size is sub-millisecond.
 */
const formatTickUs = (us: number): string => {
  return props.histogram.step_us < 1000 ? formatUsAsMs(us) : formatUsAsMsInt(us);
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
  <div class="histogram-container">
    <div class="histogram-status" :class="{ 'is-hovered': hoveredIndex !== null }" :title="hoveredStatusText">
      {{ hoveredStatusText }}
    </div>

    <div class="histogram-chart-wrap">
      <svg class="histogram-svg" :viewBox="`0 0 ${SVG_WIDTH} ${SVG_HEIGHT}`" preserveAspectRatio="none"
        @mouseleave="hoveredIndex = null">
        <!-- Baseline -->
        <line class="histogram-baseline" x1="0" :y1="PLOT_BOTTOM" :x2="SVG_WIDTH" :y2="PLOT_BOTTOM" />

        <!-- Histogram Bars -->
        <rect v-for="(bar, idx) in displayBars" :key="bar.key" class="histogram-bar"
          :class="{ 'is-hovered': hoveredIndex === idx }" :x="bar.x" :y="bar.y" :width="bar.width" :height="bar.height"
          :fill="bar.fill" rx="1" />

        <!-- Reference Lines (drawn last per SVG painter's order) -->
        <line v-if="zeroLineX !== null" class="histogram-ref-zero" :x1="zeroLineX" :y1="PLOT_TOP" :x2="zeroLineX"
          :y2="PLOT_BOTTOM" />
        <line v-if="thresholdLineX !== null" class="histogram-ref-threshold" :x1="thresholdLineX" :y1="PLOT_TOP"
          :x2="thresholdLineX" :y2="PLOT_BOTTOM" />

        <!-- Full-height Invisible Hit Targets -->
        <rect v-for="(bar, idx) in displayBars" :key="`${bar.key}-hit`" class="histogram-hit" :x="bar.slotX" y="0"
          :width="bar.slotWidth" :height="SVG_HEIGHT" @mouseenter="hoveredIndex = idx" />
      </svg>
    </div>

    <div class="histogram-axis">
      <span>{{ leftTickLabel }}</span>
      <span v-if="zeroLabelPercent !== null" class="histogram-axis-zero" :style="{ left: `${zeroLabelPercent}%` }">
        0 ms
      </span>
      <span>{{ rightTickLabel }}</span>
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

.histogram-container {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
}

.histogram-chart-wrap {
  width: 100%;
  height: v-bind('SVG_HEIGHT + "px"');
  padding: 2px 4px;
  border: var(--rt-border-main);
  border-radius: var(--rt-radius-s);
  background-color: var(--rt-color-surface);
}

/* -----------------------------------------------------------------------------
 * Components & Elements
 * ----------------------------------------------------------------------------- */

.histogram-status {
  height: 18px;
  overflow: hidden;
  font-family: var(--rt-font-mono);
  font-size: var(--rt-font-xs);
  line-height: 18px;
  color: var(--rt-color-text-dim);
  font-variant-numeric: tabular-nums;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.histogram-svg {
  display: block;
  width: 100%;
  height: 100%;
}

.histogram-baseline {
  stroke: var(--rt-color-border);
  stroke-width: 1;
}

.histogram-bar {
  opacity: 0.85;
  transition: y 0.2s ease, height 0.2s ease, fill 0.2s ease, opacity 0.12s ease;
}

.histogram-ref-zero {
  stroke: var(--rt-color-text-dim);
  stroke-width: 1.5;
  stroke-dasharray: 3 2;
  pointer-events: none;
}

.histogram-ref-threshold {
  stroke: var(--rt-color-error);
  stroke-width: 1.5;
  stroke-dasharray: 4 3;
  pointer-events: none;
}

.histogram-hit {
  fill: transparent;
  cursor: crosshair;
}

.histogram-axis {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-family: var(--rt-font-mono);
  font-size: 10px;
  color: var(--rt-color-text-dim);
  font-variant-numeric: tabular-nums;
}

.histogram-axis-zero {
  position: absolute;
  top: 0;
  transform: translateX(-50%);
  font-weight: 600;
  color: var(--rt-color-text);
  pointer-events: none;
}

/* -----------------------------------------------------------------------------
 * Overlays & Tooltips
 * ----------------------------------------------------------------------------- */

/* (none) */

/* -----------------------------------------------------------------------------
 * States & Modifiers
 * ----------------------------------------------------------------------------- */

.histogram-status.is-hovered {
  font-weight: 600;
  color: var(--rt-color-text);
}

.histogram-bar.is-hovered {
  opacity: 1;
  filter: brightness(1.15);
}
</style>
