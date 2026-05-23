<script setup lang="ts">
import { ref } from 'vue';
import { useAppState } from '../composables/useAppState';

// --- State and Composables ---
const { config, config_errors, openEdit, addClient, moveClientConfig } = useAppState();

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

// --- Drag and Drop State ---

const draggingIndex = ref<number | null>(null);
const targetIndex = ref<number | null>(null);

let scrollAnimationFrameId: number | null = null;
const scrollSpeed = ref(0);
let lastMouseY = 0;

// --- Drag and Drop Handlers ---

/**
 * Handle the start of dragging a process card.
 */
const startDrag = (event: MouseEvent, index: number) => {
  draggingIndex.value = index;
  targetIndex.value = index;
  lastMouseY = event.clientY;

  document.body.classList.add('is-dragging-move');

  window.addEventListener('mousemove', onMouseMove);
  window.addEventListener('mouseup', onMouseUp);

  startScrollLoop();
};

/**
 * Handle mouse movement during dragging.
 */
const onMouseMove = (event: MouseEvent) => {
  lastMouseY = event.clientY;
  updateTargetIndex(event.clientY);
  updateScrollSpeed(event.clientY);
};

/**
 * Update the target drop index based on current mouse coordinates.
 */
const updateTargetIndex = (clientY: number) => {
  const container = scrollEl.value;
  if (!container) return;

  const rect = container.getBoundingClientRect();
  const relativeY = clientY - rect.top + container.scrollTop;

  const rows = container.querySelectorAll('.process-row-wrapper:not(.add-btn-row)');
  let computedIndex = 0;

  for (let i = 0; i < rows.length; i++) {
    const row = rows[i] as HTMLElement;
    const rowTop = row.offsetTop;
    const rowHeight = row.offsetHeight;
    const rowMiddle = rowTop + rowHeight / 2;

    if (relativeY > rowMiddle) {
      computedIndex = i + 1;
    } else {
      break;
    }
  }

  // Boundary Guard: prevent inserting below the add process button
  const boundedIndex = Math.min(config.client_configs.length, Math.max(0, computedIndex));

  if (targetIndex.value !== boundedIndex) {
    targetIndex.value = boundedIndex;
  }
};

/**
 * Update the scrolling speed of the container when dragging near top/bottom boundaries.
 */
const updateScrollSpeed = (clientY: number) => {
  const container = scrollEl.value;
  if (!container) return;

  const rect = container.getBoundingClientRect();
  const localY = clientY - rect.top;

  const threshold = 40;
  const maxSpeed = 10;

  if (localY < threshold) {
    const speed = ((threshold - localY) / threshold) * maxSpeed;
    scrollSpeed.value = -speed;
  } else if (localY > rect.height - threshold) {
    const speed = ((localY - (rect.height - threshold)) / threshold) * maxSpeed;
    scrollSpeed.value = speed;
  } else {
    scrollSpeed.value = 0;
  }
};

/**
 * Start the requestAnimationFrame animation loop for autoscrolling.
 */
const startScrollLoop = () => {
  if (scrollAnimationFrameId !== null) return;

  const loop = () => {
    if (scrollSpeed.value !== 0 && scrollEl.value) {
      scrollEl.value.scrollTop += scrollSpeed.value;
      updateTargetIndex(lastMouseY);
    }
    scrollAnimationFrameId = requestAnimationFrame(loop);
  };
  scrollAnimationFrameId = requestAnimationFrame(loop);
};

/**
 * Stop the autoscrolling animation loop.
 */
const stopScrollLoop = () => {
  if (scrollAnimationFrameId !== null) {
    cancelAnimationFrame(scrollAnimationFrameId);
    scrollAnimationFrameId = null;
  }
  scrollSpeed.value = 0;
};

/**
 * Handle mouseup event to complete or cancel dragging.
 */
const onMouseUp = () => {
  if (draggingIndex.value !== null) {
    document.body.classList.remove('is-dragging-move');
  }

  if (draggingIndex.value !== null && targetIndex.value !== null) {
    moveClientConfig(draggingIndex.value, targetIndex.value);
  }

  draggingIndex.value = null;
  targetIndex.value = null;

  window.removeEventListener('mousemove', onMouseMove);
  window.removeEventListener('mouseup', onMouseUp);

  stopScrollLoop();
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
      <div class="process-list-content" style="position: relative;">
        <template v-for="(clientWrap, index) in config.client_configs" :key="clientWrap.configId">
          <div class="drop-indicator" :class="{ 'is-active': targetIndex === index }"></div>
          <div class="process-row-wrapper" :class="{ 'is-dragging': draggingIndex === index }">
            <div class="drag-handle" @mousedown.prevent.stop="startDrag($event, index)">
              <svg viewBox="0 0 14 24" width="14" height="20" fill="currentColor">
                <circle cx="4" cy="6" r="1.5" />
                <circle cx="4" cy="12" r="1.5" />
                <circle cx="4" cy="18" r="1.5" />
                <circle cx="10" cy="6" r="1.5" />
                <circle cx="10" cy="12" r="1.5" />
                <circle cx="10" cy="18" r="1.5" />
              </svg>
            </div>
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
              <div class="card-meta">
                <div class="details">C: {{ clientWrap.data.cycle }}, O: {{ clientWrap.data.cycle_offset }}, D: {{
                  clientWrap.data.expected_duration_ms }}ms</div>
                <div class="depends" :class="{ 'no-deps': clientWrap.data.depends.length === 0 }">Deps: {{
                  clientWrap.data.depends.length > 0 ? clientWrap.data.depends.join(', ') : '-' }}</div>
              </div>
            </div>
          </div>
        </template>
        <div class="drop-indicator" :class="{ 'is-active': targetIndex === config.client_configs.length }"></div>
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
  background-color: var(--rt-color-surface);
}

.pane-header {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  height: var(--header-row-height);
  padding: 0 1rem;
  border-bottom: var(--rt-border-main);
  background: var(--rt-color-surface-header);
  font-size: var(--rt-font-xs);
  font-weight: bold;
  color: var(--rt-color-text-dim);
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
  /* Box Model */
  display: flex;
  align-items: center;
  height: var(--row-height);
  padding: 0.4rem 0.75rem;
  gap: 0.25rem;

  /* Border/Background */
  border-bottom: var(--rt-border-main);
}

.process-card {
  display: flex;
  flex-direction: column;
  justify-content: center;
  width: 100%;
  height: 100%;
  padding: 0.4rem 0.75rem;
  background-color: var(--rt-color-surface);
  border: 1px solid var(--rt-color-border);
  border-radius: var(--rt-radius-m);
  cursor: pointer;
  transition: border-color 0.2s, box-shadow 0.2s, background-color 0.2s;
}

.process-card:hover {
  border-color: var(--rt-color-primary);
  box-shadow: var(--rt-bshadow-pop);
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.card-header .cid {
  width: 100%;
  overflow: hidden;
  font-size: var(--rt-font-l);
  font-weight: bold;
  color: var(--rt-color-text);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cid .display-name {
  margin-left: 4px;
  font-size: var(--rt-font-s);
  font-weight: normal;
  color: var(--rt-color-text-dim);
}

.card-meta {
  display: flex;
  flex-direction: column;
  gap: 1px;
  margin-top: 2px;
}

.card-meta .details {
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
}

.card-meta .depends {
  font-size: var(--rt-font-xs);
  font-weight: bold;
  color: var(--rt-color-accent);
}

.card-meta .depends.no-deps {
  font-weight: normal;
  color: var(--rt-color-text-dim);
}

.process-card.has-error {
  background-color: var(--rt-color-error-container);
  border-color: var(--rt-color-error);
  color: var(--rt-color-on-error-container);
}

.process-card.has-error:hover {
  box-shadow: var(--rt-bshadow-pop-error);
}

.process-card.has-error .cid,
.process-card.has-error .display-name,
.process-card.has-error .details,
.process-card.has-error .depends,
.process-card.has-error .depends.no-deps {
  color: var(--rt-color-on-error-container);
}

.add-btn {
  width: 100%;
  height: 40px;
  border: 2px dashed var(--rt-color-border);
  border-radius: var(--rt-radius-m);
  background: transparent;
  font-size: var(--rt-font-s);
  font-weight: bold;
  color: var(--rt-color-text-dim);
  cursor: pointer;
  transition: border-color 0.2s, color 0.2s, background-color 0.2s;
}

.add-btn:hover {
  border-color: var(--rt-color-primary);
  background-color: var(--rt-color-surface-header);
  color: var(--rt-color-primary);
}

.drag-handle {
  /* Positioning */
  flex-shrink: 0;

  /* Box Model */
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 24px;

  /* Typography/Color */
  color: var(--rt-color-text-dim);

  /* Misc */
  cursor: grab;
  transition: color 0.2s;
}

.drag-handle:hover {
  color: var(--rt-color-primary);
}

.drag-handle:active {
  cursor: grabbing;
}

.process-row-wrapper.is-dragging {
  opacity: 0.5;
}

.drop-indicator {
  /* Box Model */
  height: 4px;
  margin: -2px 0.75rem;

  /* Border/Background */
  background-color: transparent;

  /* Misc */
  transition: background-color 0.15s, box-shadow 0.15s;
}

.drop-indicator.is-active {
  /* Border/Background */
  background-color: var(--rt-color-primary);
  border-radius: var(--rt-radius-s);
  box-shadow: 0 0 8px var(--rt-color-primary);
}
</style>
