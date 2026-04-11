import { ref, reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { SchedulerConfig, AppMode, ClientConfig } from "../types/config";

// --- Singleton App State ---
const mode = ref<AppMode>('Create');
const editingClient = ref<ClientConfig | null>(null);
const originalClientId = ref<number | null>(null);
const editingDependsStr = ref<string>("");
const isConfirmingDelete = ref<boolean>(false);
const currentConfigPath = ref<string>("../../rt-server-rs/config.toml");

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

// --- Actions ---
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
    // Apply loaded config to the reactive object
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

const openEdit = (client: ClientConfig) => {
  editingClient.value = JSON.parse(JSON.stringify(client));
  originalClientId.value = client.client_id;
  editingDependsStr.value = client.depends.join(', ');
  isConfirmingDelete.value = false;
};

const closeEdit = (save: boolean) => {
  if (save && editingClient.value && originalClientId.value !== null) {
    // CID duplication check
    const newId = editingClient.value.client_id;
    if (newId !== originalClientId.value) {
      const exists = config.client_configs.some(c => c.client_id === newId);
      if (exists) {
        alert(`CID ${newId} already exists! Please choose a unique ID.`);
        return; // Abort save, keep popup open
      }
    }
    // Parse depends CSV
    editingClient.value.depends = editingDependsStr.value
      .split(',')
      .map(s => parseInt(s.trim()))
      .filter(n => !isNaN(n));
    // Save
    const idx = config.client_configs.findIndex(c => c.client_id === originalClientId.value);
    if (idx !== -1) {
      config.client_configs[idx] = editingClient.value;
    }
  }
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
    config.client_configs = config.client_configs.filter(c => c.client_id !== cid);

    // Close the popup after deletion without saving
    editingClient.value = null;
    originalClientId.value = null;
    editingDependsStr.value = "";
    isConfirmingDelete.value = false;
  }
};

export function useAppState() {
  return {
    mode,
    editingClient,
    originalClientId,
    editingDependsStr,
    isConfirmingDelete,
    config,
    loadConfig,
    saveConfig,
    openEdit,
    closeEdit,
    addProcess,
    deleteProcess,
    resetDeleteConfirm
  };
}
