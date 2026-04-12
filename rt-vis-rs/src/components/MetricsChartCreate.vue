<script setup lang="ts">
import { ref } from 'vue';
import { useTimeScale } from '../composables/useTimeScale';
import { useCreateModeLayout } from '../composables/useCreateModeLayout';

const { cycleTimeMs } = useTimeScale();
const { totalCycles, gridInfo, totalWidth } = useCreateModeLayout();

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
  <div class="metrics-chart">
    <!-- Header (Synced with Timeline Header) -->
    <div class="metrics-header hide-scrollbar" ref="headerScrollEl">
      <div class="metrics-axis" :style="{ width: totalWidth + 'px' }">
        <div v-for="n in totalCycles" :key="n" class="metrics-tick" :style="{ width: gridInfo.majorPx + 'px' }">
          <span class="cycle-label">Cycle {{ n - 1 }}</span>
          <span class="time-label">{{ (n - 1) * cycleTimeMs }}ms</span>
        </div>
      </div>
    </div>

    <!-- Content Area (With Horizontal Scrollbar) -->
    <div class="scroll-area metrics-scroll" ref="contentScrollEl" @scroll="onScroll">
      <div class="metrics-container" :style="{
        width: totalWidth + 'px',
        backgroundSize: `${gridInfo.majorPx}px 100%, ${gridInfo.minorPx}px 100%`
      }">
        <div class="metrics-row">
          <div class="placeholder-text">Concurrent Processes Area Chart</div>
        </div>
        <div class="metrics-row">
          <div class="placeholder-text">Cycle Jitter Line Chart</div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.metrics-chart {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  height: 100%;
}

.metrics-header {
  height: var(--header-row-height);
  overflow: hidden;
  flex-shrink: 0;
  border-bottom: 1px solid var(--border-color);
  background: rgba(0, 0, 0, 0.02);
}

.scroll-area {
  flex: 1;
  min-height: 0;
  min-width: 0;
}

.metrics-scroll {
  overflow-x: scroll;
  overflow-y: hidden;
  width: 100%;
  height: 100%;
}

.metrics-axis {
  display: flex;
  height: 100%;
}

.metrics-tick {
  height: 100%;
  border-right: 1px solid var(--border-color);
  padding: 0 0.5rem;
  display: flex;
  flex-direction: column;
  justify-content: center;
  font-size: 0.7rem;
  flex-shrink: 0;
}

.cycle-label {
  font-weight: bold;
  color: var(--text-main);
}

.time-label {
  color: var(--text-dim);
  font-size: 0.65rem;
}

.metrics-container {
  min-height: 100%;
  background-image:
    linear-gradient(90deg, rgba(128, 128, 128, 0.3) 1px, transparent 1px),
    linear-gradient(90deg, rgba(128, 128, 128, 0.1) 1px, transparent 1px);
  background-position: -1px 0, -1px 0;
}

.metrics-row {
  height: 60px;
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  padding: 0 1rem;
}

.placeholder-text {
  font-size: 0.75rem;
  color: var(--text-dim);
  opacity: 0.4;
}
</style>
