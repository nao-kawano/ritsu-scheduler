<script setup lang="ts">
import { ref } from 'vue';
import { useAppState } from '../composables/useAppState';

// --- State and Composables ---
const { config, config_errors, openEdit, addProcess } = useAppState();

// -----------------------------------------------------------------------------
// Props and Emits

const emit = defineEmits<{
  (e: 'scroll', event: Event): void
}>();

// --- Viewport and Scrolling ---

const scrollEl = ref<HTMLElement | null>(null);

const onScroll = (e: Event) => {
  emit('scroll', e);
};

// --- Validation and Error Helpers ---

const getErrors = (cid: number) => {
  return config_errors.value[cid] || [];
};

// -----------------------------------------------------------------------------
// Expose

defineExpose({ scrollEl });
</script>

<template>
  <aside class="process-list-pane" :key="config.sessionId">
    <div class="pane-header">Processes</div>
    <div class="scroll-area process-list-scroll sb-hide-all" ref="scrollEl" @scroll="onScroll">
      <div class="process-list-content">
        <div v-for="clientWrap in config.client_configs" :key="clientWrap.configId" class="process-row-wrapper">
          <div class="process-card" :class="{ 'has-error': getErrors(clientWrap.data.client_id).length > 0 }"
            @click="openEdit(clientWrap)" :title="getErrors(clientWrap.data.client_id).join('\n')">
            <div class="card-header">
              <div class="cid">
                CID: {{ String(clientWrap.data.client_id).padStart(3, '0') }}
                <span v-if="getErrors(clientWrap.data.client_id).length > 0" class="error-icon">⚠️</span>
              </div>
            </div>
            <div class="meta-block">
              <div class="details">C: {{ clientWrap.data.cycle }}, O: {{ clientWrap.data.cycle_offset }}, D: {{
                clientWrap.data.expected_duration_ms }}ms</div>
              <div class="depends" :class="{ 'no-deps': clientWrap.data.depends.length === 0 }">Deps: {{
                clientWrap.data.depends.length > 0 ? clientWrap.data.depends.join(', ') : '-' }}</div>
            </div>
          </div>
        </div>
        <div class="process-row-wrapper add-btn-row">
          <button class="add-btn" @click="addProcess">+ Add Process</button>
        </div>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.process-list-pane {
  background-color: var(--pane-bg);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  height: 100%;
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

.process-list-scroll {
  overflow-y: scroll;
  overflow-x: hidden;
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

.process-card.has-error {
  border-color: #ff4d4f;
  background-color: #fff1f0;
}

.process-card.has-error:hover {
  box-shadow: 0 2px 8px rgba(255, 77, 79, 0.2);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.process-card .cid {
  font-weight: bold;
  font-size: 1.05rem;
}

.error-icon {
  font-size: 0.9rem;
  margin-left: 4px;
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
</style>
