import { ref, reactive } from "vue";
import type { SchedulerConfig, AppMode, ClientConfig } from "../types/config";

// --- Singleton App State ---
const mode = ref<AppMode>('Create');
const editingClient = ref<ClientConfig | null>(null);
const originalClientId = ref<number | null>(null);
const editingDependsStr = ref<string>("");

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
const saveConfig = () => {
  console.log("Saving config...", JSON.stringify(config, null, 2));
};

const openEdit = (client: ClientConfig) => {
  editingClient.value = JSON.parse(JSON.stringify(client));
  originalClientId.value = client.client_id;
  editingDependsStr.value = client.depends.join(', ');
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

export function useAppState() {
  return {
    mode,
    editingClient,
    originalClientId,
    editingDependsStr,
    config,
    saveConfig,
    openEdit,
    closeEdit,
    addProcess
  };
}
