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

import { ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { PlannedExecution, PlannedMetricPoint, SimulationResult } from '../types/simulation';
import { useApp } from './useApp';
import { useConfig } from './useConfig';

// =============================================================================
// Types & Constants
// =============================================================================

const SIMULATION_DEBOUNCE_MS = 100;

// =============================================================================
// Module State (Singleton)
// =============================================================================

// Simulation results for Create Mode
const plannedExecutionsCreateMode = ref<PlannedExecution[]>([]);
const plannedMetricsCreateMode = ref<PlannedMetricPoint[]>([]);

// Simulation results for Analyze Mode
const plannedExecutionsAnalyzeMode = ref<PlannedExecution[]>([]);
const plannedMetricsAnalyzeMode = ref<PlannedMetricPoint[]>([]);

// Shared validation and error states
const configErrors = ref<Record<number, string[]>>({});
const simulationError = ref<string | null>(null);

// In-flight debounce timer
let simulateTimeout: ReturnType<typeof setTimeout> | null = null;

// =============================================================================
// Internal Helpers
// =============================================================================

// (none)

// =============================================================================
// Composable Implementation
// =============================================================================

export function useSimulation() {
  // ---------------------------------------------------------------------------
  // Dependencies & Inject

  const { mode } = useApp();
  const { configCreateMode, configAnalyzeMode, unwrapConfig } = useConfig();

  // ---------------------------------------------------------------------------
  // Local State & Computed

  // (none)

  // ---------------------------------------------------------------------------
  // Methods & Actions

  /**
   * Execute simulation on the Rust backend via IPC for the current active mode.
   */
  const simulatePlan = () => {
    if (simulateTimeout) clearTimeout(simulateTimeout);
    simulateTimeout = setTimeout(async () => {
      try {
        const targetConfig = mode.value === 'Create' ? configCreateMode : configAnalyzeMode;
        const rawConfig = unwrapConfig(targetConfig);
        const result = await invoke<SimulationResult>('simulate_plan', { config: rawConfig });

        if (mode.value === 'Create') {
          plannedExecutionsCreateMode.value = result.executions;
          plannedMetricsCreateMode.value = result.metrics;
        } else {
          plannedExecutionsAnalyzeMode.value = result.executions;
          plannedMetricsAnalyzeMode.value = result.metrics;
        }

        configErrors.value = result.config_errors;
        simulationError.value = null;
      } catch (e) {
        console.error('Simulation failed:', e);
        simulationError.value = String(e);

        if (mode.value === 'Create') {
          plannedExecutionsCreateMode.value = [];
          plannedMetricsCreateMode.value = [];
        } else {
          plannedExecutionsAnalyzeMode.value = [];
          plannedMetricsAnalyzeMode.value = [];
        }
        configErrors.value = {};
      }
    }, SIMULATION_DEBOUNCE_MS);
  };

  /**
   * Clear all simulation output data and errors.
   */
  const clearSimulationResults = () => {
    if (simulateTimeout) {
      clearTimeout(simulateTimeout);
      simulateTimeout = null;
    }
    if (mode.value === 'Create') {
      plannedExecutionsCreateMode.value = [];
      plannedMetricsCreateMode.value = [];
    } else {
      plannedExecutionsAnalyzeMode.value = [];
      plannedMetricsAnalyzeMode.value = [];
    }
    configErrors.value = {};
    simulationError.value = null;
  };

  // ---------------------------------------------------------------------------
  // Watchers & Reactive Triggers

  // Automatically trigger simulation whenever Create mode configuration changes
  watch(configCreateMode, () => {
    if (mode.value === 'Create') {
      simulatePlan();
    }
  }, { deep: true, immediate: true });

  // Automatically trigger simulation whenever Analyze mode configuration changes
  watch(configAnalyzeMode, () => {
    if (mode.value === 'Analyze') {
      simulatePlan();
    }
  }, { deep: true, immediate: true });

  // Trigger simulation when switching modes
  watch(mode, () => {
    simulatePlan();
  });

  // ---------------------------------------------------------------------------
  // Lifecycle Hooks & Observers

  // (none)

  // ---------------------------------------------------------------------------
  // Public API (Return)

  return {
    plannedExecutionsCreateMode,
    plannedMetricsCreateMode,
    plannedExecutionsAnalyzeMode,
    plannedMetricsAnalyzeMode,
    configErrors,
    simulationError,
    simulatePlan,
    clearSimulationResults
  };
}
