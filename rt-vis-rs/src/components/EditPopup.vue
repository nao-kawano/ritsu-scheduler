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
          <label>Name</label>
          <input type="text" v-model="draft.display_name" maxlength="20" autofocus
            @input="draft.display_name = draft.display_name.replace(/[^a-zA-Z0-9_-]/g, '')" placeholder="e.g. Camera" />

          <label>CID</label>
          <input type="number" v-model="draft.client_id" min="0" required />

          <label>Cycle</label>
          <input type="number" v-model="draft.cycle" min="1" required />

          <label>Offset</label>
          <input type="number" v-model="draft.cycle_offset" min="0" required />

          <label>Duration (ms)</label>
          <input type="number" v-model="draft.expected_duration_ms" min="0" required />

          <label>Depends</label>
          <input type="text" v-model="dependsStr" placeholder="e.g. 10, 20" />
        </div>

        <div class="dialog-footer">
          <button type="button" class="danger" :class="{ confirming: isConfirmingDelete }" @click="onDelete"
            @mouseleave="resetDeleteConfirm">
            {{ isConfirmingDelete ? 'Confirm Delete' : 'Delete' }}
          </button>
          <div style="flex: 1"></div>
          <button type="button" @click="onCancel">Cancel</button>
          <button type="submit" class="primary">Apply</button>
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
 * We use a nested .dialog-content for styling to control padding reliably.
 */
.popup-dialog {
  border: none;
  padding: 0;
  border-radius: 12px;
  background: transparent;
  box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
}

/**
 * The ::backdrop provides the Dim/Overlay effect.
 * Managed automatically by the browser when showModal() is called.
 */
.popup-dialog::backdrop {
  background: rgba(0, 0, 0, 0.5);
}

.dialog-content {
  background: var(--pane-bg);
  padding: 2rem;
  width: 400px;
  color: var(--text-main);
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
}

.dialog-content h3 {
  margin-top: 0;
}

/* ==========================================================================
   Form and Inputs
   ========================================================================== */

.dialog-body {
  display: grid;
  grid-template-columns: 1fr 2fr;
  gap: 1rem;
  align-items: center;
  margin-top: 1.5rem;
}

.dialog-body input {
  width: 100%;
  padding: 8px 12px;
  border-radius: 4px;
  border: 1px solid var(--border-color);
  background: var(--bg-color);
  color: var(--text-main);
  font-weight: 500;
  font-family: inherit;
  transition: border-color 0.2s, box-shadow 0.2s;
}

/* Focused state for high-signal feedback */
.dialog-body input:focus {
  outline: none;
  border-color: var(--primary-color);
  box-shadow: 0 0 0 3px rgba(57, 108, 216, 0.2);
}

/* HTML5 Validation Feedback (via required/min attributes) */
.dialog-body input:invalid {
  border-color: #f44336;
  box-shadow: 0 0 0 3px rgba(244, 67, 54, 0.2);
}

/* ==========================================================================
   Action Footer
   ========================================================================== */

.dialog-footer {
  margin-top: 2rem;
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 1rem;
}

.dialog-footer button {
  background: transparent;
  color: var(--text-main);
  border: 1px solid var(--border-color);
  padding: 0 1.2rem;
  height: 38px;
  border-radius: 6px;
  font-weight: bold;
  cursor: pointer;
  transition: background-color 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s, transform 0.2s;
}

.dialog-footer button:hover {
  background: var(--border-color);
}

/* --- Contextual Button Styles --- */

button.primary {
  background: var(--primary-color);
  color: white;
  border: none;
}

button.danger {
  color: #f44336;
  border-color: #f44336;
}

button.danger:hover {
  background: rgba(244, 67, 54, 0.1);
}

/* 2-step confirmation active state */
button.danger.confirming {
  background: #f44336;
  color: white;
  border-color: #f44336;
  box-shadow: 0 4px 12px rgba(244, 67, 54, 0.3);
  transform: scale(1.05);
}
</style>
