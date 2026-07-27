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
import { ref, reactive, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { AppMode } from "../types/app";
import type { SchedulerConfig, ClientConfig, SchedulerConfigUI, ClientConfigUI } from "../types/config";
import type { PlannedExecution, PlannedMetricPoint, SimulationResult } from "../types/simulation";

// --- App Metadata ---
const appVersion = ref<string>("");

const loadAppVersion = async () => {
  try {
    appVersion.value = await invoke<string>("get_app_version");
  } catch (e) {
    console.error("Failed to load app version:", e);
  }
};

// Fetch version on module initialization
loadAppVersion();

// --- Global ID Counters ---
let nextSessionId = 1;
let nextConfigId = 1;

// --- ID Management Helpers ---

/**
 * Wrap raw SchedulerConfig into UI-friendly structure with unique IDs.
 */
const wrapConfig = (raw: SchedulerConfig): SchedulerConfigUI => {
  const sessionId = nextSessionId++;
  nextConfigId = 1; // Reset config ID for new session

  return {
    sessionId,
    server_config: { ...raw.server_config },
    client_configs: raw.client_configs.map(c => ({
      configId: nextConfigId++,
      data: { ...c }
    }))
  };
};

/**
 * Unwrap UI-friendly structure back to raw SchedulerConfig for Rust backend.
 */
const unwrapConfig = (ui: SchedulerConfigUI): SchedulerConfig => {
  return {
    server_config: { ...ui.server_config },
    client_configs: ui.client_configs.map(c => ({ ...c.data }))
  };
};

// --- Sample Configuration Data ---
const sampleConfigCreateMode: SchedulerConfig = {
  server_config: {
    port: 7878,
    cycle_time_ms: 50,
    stats_interval_cycle: 200
  },
  client_configs: [
    { client_id: 1, display_name: "Camera", cycle: 2, cycle_offset: 0, depends: [], expected_duration_ms: 10 },
    { client_id: 101, display_name: "ObjectDetect", cycle: 2, cycle_offset: 0, depends: [1], expected_duration_ms: 60 },
    { client_id: 102, display_name: "LaneDetect", cycle: 2, cycle_offset: 0, depends: [1], expected_duration_ms: 20 },
    { client_id: 501, display_name: "Control", cycle: 2, cycle_offset: 0, depends: [101, 102], expected_duration_ms: 20 },
    { client_id: 901, display_name: "Telemetry", cycle: 2, cycle_offset: 1, depends: [], expected_duration_ms: 25 },
  ]
};

// --- Singleton App State & Mode ---
const mode = ref<AppMode>('Create');
const selectedClientWrap = ref<ClientConfigUI | null>(null);

// --- Create Mode State ---
const currentConfigPathCreateMode = ref<string>("");
const configCreateMode = reactive<SchedulerConfigUI>(wrapConfig(sampleConfigCreateMode));
const plannedExecutionsCreateMode = ref<PlannedExecution[]>([]);
const plannedMetricsCreateMode = ref<PlannedMetricPoint[]>([]);

// --- Analyze Mode State ---
const currentConfigPathAnalyzeMode = ref<string>("");
const configAnalyzeMode = reactive<SchedulerConfigUI>(wrapConfig(sampleConfigCreateMode));
const plannedExecutionsAnalyzeMode = ref<PlannedExecution[]>([]);
const plannedMetricsAnalyzeMode = ref<PlannedMetricPoint[]>([]);

// --- Shared Simulation Validation & Error States ---
const configErrors = ref<Record<number, string[]>>({});
const simulationError = ref<string | null>(null);

// --- Active Mode Accessors ---
const activeConfigPath = computed<string>(() => {
  return mode.value === 'Create' ? currentConfigPathCreateMode.value : currentConfigPathAnalyzeMode.value;
});

const activeConfig = computed<SchedulerConfigUI>(() => {
  return mode.value === 'Create' ? configCreateMode : configAnalyzeMode;
});

const activePlannedExecutions = computed<PlannedExecution[]>(() => {
  return mode.value === 'Create' ? plannedExecutionsCreateMode.value : plannedExecutionsAnalyzeMode.value;
});

const activePlannedMetrics = computed<PlannedMetricPoint[]>(() => {
  return mode.value === 'Create' ? plannedMetricsCreateMode.value : plannedMetricsAnalyzeMode.value;
});

// --- Simulation Logic & Watchers ---
let simulateTimeout: ReturnType<typeof setTimeout> | null = null;

/**
 * Execute simulation on the Rust backend via IPC for the current active mode (Create or Analyze).
 * Results are used to render the timeline and metrics chart.
 */
const simulatePlan = () => {
  // Use debouncing to prevent excessive IPC calls during rapid configuration changes.
  if (simulateTimeout) clearTimeout(simulateTimeout);
  simulateTimeout = setTimeout(async () => {
    try {
      const targetConfig = mode.value === 'Create' ? configCreateMode : configAnalyzeMode;
      const rawConfig = unwrapConfig(targetConfig);
      const result = await invoke<SimulationResult>("simulate_plan", { config: rawConfig });

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
      console.error("Simulation failed:", e);
      simulationError.value = String(e);

      // Clear simulation results on failure to maintain UI consistency.
      if (mode.value === 'Create') {
        plannedExecutionsCreateMode.value = [];
        plannedMetricsCreateMode.value = [];
      } else {
        plannedExecutionsAnalyzeMode.value = [];
        plannedMetricsAnalyzeMode.value = [];
      }
      configErrors.value = {};
    }
  }, 100); // 100ms debounce
};

// Automatically trigger simulation whenever Create mode configuration changes.
watch(configCreateMode, () => {
  if (mode.value === 'Create') {
    simulatePlan();
  }
}, { deep: true, immediate: true });

// Automatically trigger simulation whenever Analyze mode configuration changes.
watch(configAnalyzeMode, () => {
  if (mode.value === 'Analyze') {
    simulatePlan();
  }
}, { deep: true, immediate: true });

// Trigger simulation and reset dialog when switching modes.
watch(mode, () => {
  closeEdit();
  simulatePlan();
});

// --- Configuration Management Actions ---

const newConfig = () => {
  // Clear simulation data to ensure a clean state for the new configuration.
  if (mode.value === 'Create') {
    plannedExecutionsCreateMode.value = [];
    plannedMetricsCreateMode.value = [];
    currentConfigPathCreateMode.value = "";
  } else {
    plannedExecutionsAnalyzeMode.value = [];
    plannedMetricsAnalyzeMode.value = [];
    currentConfigPathAnalyzeMode.value = "";
  }
  configErrors.value = {};
  simulationError.value = null;

  // Reset configuration to a clean, default state.
  const blankConfig: SchedulerConfig = {
    server_config: {
      port: 7878,
      cycle_time_ms: 50,
      stats_interval_cycle: 0 // Disabled by default
    },
    client_configs: [] // Start with an empty list
  };

  const targetConfig = mode.value === 'Create' ? configCreateMode : configAnalyzeMode;
  const wrapped = wrapConfig(blankConfig);
  targetConfig.sessionId = wrapped.sessionId;
  targetConfig.server_config = wrapped.server_config;
  targetConfig.client_configs = wrapped.client_configs;

  // Ensure any open edit dialogs are closed to prevent inconsistent UI state.
  closeEdit();

  console.log("New config initialized.");
};

const loadConfig = async () => {
  try {
    const selectedPath = await open({
      title: 'Select Config File',
      filters: [{ name: 'TOML Configuration', extensions: ['toml'] }],
      defaultPath: activeConfigPath.value
    });
    if (selectedPath === null) {
      return; // User cancelled
    }

    // Clear the current simulated data to avoid mixing it with the newly loaded data.
    if (mode.value === 'Create') {
      plannedExecutionsCreateMode.value = [];
      plannedMetricsCreateMode.value = [];
      currentConfigPathCreateMode.value = selectedPath as string;
    } else {
      plannedExecutionsAnalyzeMode.value = [];
      plannedMetricsAnalyzeMode.value = [];
      currentConfigPathAnalyzeMode.value = selectedPath as string;
    }
    configErrors.value = {};
    simulationError.value = null;

    // Ensure any open edit dialogs are closed to prevent inconsistent UI state.
    closeEdit();

    // Load configuration.
    const loaded = await invoke<SchedulerConfig>("load_config", { path: selectedPath as string });

    // Sync reactive config with wrapped data
    const targetConfig = mode.value === 'Create' ? configCreateMode : configAnalyzeMode;
    const wrapped = wrapConfig(loaded);
    targetConfig.sessionId = wrapped.sessionId;
    targetConfig.server_config = wrapped.server_config;
    targetConfig.client_configs = wrapped.client_configs;

    console.log("Config loaded successfully.");
    alert(`Config loaded successfully!\nPath: ${selectedPath}`);
  } catch (e) {
    console.error("Failed to load config:", e);
    alert(`Failed to load config:\n${e}`);
  }
};

const saveConfig = async () => {
  try {
    const selectedPath = await save({
      title: 'Save Config File',
      filters: [{ name: 'TOML Configuration', extensions: ['toml'] }],
      defaultPath: activeConfigPath.value
    });

    if (selectedPath === null) {
      return; // User cancelled
    }

    if (mode.value === 'Create') {
      currentConfigPathCreateMode.value = selectedPath as string;
    } else {
      currentConfigPathAnalyzeMode.value = selectedPath as string;
    }

    const targetConfig = mode.value === 'Create' ? configCreateMode : configAnalyzeMode;
    const rawConfig = unwrapConfig(targetConfig);
    console.log("Saving config...", JSON.stringify(rawConfig, null, 2));
    await invoke("save_config", { path: selectedPath as string, config: rawConfig });
    console.log("Config saved successfully.");
    alert(`Config saved successfully!\nPath: ${selectedPath}`);
  } catch (e) {
    console.error("Failed to save config:", e);
    alert(`Failed to save config:\n${e}`);
  }
};

// --- Process Management Actions ---

const openEdit = (clientWrap: ClientConfigUI) => {
  selectedClientWrap.value = clientWrap;
};

const closeEdit = () => {
  selectedClientWrap.value = null;
};

const addClient = () => {
  const targetConfig = mode.value === 'Create' ? configCreateMode : configAnalyzeMode;
  const newId = Math.max(0, ...targetConfig.client_configs.map(c => c.data.client_id)) + 1;
  targetConfig.client_configs.push({
    configId: nextConfigId++,
    data: {
      client_id: newId,
      display_name: "",
      cycle: 1,
      cycle_offset: 0,
      depends: [],
      expected_duration_ms: 10
    }
  });
};

const updateClient = (configId: number, newData: ClientConfig): boolean => {
  const targetConfig = mode.value === 'Create' ? configCreateMode : configAnalyzeMode;

  // Validate CID uniqueness if it has been changed.
  const exists = targetConfig.client_configs.some(c =>
    c.configId !== configId && c.data.client_id === newData.client_id
  );
  if (exists) {
    alert(`CID ${newData.client_id} already exists! Please choose a unique ID.`);
    return false;
  }

  // Update the main config array.
  const idx = targetConfig.client_configs.findIndex(c => c.configId === configId);
  if (idx !== -1) {
    targetConfig.client_configs[idx].data = JSON.parse(JSON.stringify(newData)); // Deep copy to SSOT
    return true;
  }
  return false;
};

const deleteClient = (configId: number) => {
  const targetConfig = mode.value === 'Create' ? configCreateMode : configAnalyzeMode;
  const idx = targetConfig.client_configs.findIndex(c => c.configId === configId);
  if (idx === -1) return;

  const targetCid = targetConfig.client_configs[idx].data.client_id;

  // 1. Remove the target process itself
  targetConfig.client_configs.splice(idx, 1);

  // 2. Cleanup stale dependencies: Remove the deleted CID from all other processes' depends lists
  targetConfig.client_configs.forEach(c => {
    c.data.depends = c.data.depends.filter(depId => depId !== targetCid);
  });
};

/**
 * Move a client config element from one index to another to reorder.
 */
const moveClientConfig = (fromIndex: number, toIndex: number) => {
  const targetConfig = mode.value === 'Create' ? configCreateMode : configAnalyzeMode;
  if (fromIndex === toIndex || toIndex === fromIndex + 1) {
    return;
  }

  const target = targetConfig.client_configs[fromIndex];
  if (!target) return;

  targetConfig.client_configs.splice(fromIndex, 1);
  const insertIndex = fromIndex < toIndex ? toIndex - 1 : toIndex;
  targetConfig.client_configs.splice(insertIndex, 0, target);
};

/**
 * Hook to access global application state.
 */
export function useAppState() {
  return {
    // App Metadata
    appVersion,

    // Singleton App State & Mode
    mode,
    selectedClientWrap,

    // Create Mode State
    currentConfigPathCreateMode,
    configCreateMode,
    plannedExecutionsCreateMode,
    plannedMetricsCreateMode,

    // Analyze Mode State
    currentConfigPathAnalyzeMode,
    configAnalyzeMode,
    plannedExecutionsAnalyzeMode,
    plannedMetricsAnalyzeMode,

    // Shared Simulation Validation & Error States
    configErrors,
    simulationError,

    // Active Mode Accessors
    activeConfigPath,
    activeConfig,
    activePlannedExecutions,
    activePlannedMetrics,

    // Actions
    newConfig,
    loadConfig,
    saveConfig,
    openEdit,
    closeEdit,
    addClient,
    updateClient,
    deleteClient,
    moveClientConfig
  };
}
