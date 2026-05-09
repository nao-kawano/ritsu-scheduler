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
  max-width: 400px;
  background-color: #ff4d4f;
  color: white;
  padding: var(--rt-spacing-s) var(--rt-spacing-m);
  border-radius: var(--rt-radius-m);
  box-shadow: var(--rt-shadow-pop);
  display: flex;
  flex-direction: column;
  gap: var(--rt-spacing-xs);
  pointer-events: auto;
}

.error-title {
  font-weight: bold;
  font-size: var(--rt-font-m);
  display: flex;
  align-items: center;
  gap: var(--rt-spacing-s);
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
