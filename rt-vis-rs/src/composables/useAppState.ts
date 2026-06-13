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
import { ref, reactive, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { AppMode } from "../types/app"
import type { SchedulerConfig, ClientConfig, SchedulerConfigUI, ClientConfigUI } from "../types/config";
import type { PlannedExecution, PlannedMetricPoint, SimulationResult } from "../types/simulation";

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

// --- Singleton App State ---
const mode = ref<AppMode>('Create');
const selectedClientWrap = ref<ClientConfigUI | null>(null);
const currentConfigPath = ref<string>("");

// --- Simulation State ---
const planned_executions = ref<PlannedExecution[]>([]);
const planned_metrics = ref<PlannedMetricPoint[]>([]);
const config_errors = ref<Record<number, string[]>>({});
const simulation_error = ref<string | null>(null);

let simulateTimeout: ReturnType<typeof setTimeout> | null = null;

/**
 * Execute simulation on the Rust backend via IPC.
 * Results are used to render the timeline and metrics chart.
 */
const simulatePlan = () => {
  // Use debouncing to prevent excessive IPC calls during rapid configuration changes.
  if (simulateTimeout) clearTimeout(simulateTimeout);
  simulateTimeout = setTimeout(async () => {
    try {
      const rawConfig = unwrapConfig(config);
      const result = await invoke<SimulationResult>("simulate_plan", { config: rawConfig });
      planned_executions.value = result.executions;
      planned_metrics.value = result.metrics;
      config_errors.value = result.config_errors;
      simulation_error.value = null;
    } catch (e) {
      console.error("Simulation failed:", e);
      simulation_error.value = String(e);
      // Clear simulation results on failure to maintain UI consistency.
      planned_executions.value = [];
      planned_metrics.value = [];
      config_errors.value = {};
    }
  }, 100); // 100ms debounce
};

// --- Initial Sample Data ---
const initialRaw: SchedulerConfig = {
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

const config = reactive<SchedulerConfigUI>(wrapConfig(initialRaw));

// Automatically trigger simulation whenever the reactive config object changes.
watch(config, () => {
  if (mode.value === 'Create') {
    simulatePlan();
  }
}, { deep: true, immediate: true });

// --- Configuration Management Actions ---

const newConfig = () => {
  // Clear simulation data to ensure a clean state for the new configuration.
  planned_executions.value = [];
  planned_metrics.value = [];
  config_errors.value = {};
  simulation_error.value = null;

  // Reset file path to indicate this is a new, unsaved configuration.
  currentConfigPath.value = "";

  // Reset configuration to a clean, default state.
  const blankConfig: SchedulerConfig = {
    server_config: {
      port: 7878,
      cycle_time_ms: 50,
      stats_interval_cycle: 0 // Disabled by default
    },
    client_configs: [] // Start with an empty list
  };

  const wrapped = wrapConfig(blankConfig);
  config.sessionId = wrapped.sessionId;
  config.server_config = wrapped.server_config;
  config.client_configs = wrapped.client_configs;

  // Ensure any open edit dialogs are closed to prevent inconsistent UI state.
  closeEdit();

  console.log("New config initialized.");
};

const loadConfig = async () => {
  try {
    const selectedPath = await open({
      title: 'Select Config File',
      filters: [{ name: 'TOML Configuration', extensions: ['toml'] }],
      defaultPath: currentConfigPath.value
    });
    if (selectedPath === null) {
      return; // User cancelled
    }

    // Clear the current simulated data to avoid mixing it with the newly loaded data.
    planned_executions.value = [];
    planned_metrics.value = [];
    config_errors.value = {};
    simulation_error.value = null;

    // Load configuration.
    currentConfigPath.value = selectedPath as string;
    const loaded = await invoke<SchedulerConfig>("load_config", { path: currentConfigPath.value });

    // Sync reactive config with wrapped data
    const wrapped = wrapConfig(loaded);
    config.sessionId = wrapped.sessionId;
    config.server_config = wrapped.server_config;
    config.client_configs = wrapped.client_configs;

    console.log("Config loaded successfully.");
    alert(`Config loaded successfully!\nPath: ${currentConfigPath.value}`);
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
      defaultPath: currentConfigPath.value
    });

    if (selectedPath === null) {
      return; // User cancelled
    }

    currentConfigPath.value = selectedPath as string;

    const rawConfig = unwrapConfig(config);
    console.log("Saving config...", JSON.stringify(rawConfig, null, 2));
    await invoke("save_config", { path: currentConfigPath.value, config: rawConfig });
    console.log("Config saved successfully.");
    alert(`Config saved successfully!\nPath: ${currentConfigPath.value}`);
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
  const newId = Math.max(0, ...config.client_configs.map(c => c.data.client_id)) + 1;
  config.client_configs.push({
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
  // Validate CID uniqueness if it has been changed.
  const exists = config.client_configs.some(c =>
    c.configId !== configId && c.data.client_id === newData.client_id
  );
  if (exists) {
    alert(`CID ${newData.client_id} already exists! Please choose a unique ID.`);
    return false;
  }

  // Update the main config array.
  const idx = config.client_configs.findIndex(c => c.configId === configId);
  if (idx !== -1) {
    config.client_configs[idx].data = JSON.parse(JSON.stringify(newData)); // Deep copy to SSOT
    return true;
  }
  return false;
};

const deleteClient = (configId: number) => {
  const idx = config.client_configs.findIndex(c => c.configId === configId);
  if (idx === -1) return;

  const targetCid = config.client_configs[idx].data.client_id;

  // 1. Remove the target process itself
  config.client_configs.splice(idx, 1);

  // 2. Cleanup stale dependencies: Remove the deleted CID from all other processes' depends lists
  config.client_configs.forEach(c => {
    c.data.depends = c.data.depends.filter(depId => depId !== targetCid);
  });
};

/**
 * Move a client config element from one index to another to reorder.
 */
const moveClientConfig = (fromIndex: number, toIndex: number) => {
  if (fromIndex === toIndex || toIndex === fromIndex + 1) {
    return;
  }

  const target = config.client_configs[fromIndex];
  if (!target) return;

  config.client_configs.splice(fromIndex, 1);
  const insertIndex = fromIndex < toIndex ? toIndex - 1 : toIndex;
  config.client_configs.splice(insertIndex, 0, target);
};

/**
 * Hook to access global application state.
 */
export function useAppState() {
  return {
    mode,
    selectedClientWrap,
    config,
    planned_executions,
    planned_metrics,
    config_errors,
    simulation_error,
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
