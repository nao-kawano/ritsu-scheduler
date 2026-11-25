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

// -----------------------------------------------------------------------------
// Global State & Composables

// (none)

// -----------------------------------------------------------------------------
// Props & Emits

const props = defineProps<{
  title: string;
  description: string;
  tip?: string;
}>();

// -----------------------------------------------------------------------------
// Types & Interfaces

// (none)

// -----------------------------------------------------------------------------
// Constants & Layout

const TOOLTIP_MAX_WIDTH = 280;
const VIEWPORT_MARGIN = 8;
const TRIGGER_OFFSET = 6;

// -----------------------------------------------------------------------------
// Local State & Computed

const triggerEl = ref<HTMLElement | null>(null);
const isVisible = ref<boolean>(false);
const anchorPos = ref<{ left: number; top: number; flipY: boolean }>({
  left: 0,
  top: 0,
  flipY: false
});

/**
 * Positions the floating tooltip and flips it upward via CSS transform without DOM re-measurement.
 */
const floatingStyle = computed(() => {
  return {
    left: `${anchorPos.value.left}px`,
    top: `${anchorPos.value.top}px`,
    transform: anchorPos.value.flipY ? 'translateY(-100%)' : 'translateY(0)'
  };
});

// -----------------------------------------------------------------------------
// Methods & Logic

/**
 * Computes viewport-clamped tooltip coordinates with vertical flip fallback.
 */
const updatePosition = (): void => {
  if (!triggerEl.value) {
    return;
  }

  const rect = triggerEl.value.getBoundingClientRect();
  const estimatedHeight = props.tip ? 120 : 80;

  // Clamp horizontal coordinate inside viewport bounds
  const maxLeft = Math.max(VIEWPORT_MARGIN, window.innerWidth - TOOLTIP_MAX_WIDTH - VIEWPORT_MARGIN);
  const left = Math.max(VIEWPORT_MARGIN, Math.min(maxLeft, rect.left));

  // Flip above trigger when space below is constrained and space above is larger
  const hasSpaceBelow = rect.bottom + TRIGGER_OFFSET + estimatedHeight + VIEWPORT_MARGIN <= window.innerHeight;
  const hasSpaceAbove = rect.top - TRIGGER_OFFSET - estimatedHeight >= VIEWPORT_MARGIN;
  const isLowerHalf = rect.top >= window.innerHeight / 2;
  const flipY = !hasSpaceBelow && (hasSpaceAbove || isLowerHalf);

  // Clamp vertical anchor coordinate within viewport bounds for either placement direction
  const maxTop = Math.max(VIEWPORT_MARGIN, window.innerHeight - estimatedHeight - VIEWPORT_MARGIN);
  const top = flipY
    ? Math.max(estimatedHeight + VIEWPORT_MARGIN, rect.top - TRIGGER_OFFSET)
    : Math.min(maxTop, rect.bottom + TRIGGER_OFFSET);

  anchorPos.value = { left, top, flipY };
};

// -----------------------------------------------------------------------------
// Event Handlers

/**
 * Computes tooltip placement and reveals the floating tooltip.
 */
const onShow = (): void => {
  updatePosition();
  isVisible.value = true;
};

/**
 * Hides the floating tooltip.
 */
const onHide = (): void => {
  isVisible.value = false;
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
  <button ref="triggerEl" type="button" class="info-tooltip-trigger" :aria-label="`About ${title}`" @mouseenter="onShow"
    @mouseleave="onHide" @focus="onShow" @blur="onHide">
    ?
  </button>

  <Teleport to=".app-container">
    <div v-if="isVisible" class="rt-tooltip info-tooltip-floating" :style="floatingStyle" role="tooltip">
      <div class="info-tooltip-header">{{ title }}</div>
      <div class="info-tooltip-desc">{{ description }}</div>
      <div v-if="tip" class="info-tooltip-tip">
        <span class="info-tooltip-tip-label">Tip:</span>
        {{ tip }}
      </div>
    </div>
  </Teleport>
</template>

<!-- ========================================================================== -->
<!-- Style Section                                                              -->
<!-- ========================================================================== -->
<style scoped>
/* -----------------------------------------------------------------------------
 * Layout & Containers
 * ----------------------------------------------------------------------------- */

/* (none) */

/* -----------------------------------------------------------------------------
 * Components & Elements
 * ----------------------------------------------------------------------------- */

.info-tooltip-trigger {
  display: inline-flex;
  flex-shrink: 0;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  padding: 0;
  border: 1px solid var(--rt-color-border);
  border-radius: 50%;
  background: transparent;
  font-family: inherit;
  font-size: 10px;
  font-weight: 700;
  line-height: 1;
  color: var(--rt-color-text-dim);
  cursor: help;
  transition: border-color 0.15s, color 0.15s, background-color 0.15s;
}

/* -----------------------------------------------------------------------------
 * Overlays & Tooltips
 * ----------------------------------------------------------------------------- */

.info-tooltip-floating {
  position: fixed;
  z-index: 200;
  max-width: v-bind('TOOLTIP_MAX_WIDTH + "px"');
  padding: 8px 10px;
  font-size: var(--rt-font-xs);
  line-height: 1.45;
}

.info-tooltip-header {
  margin-bottom: 4px;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--rt-color-border);
  font-weight: 600;
  color: var(--rt-color-text);
}

.info-tooltip-desc {
  color: var(--rt-color-text);
}

.info-tooltip-tip {
  margin-top: 5px;
  color: var(--rt-color-text-dim);
}

.info-tooltip-tip-label {
  font-weight: 700;
  color: var(--rt-color-primary);
}

/* -----------------------------------------------------------------------------
 * States & Modifiers
 * ----------------------------------------------------------------------------- */

.info-tooltip-trigger:hover,
.info-tooltip-trigger:focus-visible {
  outline: none;
  border-color: var(--rt-color-primary);
  background-color: var(--rt-color-primary-container);
  color: var(--rt-color-on-primary-container);
}
</style>
