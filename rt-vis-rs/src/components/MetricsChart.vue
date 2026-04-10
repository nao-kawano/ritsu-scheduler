<script setup lang="ts">
import { ref } from 'vue';

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
  <div class="metrics-content-pane">
    <div class="timeline-header hide-scrollbar" ref="headerScrollEl">
      <div class="time-axis">
        <div v-for="n in 50" :key="n" class="time-tick">{{ (n - 1) * 5 }}ms</div>
      </div>
    </div>
    <div class="metrics-chart-scroll" ref="contentScrollEl" @scroll="onScroll">
      <div class="metrics-timeline">
        <div class="placeholder">Metrics Graph Area (Synced with Timeline)</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.metrics-content-pane {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  height: 100%;
  background-color: var(--pane-bg);
}

.timeline-header {
  height: var(--header-row-height);
  overflow: hidden;
  padding-right: 10px;
  flex-shrink: 0;
  border-bottom: 1px solid var(--border-color);
  background: rgba(0, 0, 0, 0.02);
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

.metrics-chart-scroll {
  flex: 1;
  overflow-x: scroll;
  overflow-y: hidden;
  padding-right: 10px;
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
</style>
