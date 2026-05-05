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
const editingClient = ref<ClientConfig | null>(null);
const editingConfigId = ref<number | null>(null);
const editingDependsStr = ref<string>("");
const isConfirmingDelete = ref<boolean>(false);
const currentConfigPath = ref<string>("../../rt-server-rs/config.toml");

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
    stats_interval_cycle: 40
  },
  client_configs: [
    { client_id: 10, display_name: "Camera", cycle: 2, cycle_offset: 0, depends: [], expected_duration_ms: 15 },
    { client_id: 11, display_name: "Preprocess", cycle: 2, cycle_offset: 0, depends: [10], expected_duration_ms: 20 },
    { client_id: 20, display_name: "MainProcess", cycle: 2, cycle_offset: 1, depends: [], expected_duration_ms: 40 },
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

// --- Process Editing Actions ---

const openEdit = (clientWrap: ClientConfigUI) => {
  // Create a deep copy for editing to allow cancellation.
  editingClient.value = JSON.parse(JSON.stringify(clientWrap.data));
  editingConfigId.value = clientWrap.configId;
  editingDependsStr.value = clientWrap.data.depends.join(', ');
  isConfirmingDelete.value = false;
};

const closeEdit = (save: boolean) => {
  if (save && editingClient.value && editingConfigId.value !== null) {
    const newId = editingClient.value.client_id;
    const targetConfigId = editingConfigId.value;

    // Validate CID uniqueness if it has been changed.
    const exists = config.client_configs.some(c =>
      c.configId !== targetConfigId && c.data.client_id === newId
    );
    if (exists) {
      alert(`CID ${newId} already exists! Please choose a unique ID.`);
      return;
    }

    // Parse comma-separated string back to array of numbers.
    editingClient.value.depends = editingDependsStr.value
      .split(',')
      .map(s => parseInt(s.trim()))
      .filter(n => !isNaN(n));

    // Update the main config array.
    const idx = config.client_configs.findIndex(c => c.configId === targetConfigId);
    if (idx !== -1) {
      config.client_configs[idx].data = editingClient.value;
    }
  }

  // Reset editing state.
  editingClient.value = null;
  editingConfigId.value = null;
  editingDependsStr.value = "";
  isConfirmingDelete.value = false;
};

const addProcess = () => {
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

const resetDeleteConfirm = () => {
  isConfirmingDelete.value = false;
};

const deleteProcess = async () => {
  if (editingConfigId.value !== null) {
    if (!isConfirmingDelete.value) {
      isConfirmingDelete.value = true;
      return;
    }

    const targetConfigId = editingConfigId.value;
    const targetCid = editingClient.value?.client_id;

    // 1. Remove the target process itself
    config.client_configs = config.client_configs.filter(c => c.configId !== targetConfigId);

    // 2. Cleanup stale dependencies: Remove the deleted CID from all other processes' depends lists
    if (targetCid !== undefined) {
      config.client_configs.forEach(c => {
        c.data.depends = c.data.depends.filter(depId => depId !== targetCid);
      });
    }

    // Close the popup after deletion without saving
    editingClient.value = null;
    editingConfigId.value = null;
    editingDependsStr.value = "";
    isConfirmingDelete.value = false;
  }
};

/**
 * Hook to access global application state.
 */
export function useAppState() {
  return {
    mode,
    editingClient,
    editingConfigId,
    editingDependsStr,
    isConfirmingDelete,
    config,
    planned_executions,
    planned_metrics,
    config_errors,
    simulation_error,
    loadConfig,
    saveConfig,
    openEdit,
    closeEdit,
    addProcess,
    deleteProcess,
    resetDeleteConfirm
  };
}
