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

// (none)

// -----------------------------------------------------------------------------
// Global State & Composables

// (none)

// -----------------------------------------------------------------------------
// Props & Emits

withDefaults(
  defineProps<{
    name: 'stats' | 'anomaly' | 'jump' | 'close';
    size?: number;
  }>(),
  {
    size: 14
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

// (none)

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
  <svg v-if="name === 'stats'" class="app-icon" viewBox="0 0 16 16" :width="size" :height="size" fill="currentColor"
    aria-hidden="true">
    <rect x="2" y="8" width="3" height="6" rx="0.5" />
    <rect x="6.5" y="4" width="3" height="10" rx="0.5" />
    <rect x="11" y="6" width="3" height="8" rx="0.5" />
  </svg>

  <svg v-else-if="name === 'anomaly'" class="app-icon" viewBox="0 0 16 16" :width="size" :height="size" fill="none"
    stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M8 2.2L14.2 13.2H1.8L8 2.2Z" />
    <line x1="8" y1="6.4" x2="8" y2="9.6" />
    <circle cx="8" cy="11.6" r="0.6" fill="currentColor" stroke="none" />
  </svg>

  <svg v-else-if="name === 'jump'" class="app-icon" viewBox="0 0 16 16" :width="size" :height="size" fill="none"
    stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M5 11L11 5" />
    <path d="M6 5h5v5" />
  </svg>

  <svg v-else class="app-icon" viewBox="0 0 16 16" :width="size" :height="size" fill="none" stroke="currentColor"
    stroke-width="2" stroke-linecap="round" aria-hidden="true">
    <line x1="4" y1="4" x2="12" y2="12" />
    <line x1="12" y1="4" x2="4" y2="12" />
  </svg>
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

.app-icon {
  display: block;
  flex-shrink: 0;
}

/* -----------------------------------------------------------------------------
 * Overlays & Tooltips
 * ----------------------------------------------------------------------------- */

/* (none) */

/* -----------------------------------------------------------------------------
 * States & Modifiers
 * ----------------------------------------------------------------------------- */

/* (none) */
</style>
