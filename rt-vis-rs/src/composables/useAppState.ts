import { ref, reactive, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { SchedulerConfig, AppMode, ClientConfig } from "../types/config";
import type { PlannedExecution, PlannedMetricPoint, SimulationResult } from "../types/simulation";

// --- Singleton App State ---
const mode = ref<AppMode>('Create');
const editingClient = ref<ClientConfig | null>(null);
const originalClientId = ref<number | null>(null);
const editingDependsStr = ref<string>("");
const isConfirmingDelete = ref<boolean>(false);
const currentConfigPath = ref<string>("../../rt-server-rs/config.toml");

// --- Simulation State ---
const plannedExecutions = ref<PlannedExecution[]>([]);
const plannedMetrics = ref<PlannedMetricPoint[]>([]);
const configErrors = ref<Record<number, string[]>>({});
const simulationError = ref<string | null>(null);

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
      const result = await invoke<SimulationResult>("simulate_plan", { config: config });
      plannedExecutions.value = result.executions;
      plannedMetrics.value = result.metrics;
      configErrors.value = result.configErrors;
      simulationError.value = null;
    } catch (e) {
      console.error("Simulation failed:", e);
      simulationError.value = String(e);
      // Clear simulation results on failure to maintain UI consistency.
      plannedExecutions.value = [];
      plannedMetrics.value = [];
      configErrors.value = {};
    }
  }, 100); // 100ms debounce
};

// --- Sample Data ---
const config = reactive<SchedulerConfig>({
  server_config: {
    port: 7878,
    cycle_time_ms: 50,
    stats_interval_cycle: 40
  },
  client_configs: [
    { client_id: 10, cycle: 2, cycle_offset: 0, depends: [], expected_duration_ms: 15 },
    { client_id: 11, cycle: 2, cycle_offset: 0, depends: [10], expected_duration_ms: 20 },
    { client_id: 20, cycle: 2, cycle_offset: 1, depends: [], expected_duration_ms: 40 },
  ]
});

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

    currentConfigPath.value = selectedPath as string;
    const loaded = await invoke<SchedulerConfig>("load_config", { path: currentConfigPath.value });

    // Sync reactive config with loaded data
    config.server_config = loaded.server_config;
    config.client_configs = loaded.client_configs;
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

    console.log("Saving config...", JSON.stringify(config, null, 2));
    await invoke("save_config", { path: currentConfigPath.value, config: config });
    console.log("Config saved successfully.");
    alert(`Config saved successfully!\nPath: ${currentConfigPath.value}`);
  } catch (e) {
    console.error("Failed to save config:", e);
    alert(`Failed to save config:\n${e}`);
  }
};

// --- Process Editing Actions ---

const openEdit = (client: ClientConfig) => {
  // Create a deep copy for editing to allow cancellation.
  editingClient.value = JSON.parse(JSON.stringify(client));
  originalClientId.value = client.client_id;
  editingDependsStr.value = client.depends.join(', ');
  isConfirmingDelete.value = false;
};

const closeEdit = (save: boolean) => {
  if (save && editingClient.value && originalClientId.value !== null) {
    const newId = editingClient.value.client_id;

    // Validate CID uniqueness if it has been changed.
    if (newId !== originalClientId.value) {
      const exists = config.client_configs.some(c => c.client_id === newId);
      if (exists) {
        alert(`CID ${newId} already exists! Please choose a unique ID.`);
        return;
      }
    }

    // Parse comma-separated string back to array of numbers.
    editingClient.value.depends = editingDependsStr.value
      .split(',')
      .map(s => parseInt(s.trim()))
      .filter(n => !isNaN(n));

    // Update the main config array.
    const idx = config.client_configs.findIndex(c => c.client_id === originalClientId.value);
    if (idx !== -1) {
      config.client_configs[idx] = editingClient.value;
    }
  }

  // Reset editing state.
  editingClient.value = null;
  originalClientId.value = null;
  editingDependsStr.value = "";
  isConfirmingDelete.value = false;
};

const addProcess = () => {
  const newId = Math.max(0, ...config.client_configs.map(c => c.client_id)) + 1;
  config.client_configs.push({
    client_id: newId,
    cycle: 1,
    cycle_offset: 0,
    depends: [],
    expected_duration_ms: 10
  });
};

const resetDeleteConfirm = () => {
  isConfirmingDelete.value = false;
};

const deleteProcess = async () => {
  if (originalClientId.value !== null) {
    if (!isConfirmingDelete.value) {
      isConfirmingDelete.value = true;
      return;
    }

    const cid = originalClientId.value;

    // 1. Remove the target process itself
    config.client_configs = config.client_configs.filter(c => c.client_id !== cid);

    // 2. Cleanup stale dependencies: Remove the deleted CID from all other processes' depends lists
    config.client_configs.forEach(c => {
      c.depends = c.depends.filter(depId => depId !== cid);
    });

    // Close the popup after deletion without saving
    editingClient.value = null;
    originalClientId.value = null;
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
    originalClientId,
    editingDependsStr,
    isConfirmingDelete,
    config,
    plannedExecutions,
    plannedMetrics,
    configErrors,
    simulationError,
    loadConfig,
    saveConfig,
    openEdit,
    closeEdit,
    addProcess,
    deleteProcess,
    resetDeleteConfirm
  };
}
