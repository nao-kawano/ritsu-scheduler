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
<script setup lang="ts">
/**
 * GlobalError Component
 * Provides a floating notification for displaying error messages.
 */

// -----------------------------------------------------------------------------
// Props and Emits

defineProps<{
  message: string | null;
}>();

// -----------------------------------------------------------------------------
// State, Computed, and Logic

// -----------------------------------------------------------------------------
// Expose

</script>

<template>
  <Transition name="fade">
    <div v-if="message" class="global-error-card">
      <div class="error-title">
        <span class="warning-icon">⚠️</span>
        Error
      </div>
      <div class="error-detail">{{ message }}</div>
    </div>
  </Transition>
</template>

<style scoped>
.global-error-card {
  display: flex;
  flex-direction: column;
  gap: var(--rt-spacing-xs);
  max-width: 400px;
  padding: var(--rt-spacing-s) var(--rt-spacing-m);
  background-color: var(--rt-color-error);
  border-radius: var(--rt-radius-m);
  box-shadow: var(--rt-bshadow-pop);
  color: var(--rt-color-on-error);
  pointer-events: auto;
}

.error-title {
  display: flex;
  align-items: center;
  gap: var(--rt-spacing-s);
  font-size: var(--rt-font-m);
  font-weight: bold;
}

.error-detail {
  font-size: var(--rt-font-s);
  opacity: 0.9;
  word-break: break-all;
}

/* --- Transition Effects --- */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease, transform 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}
</style>
