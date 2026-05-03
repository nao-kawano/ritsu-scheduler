<script setup lang="ts">
import { ref } from 'vue';
import { useAppState } from '../composables/useAppState';

// --- State and Composables ---
const { config } = useAppState();

// --- Viewport and Scrolling ---
const headerScrollEl = ref<HTMLElement | null>(null);
const contentScrollEl = ref<HTMLElement | null>(null);

const emit = defineEmits<{
  (e: 'scroll', event: Event): void
}>();

const onScroll = (e: Event) => {
  emit('scroll', e);
};

defineExpose({
  headerScrollEl,
  contentScrollEl
});
</script>

<template>
  <main class="timeline-pane" :key="config.sessionId">
    <div class="timeline-header hide-scrollbar" ref="headerScrollEl">
      <div class="time-axis">
        <div v-for="n in 50" :key="n" class="time-tick">{{ (n - 1) * 5 }}ms</div>
      </div>
    </div>
    <div class="scroll-area timeline-scroll" ref="contentScrollEl" @scroll="onScroll">
      <div class="timeline-container">
        <div v-for="clientWrap in config.client_configs" :key="clientWrap.configId" class="timeline-row">
          <div class="plan-preview">Timeline Row for CID {{ clientWrap.data.client_id }}</div>
        </div>
        <div class="timeline-row add-btn-placeholder"></div>
      </div>
    </div>
  </main>
</template>

<style scoped>
.timeline-pane {
  display: flex;
  flex-direction: column;
  background-color: var(--pane-bg);
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  height: 100%;
}

.timeline-header {
  height: var(--header-row-height);
  overflow: hidden;
  padding-right: 10px;
  flex-shrink: 0;
  border-bottom: 1px solid var(--border-color);
  background: rgba(0, 0, 0, 0.02);
}

.scroll-area {
  flex: 1;
  min-height: 0;
  min-width: 0;
}

.timeline-scroll {
  overflow-y: scroll;
  overflow-x: hidden;
  width: 100%;
  height: 100%;
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
</style>
