<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useAppState } from "../composables/useAppState";
import type { ClientConfig } from "../types/config";

// --- State and Composables ---
const { selectedClientWrap, updateClient, deleteClient, closeEdit } = useAppState();

// -----------------------------------------------------------------------------
// Props and Emits
// (None)

// -----------------------------------------------------------------------------
// State, Computed, and Logic

// --- Dialog Management ---

const dialogRef = ref<HTMLDialogElement | null>(null);

// --- Edit State ---

// isConfirmingDelete: Toggle for the 2-step deletion confirmation
const isConfirmingDelete = ref(false);

/**
 * draft: Working copy of the configuration to avoid mutating the "Source of Truth"
 * prematurely. All edits happen on this local buffer.
 */
const draft = ref<ClientConfig | null>(null);

/**
 * dependsStr: Human-readable comma-separated string for dependency IDs.
 * Separated from the numeric array for easier user editing.
 */
const dependsStr = ref("");

// Initialize local buffer from the global selection on mount
if (selectedClientWrap.value) {
  draft.value = JSON.parse(JSON.stringify(selectedClientWrap.value.data));
  dependsStr.value = draft.value?.depends.join(", ") || "";
}

onMounted(() => {
  /**
   * Use the native showModal() to enable browser-level focus trapping
   * and the ::backdrop overlay for a professional modal experience.
   */
  dialogRef.value?.showModal();
});

// --- Action Handlers ---

/**
 * Close the dialog without saving any changes.
 * This simply resets the global selection state.
 */
const onCancel = () => {
  closeEdit();
};

/**
 * Validate and commit changes to the global state.
 * Includes parsing the dependency string back to a numeric array.
 */
const onApply = () => {
  if (!draft.value || !selectedClientWrap.value) return;

  // Convert "10, 20" string format back to [10, 20] array.
  // We ignore non-numeric entries to maintain data integrity.
  const parsedDepends = dependsStr.value
    .split(",")
    .map((s) => parseInt(s.trim()))
    .filter((n) => !isNaN(n));

  const updatedData: ClientConfig = {
    ...draft.value,
    depends: parsedDepends,
  };

  /**
   * Attempt to update the SSOT (Global State).
   * If the domain-level validation fails (e.g., CID conflict), it returns false
   * and we keep the dialog open so the user can correct the error.
   */
  const success = updateClient(selectedClientWrap.value.configId, updatedData);
  if (success) {
    closeEdit();
  }
};

/**
 * Perform a 2-step deletion for safety.
 * First click enters 'confirming' state, second click executes deletion.
 */
const onDelete = () => {
  if (!selectedClientWrap.value) return;

  if (!isConfirmingDelete.value) {
    isConfirmingDelete.value = true;
    return;
  }

  deleteClient(selectedClientWrap.value.configId);
  closeEdit();
};

const resetDeleteConfirm = () => {
  isConfirmingDelete.value = false;
};

// -----------------------------------------------------------------------------
// Expose
// (None)
</script>

<template>
  <!-- Standard dialog with @click.self for background clicking and @close for native ESC key support -->
  <dialog ref="dialogRef" class="popup-dialog" @click.self="onCancel" @close="onCancel">
    <div v-if="draft" class="dialog-content">
      <h3>Edit Process: CID {{ String(draft.client_id).padStart(3, '0') }}</h3>
      <form @submit.prevent="onApply">
        <div class="dialog-body">
          <label class="rt-input-label">Name</label>
          <input type="text" v-model="draft.display_name" maxlength="20" autofocus
            @input="draft.display_name = draft.display_name.replace(/[^a-zA-Z0-9_-]/g, '')" placeholder="e.g. Camera"
            class="rt-input" />

          <label class="rt-input-label">CID</label>
          <input type="number" v-model="draft.client_id" min="0" required class="rt-input" />

          <label class="rt-input-label">Cycle</label>
          <input type="number" v-model="draft.cycle" min="1" required class="rt-input" />

          <label class="rt-input-label">Offset</label>
          <input type="number" v-model="draft.cycle_offset" min="0" required class="rt-input" />

          <label class="rt-input-label">Duration (ms)</label>
          <input type="number" v-model="draft.expected_duration_ms" min="0" required class="rt-input" />

          <label class="rt-input-label">Depends</label>
          <input type="text" v-model="dependsStr" placeholder="e.g. 10, 20" class="rt-input" />
        </div>

        <div class="dialog-actions">
          <button type="button" class="rt-btn rt-btn-danger" :class="{ active: isConfirmingDelete }" @click="onDelete"
            @mouseleave="resetDeleteConfirm">
            {{ isConfirmingDelete ? 'Confirm Delete' : 'Delete' }}
          </button>
          <div style="flex: 1"></div>
          <button type="button" class="rt-btn rt-btn-secondary" @click="onCancel">Cancel</button>
          <button type="submit" class="rt-btn rt-btn-primary">Apply</button>
        </div>
      </form>
    </div>
  </dialog>
</template>

<style scoped>
/* ==========================================================================
   Dialog and Layout
   ========================================================================== */

/**
 * Standard <dialog> reset.
 */
.popup-dialog {
  padding: 0;
  background: transparent;
  border: none;
  border-radius: var(--rt-radius-l);
  box-shadow: var(--rt-bshadow-pop);
}

/**
 * The ::backdrop provides the Dim/Overlay effect.
 */
.popup-dialog::backdrop {
  background: rgba(0, 0, 0, 0.5);
}

.dialog-content {
  width: 440px;
  padding: 2rem;
  background: var(--rt-color-surface);
  color: var(--rt-color-text);
}

.dialog-content h3 {
  margin-top: 0;
  margin-bottom: 1.5rem;
  font-size: var(--rt-font-l);
  color: var(--rt-color-primary);
}

/* ==========================================================================
   Form and Inputs
   ========================================================================== */

.dialog-body {
  display: grid;
  grid-template-columns: 100px 1fr;
  align-items: center;
  gap: 1rem;
}

/* ==========================================================================
   Action Footer
   ========================================================================== */

.dialog-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 1rem;
  margin-top: 2rem;
}
</style>
