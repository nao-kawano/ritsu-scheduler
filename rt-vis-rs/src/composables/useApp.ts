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

// =============================================================================
// Imports
// =============================================================================

import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { AppMode } from '../types/app';

// =============================================================================
// Types & Constants
// =============================================================================

// (none)

// =============================================================================
// Module State (Singleton)
// =============================================================================

// Shared reactive state for application core
const mode = ref<AppMode>('Create');
const appVersion = ref<string>('');

// =============================================================================
// Internal Helpers
// =============================================================================

/**
 * Fetch the application version string from the backend Tauri runtime.
 */
const loadAppVersion = async (): Promise<void> => {
  try {
    appVersion.value = await invoke<string>('get_app_version');
  } catch (e) {
    console.error('Failed to load app version:', e);
  }
};

// Fetch version on module initialization
loadAppVersion();

// =============================================================================
// Composable Implementation
// =============================================================================

export function useApp() {
  // ---------------------------------------------------------------------------
  // Dependencies & Inject

  // (none)

  // ---------------------------------------------------------------------------
  // Local State & Computed

  // (none)

  // ---------------------------------------------------------------------------
  // Methods & Actions

  // (none)

  // ---------------------------------------------------------------------------
  // Watchers & Reactive Triggers

  // (none)

  // ---------------------------------------------------------------------------
  // Lifecycle Hooks & Observers

  // (none)

  // ---------------------------------------------------------------------------
  // Public API (Return)

  return {
    mode,
    appVersion
  };
}
