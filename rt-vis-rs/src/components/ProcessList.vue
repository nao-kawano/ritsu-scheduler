<script setup lang="ts">
import { ref } from 'vue';
import { useAppState } from '../composables/useAppState';

// --- State and Composables ---
const { config, config_errors, openEdit, addClient } = useAppState();

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
                <span v-if="clientWrap.data.display_name" class="display-name">
                  ({{ clientWrap.data.display_name }})
                </span>
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
          <button class="add-btn" @click="addClient">+ Add Process</button>
        </div>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.process-list-pane {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  height: 100%;
  border-right: var(--rt-border-main);
  background-color: var(--pane-bg);
}

.pane-header {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  height: var(--header-row-height);
  padding: 0 1rem;
  border-bottom: var(--rt-border-main);
  background: var(--rt-bg-header);
  font-size: var(--rt-font-xs);
  font-weight: bold;
  color: var(--text-dim);
  text-transform: uppercase;
}

.scroll-area {
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.process-list-scroll {
  overflow-x: hidden;
  overflow-y: scroll;
}

.process-row-wrapper {
  display: flex;
  align-items: center;
  height: var(--row-height);
  padding: 0.4rem 0.75rem;
  border-bottom: var(--rt-border-main);
}

.process-card {
  display: flex;
  flex-direction: column;
  justify-content: center;
  width: 100%;
  height: 100%;
  padding: 0.4rem 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: var(--rt-radius-m);
  background-color: var(--pane-bg);
  cursor: pointer;
  transition: border-color 0.2s, box-shadow 0.2s, background-color 0.2s;
}

.process-card:hover {
  border-color: var(--primary-color);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.process-card.has-error {
  border-color: #ff4d4f;
  background-color: rgba(255, 77, 79, 0.05);
}

.process-card.has-error:hover {
  box-shadow: 0 2px 8px rgba(255, 77, 79, 0.2);
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.process-card .cid {
  width: 100%;
  overflow: hidden;
  font-size: var(--rt-font-l);
  font-weight: bold;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.display-name {
  margin-left: 4px;
  font-size: var(--rt-font-s);
  font-weight: normal;
  color: var(--text-dim);
}

.meta-block {
  display: flex;
  flex-direction: column;
  gap: 1px;
  margin-top: 2px;
}

.process-card .details {
  font-size: var(--rt-font-xs);
  color: var(--text-dim);
}

.process-card .depends {
  font-size: var(--rt-font-xs);
  font-weight: bold;
  color: var(--accent-color);
}

.process-card .depends.no-deps {
  font-weight: normal;
  color: var(--text-dim);
}

.add-btn {
  width: 100%;
  height: 40px;
  border: 2px dashed var(--border-color);
  border-radius: var(--rt-radius-m);
  background: transparent;
  font-size: var(--rt-font-s);
  font-weight: bold;
  color: var(--text-dim);
  cursor: pointer;
  transition: border-color 0.2s, color 0.2s, background-color 0.2s;
}

.add-btn:hover {
  border-color: var(--primary-color);
  background-color: rgba(57, 108, 216, 0.05);
  color: var(--primary-color);
}
</style>
