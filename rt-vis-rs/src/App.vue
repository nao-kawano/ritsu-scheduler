<script setup lang="ts">
import { ref, reactive } from "vue";
import type { SchedulerConfig, AppMode, ClientConfig } from "./types/config";
import { useScrollSync } from "./composables/useScrollSync";

// --- App State ---
const mode = ref<AppMode>('Create');
const editingClient = ref<ClientConfig | null>(null);
const originalClientId = ref<number | null>(null);
const editingDependsStr = ref<string>("");

// --- Refs for Scroll Sync ---
const processListScroll = ref<HTMLElement | null>(null);
const timelineHeaderScroll = ref<HTMLElement | null>(null);
const timelineScroll = ref<HTMLElement | null>(null);
const metricsHeaderScroll = ref<HTMLElement | null>(null);
const metricsChartScroll = ref<HTMLElement | null>(null);

// Initialize Scroll Sync
const { onTimelineScroll, onMetricsScroll } = useScrollSync(
  processListScroll,
  timelineHeaderScroll,
  timelineScroll,
  metricsHeaderScroll,
  metricsChartScroll
);

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
</script>

<template>
  <div class="app-container" :class="mode.toLowerCase()">
    <!-- Top Pane -->
    <header class="global-control-pane">
      <div class="top-row">
        <div class="brand">Ritsu Vis</div>
        <div class="mode-selector">
          <button :class="{ active: mode === 'Create' }" @click="mode = 'Create'">Create</button>
          <button :class="{ active: mode === 'Analyze' }" @click="mode = 'Analyze'">Analyze</button>
        </div>
      </div>
      <div class="bottom-row">
        <div class="server-info-inputs">
          <div class="input-group">
            <label>Port</label>
            <input type="number" v-model="config.server_config.port" />
          </div>
          <div class="input-group">
            <label>Cycle(ms)</label>
            <input type="number" v-model="config.server_config.cycle_time_ms" />
          </div>
          <div class="input-group">
            <label>Stats Interval(cycles)</label>
            <input type="number" v-model="config.server_config.stats_interval_cycle" />
            <span class="calc-hint"
              v-if="config.server_config.stats_interval_cycle && config.server_config.cycle_time_ms">
              (= {{ config.server_config.stats_interval_cycle * config.server_config.cycle_time_ms }} ms)
            </span>
          </div>
        </div>
        <div class="actions">
          <button class="primary" @click="saveConfig">Save Config</button>
        </div>
      </div>
    </header>

    <div class="process-section">
      <!-- Left Pane -->
      <aside class="process-list-pane">
        <div class="pane-header">Processes</div>
        <div class="scroll-area process-list-scroll" ref="processListScroll">
          <div class="process-list-container">
            <div v-for="client in config.client_configs" :key="client.client_id" class="process-row-wrapper">
              <div class="process-card" @click="openEdit(client)">
                <div class="cid">CID: {{ String(client.client_id).padStart(3, '0') }}</div>
                <div class="meta-block">
                  <div class="details">C: {{ client.cycle }}, O: {{ client.cycle_offset }}, D: {{
                    client.expected_duration_ms }}ms</div>
                  <div class="depends" :class="{ 'no-deps': client.depends.length === 0 }">Deps: {{
                    client.depends.length > 0 ? client.depends.join(', ') : '-' }}</div>
                </div>
              </div>
            </div>
            <div class="process-row-wrapper add-btn-row">
              <button class="add-btn" @click="addProcess">+ Add Process</button>
            </div>
          </div>
        </div>
      </aside>

      <!-- Right Pane -->
      <main class="timeline-pane">
        <div class="timeline-header hide-scrollbar" ref="timelineHeaderScroll">
          <div class="time-axis">
            <div v-for="n in 50" :key="n" class="time-tick">{{ (n - 1) * 5 }}ms</div>
          </div>
        </div>
        <div class="scroll-area timeline-scroll" ref="timelineScroll" @scroll="onTimelineScroll">
          <div class="timeline-container">
            <div v-for="client in config.client_configs" :key="client.client_id" class="timeline-row">
              <div class="plan-preview">Timeline Row for CID {{ client.client_id }}</div>
            </div>
            <div class="timeline-row add-btn-placeholder"></div>
          </div>
        </div>
      </main>
    </div>

    <!-- Bottom Pane -->
    <footer class="metrics-section">
      <div class="metrics-labels-pane">
        <div class="pane-header">Metrics Labels</div>
        <div class="metrics-labels">
          <div class="metric-label">Concurrent Processes</div>
          <div class="metric-label">Cycle Jitter</div>
        </div>
      </div>
      <div class="metrics-content-pane">
        <div class="timeline-header hide-scrollbar" ref="metricsHeaderScroll">
          <div class="time-axis">
            <div v-for="n in 50" :key="n" class="time-tick">{{ (n - 1) * 5 }}ms</div>
          </div>
        </div>
        <div class="metrics-chart-scroll" ref="metricsChartScroll" @scroll="onMetricsScroll">
          <div class="metrics-timeline">
            <div class="placeholder">Metrics Graph Area (Synced with Timeline)</div>
          </div>
        </div>
      </div>
    </footer>

    <!-- Edit Popup -->
    <div v-if="editingClient" class="overlay" @click.self="closeEdit(false)">
      <div class="popup">
        <h3>Edit Process: CID {{ String(editingClient.client_id).padStart(3, '0') }}</h3>
        <div class="form-grid">
          <label>CID</label><input type="number" v-model="editingClient.client_id" min="0" />
          <label>Cycle</label><input type="number" v-model="editingClient.cycle" min="1" />
          <label>Offset</label><input type="number" v-model="editingClient.cycle_offset" min="0" />
          <label>Duration (ms)</label><input type="number" v-model="editingClient.expected_duration_ms" min="0" />
          <label>Depends (CSV)</label>
          <input type="text" v-model="editingDependsStr" />
        </div>
        <div class="popup-actions">
          <button @click="closeEdit(false)">Cancel</button>
          <button class="primary" @click="closeEdit(true)">Apply</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style>
/* =========================================
   Global Styles & CSS Variables
   ========================================= */
:root {
  --top-height: 100px;
  --bottom-height: 160px;
  --left-width: 250px;
  --row-height: 84px;
  --header-row-height: 36px;
  --tick-width: 100px;
  --total-ticks: 50;

  --bg-color: #f0f2f5;
  --pane-bg: #ffffff;
  --border-color: #dcdfe6;
  --primary-color: #396cd8;
  --accent-color: #ff4081;
  --text-main: #2c3e50;
  --text-dim: #909399;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg-color: #121212;
    --pane-bg: #1e1e1e;
    --border-color: #333;
    --text-main: #e0e0e0;
    --text-dim: #888;
  }
}

* {
  box-sizing: border-box;
}

body,
html {
  margin: 0;
  padding: 0;
  height: 100%;
  overflow: hidden;
}
</style>

<style scoped>
/* =========================================
   App Component Scoped Styles
   ========================================= */

.app-container {
  display: grid;
  grid-template-rows: var(--top-height) 1fr var(--bottom-height);
  height: 100vh;
  width: 100vw;
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
  background-color: var(--bg-color);
  overflow: hidden;
}

.global-control-pane {
  background-color: var(--pane-bg);
  border-bottom: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  padding: 0 1.5rem;
  z-index: 100;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
}

.top-row,
.bottom-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  flex: 1;
}

.top-row {
  border-bottom: 1px solid var(--border-color);
}

.brand {
  font-size: 1.4rem;
  font-weight: 800;
  color: var(--primary-color);
}

.server-info-inputs {
  display: flex;
  align-items: center;
  gap: 1.5rem;
}

.input-group {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.input-group label {
  font-size: 0.7rem;
  font-weight: bold;
  color: var(--text-dim);
}

.input-group input {
  width: 70px;
  padding: 4px 8px;
  border-radius: 4px;
  border: 1px solid var(--border-color);
  background: var(--bg-color);
  color: var(--text-main);
  font-weight: bold;
}

.calc-hint {
  font-size: 0.7rem;
  color: var(--text-dim);
  margin-left: 0.25rem;
}

.mode-selector {
  display: flex;
  background: var(--bg-color);
  padding: 3px;
  border-radius: 6px;
}

.mode-selector button {
  border: none;
  background: transparent;
  padding: 0.4rem 1rem;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 600;
  color: var(--text-dim);
}

.mode-selector button.active {
  background: var(--pane-bg);
  color: var(--primary-color);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.process-section {
  display: grid;
  grid-template-columns: var(--left-width) 1fr;
  overflow: hidden;
  min-height: 0;
}

.process-list-pane {
  background-color: var(--pane-bg);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}

.pane-header {
  height: var(--header-row-height);
  padding: 0 1rem;
  display: flex;
  align-items: center;
  font-weight: bold;
  font-size: 0.75rem;
  color: var(--text-dim);
  text-transform: uppercase;
  border-bottom: 1px solid var(--border-color);
  background: rgba(0, 0, 0, 0.02);
  flex-shrink: 0;
}

.scroll-area {
  flex: 1;
  min-height: 0;
  min-width: 0;
}

.scroll-area.timeline-scroll {
  overflow-y: scroll;
  overflow-x: hidden;
  width: 100%;
  height: 100%;
}

/* Layout Alignment & Scrollbar Sync Rules */

/* 1. Process List: no scrollbars */
.process-list-scroll {
  overflow: hidden;
}


.timeline-pane {
  display: flex;
  flex-direction: column;
  background-color: var(--pane-bg);
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.timeline-header {
  height: var(--header-row-height);
  overflow: hidden;
  padding-right: 10px;
  /* match timeline vertical scrollbar width */
  flex-shrink: 0;
  border-bottom: 1px solid var(--border-color);
  background: rgba(0, 0, 0, 0.02);
}


.process-row-wrapper {
  height: var(--row-height);
  padding: 0.4rem 0.75rem;
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
}

.process-card {
  width: 100%;
  height: 100%;
  padding: 0.4rem 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background-color: var(--pane-bg);
  cursor: pointer;
  display: flex;
  flex-direction: column;
  justify-content: center;
  transition: all 0.2s;
}

.process-card:hover {
  border-color: var(--primary-color);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.process-card .cid {
  font-weight: bold;
  font-size: 1.05rem;
}

.meta-block {
  margin-top: 2px;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.process-card .details {
  font-size: 0.75rem;
  color: var(--text-dim);
}

.process-card .depends {
  font-size: 0.7rem;
  color: var(--accent-color);
  font-weight: bold;
}

.process-card .depends.no-deps {
  color: var(--text-dim);
  font-weight: normal;
}

.add-btn {
  width: 100%;
  height: 40px;
  border: 2px dashed var(--border-color);
  border-radius: 8px;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  font-weight: bold;
  font-size: 0.8rem;
}

.time-axis {
  display: flex;
  width: max-content;
  height: 100%;
}

.time-tick {
  width: var(--tick-width);
  height: 100%;
  border-right: 1px solid var(--border-color);
  padding: 0 0.5rem;
  display: flex;
  align-items: center;
  font-size: 0.7rem;
  color: var(--text-dim);
  flex-shrink: 0;
}

.timeline-container {
  width: calc(var(--total-ticks) * var(--tick-width));
  background-image: linear-gradient(90deg, var(--border-color) 1px, transparent 1px);
  background-size: var(--tick-width) 100%;
  background-position: -1px 0;
}

.timeline-row {
  height: var(--row-height);
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  padding: 0 1rem;
}

.plan-preview {
  font-size: 0.75rem;
  color: var(--text-dim);
  opacity: 0.4;
}

.metrics-section {
  display: grid;
  grid-template-columns: var(--left-width) 1fr;
  background-color: var(--pane-bg);
  border-top: 2px solid var(--border-color);
  height: var(--bottom-height);
}

.metrics-labels-pane {
  border-right: 1px solid var(--border-color);
}

.metrics-content-pane {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.metric-label {
  height: calc((var(--bottom-height) - var(--header-row-height) - 10px) / 2);
  display: flex;
  align-items: center;
  padding: 0 1rem;
  font-size: 0.75rem;
  color: var(--text-dim);
}

.metrics-chart-scroll {
  flex: 1;
  overflow-x: scroll;
  overflow-y: hidden;
  padding-right: 10px;
  /* match timeline vertical scrollbar width */
}


.metrics-timeline {
  width: calc(var(--total-ticks) * var(--tick-width));
  height: 100%;
  background-image: linear-gradient(90deg, var(--border-color) 1px, transparent 1px);
  background-size: var(--tick-width) 100%;
  background-position: -1px 0;
  display: flex;
  align-items: center;
  justify-content: flex-start;
  padding: 0 1rem;
}

.placeholder {
  font-size: 0.75rem;
  color: var(--text-dim);
  opacity: 0.4;
}



.overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.popup {
  background: var(--pane-bg);
  padding: 2rem;
  border-radius: 12px;
  width: 400px;
  box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
}

.form-grid {
  display: grid;
  grid-template-columns: 1fr 2fr;
  gap: 1rem;
  align-items: center;
  margin-top: 1.5rem;
}

.form-grid input {
  width: 100%;
  padding: 8px 12px;
  border-radius: 4px;
  border: 1px solid var(--border-color);
  background: var(--bg-color);
  color: var(--text-main);
  font-weight: 500;
  font-family: inherit;
}

.popup-actions {
  margin-top: 2rem;
  display: flex;
  justify-content: flex-end;
  gap: 1rem;
}

.popup-actions button {
  background: transparent;
  color: var(--text-main);
  border: 1px solid var(--border-color);
  padding: 0.6rem 1.2rem;
  border-radius: 6px;
  font-weight: bold;
  cursor: pointer;
  transition: background 0.2s;
}

.popup-actions button:hover {
  background: var(--border-color);
}

button.primary {
  background: var(--primary-color);
  color: white;
  border: none;
  padding: 0.6rem 1.2rem;
  border-radius: 6px;
  font-weight: bold;
  cursor: pointer;
}
</style>
