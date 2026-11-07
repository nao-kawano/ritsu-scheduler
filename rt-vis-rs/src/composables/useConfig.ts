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

import { ref, reactive, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { SchedulerConfig, ClientConfig, SchedulerConfigUI, ClientConfigUI } from '../types/config';
import { DEFAULT_SERVER_CONFIG, DEFAULT_CLIENT_CONFIG } from '../types/config';
import { useApp } from './useApp';

// =============================================================================
// Types & Constants
// =============================================================================

const sampleConfigCreateMode: SchedulerConfigUI = {
  sessionId: 1,
  server_config: {
    ...DEFAULT_SERVER_CONFIG,
    stats_interval_cycle: 200
  },
  client_configs: [
    { configId: 1, data: { client_id: 1, start_cycle: 0, interval: 2, offset: 0, depends: [], display_name: "Camera", expected_duration_ms: 10 } },
    { configId: 2, data: { client_id: 101, start_cycle: 0, interval: 2, offset: 0, depends: [1], display_name: "ObjectDetect", expected_duration_ms: 60 } },
    { configId: 3, data: { client_id: 102, start_cycle: 0, interval: 2, offset: 0, depends: [1], display_name: "LaneDetect", expected_duration_ms: 20 } },
    { configId: 4, data: { client_id: 501, start_cycle: 0, interval: 2, offset: 0, depends: [101, 102], display_name: "Control", expected_duration_ms: 20 } },
    { configId: 5, data: { client_id: 901, start_cycle: 0, interval: 2, offset: 1, depends: [], display_name: "Telemetry", expected_duration_ms: 25 } },
  ]
};

// =============================================================================
// Module State (Singleton)
// =============================================================================

// Incremental counters derived from initial sample data
let nextSessionId = sampleConfigCreateMode.sessionId + 1;
let nextConfigId = Math.max(0, ...sampleConfigCreateMode.client_configs.map(c => c.configId)) + 1;

// Global & Session State
const configCreateMode = reactive<SchedulerConfigUI>(JSON.parse(JSON.stringify(sampleConfigCreateMode)));
const configAnalyzeMode = reactive<SchedulerConfigUI>(JSON.parse(JSON.stringify(sampleConfigCreateMode)));
const currentConfigPathCreateMode = ref<string>('');
const currentConfigPathAnalyzeMode = ref<string>('');

// Client State
const selectedClientWrap = ref<ClientConfigUI | null>(null);

// =============================================================================
// Internal Helpers
// =============================================================================

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

// =============================================================================
// Composable Implementation
// =============================================================================

export function useConfig() {
  // ---------------------------------------------------------------------------
  // Dependencies & Inject

  const { mode } = useApp();

  // ---------------------------------------------------------------------------
  // Local State & Computed

  const activeConfig = computed<SchedulerConfigUI>(() => {
    return mode.value === 'Create' ? configCreateMode : configAnalyzeMode;
  });

  const activeConfigPath = computed<string>(() => {
    return mode.value === 'Create' ? currentConfigPathCreateMode.value : currentConfigPathAnalyzeMode.value;
  });

  // ---------------------------------------------------------------------------
  // Methods & Actions

  const newConfig = () => {
    const blankConfig: SchedulerConfig = {
      server_config: { ...DEFAULT_SERVER_CONFIG },
      client_configs: [] // Start with an empty list
    };

    if (mode.value === 'Create') {
      currentConfigPathCreateMode.value = "";
    } else {
      currentConfigPathAnalyzeMode.value = "";
    }

    const targetConfig = mode.value === 'Create' ? configCreateMode : configAnalyzeMode;
    const wrapped = wrapConfig(blankConfig);
    targetConfig.sessionId = wrapped.sessionId;
    targetConfig.server_config = wrapped.server_config;
    targetConfig.client_configs = wrapped.client_configs;

    closeClientEdit();
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

      if (mode.value === 'Create') {
        currentConfigPathCreateMode.value = selectedPath as string;
      } else {
        currentConfigPathAnalyzeMode.value = selectedPath as string;
      }

      closeClientEdit();

      const loaded = await invoke<SchedulerConfig>("load_config", { path: selectedPath as string });

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

  /**
   * Restore configuration from loaded log summary (for Analyze Mode).
   */
  const restoreConfigFromLog = (rawConfig: SchedulerConfig) => {
    const wrapped = wrapConfig(rawConfig);
    configAnalyzeMode.sessionId = wrapped.sessionId;
    configAnalyzeMode.server_config = wrapped.server_config;
    configAnalyzeMode.client_configs = wrapped.client_configs;
  };

  const openClientEdit = (clientWrap: ClientConfigUI) => {
    selectedClientWrap.value = clientWrap;
  };

  const closeClientEdit = () => {
    selectedClientWrap.value = null;
  };

  const addClient = () => {
    const targetConfig = mode.value === 'Create' ? configCreateMode : configAnalyzeMode;
    const newId = Math.max(0, ...targetConfig.client_configs.map(c => c.data.client_id)) + 1;
    targetConfig.client_configs.push({
      configId: nextConfigId++,
      data: {
        ...DEFAULT_CLIENT_CONFIG,
        client_id: newId,
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

    // 1. Remove the target client itself
    targetConfig.client_configs.splice(idx, 1);

    // 2. Cleanup stale dependencies: Remove the deleted CID from all other clients' depends lists
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

  // ---------------------------------------------------------------------------
  // Watchers & Reactive Triggers

  // Reset edit dialog when switching modes
  watch(mode, () => {
    closeClientEdit();
  });

  // ---------------------------------------------------------------------------
  // Lifecycle Hooks & Observers

  // (none)

  // ---------------------------------------------------------------------------
  // Public API (Return)

  return {
    activeConfig,
    configCreateMode,
    configAnalyzeMode,
    currentConfigPathCreateMode,
    selectedClientWrap,
    newConfig,
    loadConfig,
    saveConfig,
    restoreConfigFromLog,
    unwrapConfig,
    openClientEdit,
    closeClientEdit,
    addClient,
    updateClient,
    deleteClient,
    moveClientConfig
  };
}
