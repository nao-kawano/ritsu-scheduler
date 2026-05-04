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
  padding: 0.75rem 1rem;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
  display: flex;
  flex-direction: column;
  gap: 4px;
  pointer-events: auto;
}

.error-title {
  font-weight: bold;
  font-size: 0.9rem;
  display: flex;
  align-items: center;
  gap: 6px;
}

.error-detail {
  font-size: 0.8rem;
  opacity: 0.9;
  word-break: break-all;
}

/* --- Transition Effects --- */
.fade-enter-active,
.fade-leave-active {
  transition: all 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}
</style>
