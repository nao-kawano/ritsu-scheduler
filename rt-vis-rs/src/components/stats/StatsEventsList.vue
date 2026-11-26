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

<!-- ========================================================================== -->
<!-- Script Section                                                             -->
<!-- ========================================================================== -->
<script setup lang="ts">
// -----------------------------------------------------------------------------
// Imports

import { computed, onUnmounted, ref, watch } from 'vue';
import { useConfig } from '../../composables/useConfig';
import { useExecutionLog } from '../../composables/useExecutionLog';
import { useStatsPanel } from '../../composables/useStatsPanel';
import { formatCid, formatUsAsMs } from '../../utils/format';
import {
  ANOMALY_EVENT_LABELS,
  ANOMALY_EVENT_TYPES,
  type AnomalyEventItem,
  type AnomalyEventType
} from '../../types/analyze';
import AppIcon from '../AppIcon.vue';

// -----------------------------------------------------------------------------
// Global State & Composables

const { configAnalyzeMode } = useConfig();
const { fetchAnomalyEvents, logSummaryAnalyzeMode } = useExecutionLog();
const { eventTypeFilter, toggleEventTypeFilter } = useStatsPanel();

// -----------------------------------------------------------------------------
// Props & Emits

const props = defineProps<{
  cid: number | null;
  eventCounts: Readonly<Record<AnomalyEventType, number>>;
}>();

const emit = defineEmits<{
  (e: 'jump', timeUs: number, cid: number): void;
}>();

// -----------------------------------------------------------------------------
// Types & Interfaces

interface VisibleRow {
  index: number;
  item: AnomalyEventItem | null;
}

// -----------------------------------------------------------------------------
// Constants & Layout

const ROW_HEIGHT = 54;
const PAGE_SIZE = 100;
const OVERSCAN = 5;
const SCROLL_DEBOUNCE_MS = 50;
const DEFAULT_VIEWPORT_HEIGHT = 360;

// -----------------------------------------------------------------------------
// Local State & Computed

// --- Virtual Scroll Viewport ---

const scrollContainerEl = ref<HTMLElement | null>(null);
const scrollTop = ref<number>(0);
const viewportHeight = ref<number>(DEFAULT_VIEWPORT_HEIGHT);

// --- Paged Cache & Async Race Control ---

const totalCount = ref<number>(0);
const pagesCache = ref<Map<number, AnomalyEventItem[]>>(new Map());
const isInitialLoading = ref<boolean>(false);

// Generation counter and in-flight set prevent stale responses from polluting current filter results
const loadingPages = new Set<number>();
let requestGeneration = 0;
let fetchTimer: number | null = null;

/**
 * Maps configured client IDs to their display names for row label formatting.
 */
const cidToDisplayName = computed(() => {
  const map = new Map<number, string>();
  for (const c of configAnalyzeMode.client_configs) {
    map.set(c.data.client_id, c.data.display_name);
  }
  return map;
});

const activeEventTypes = computed<AnomalyEventType[]>(() => {
  return ANOMALY_EVENT_TYPES.filter(type => eventTypeFilter.value.has(type));
});

const filterKey = computed<string>(() => activeEventTypes.value.join(','));

const totalListHeight = computed<number>(() => totalCount.value * ROW_HEIGHT);

/**
 * Computes the virtualized row index window [startIdx, endIdx) including overscan buffer.
 */
const visibleRange = computed<{ startIdx: number; endIdx: number }>(() => {
  if (totalCount.value === 0) {
    return { startIdx: 0, endIdx: 0 };
  }
  const startIdx = Math.max(0, Math.floor(scrollTop.value / ROW_HEIGHT) - OVERSCAN);
  const endIdx = Math.min(totalCount.value, Math.ceil((scrollTop.value + viewportHeight.value) / ROW_HEIGHT) + OVERSCAN);
  return { startIdx, endIdx };
});

/**
 * Projects the visible index range onto cached page items, yielding null for pending pages.
 */
const visibleRows = computed<VisibleRow[]>(() => {
  const { startIdx, endIdx } = visibleRange.value;
  const rows: VisibleRow[] = [];
  for (let idx = startIdx; idx < endIdx; idx++) {
    const pageIdx = Math.floor(idx / PAGE_SIZE);
    const pageOffset = idx % PAGE_SIZE;
    const pageItems = pagesCache.value.get(pageIdx);
    rows.push({
      index: idx,
      item: pageItems && pageOffset < pageItems.length ? pageItems[pageOffset] : null
    });
  }
  return rows;
});

// -----------------------------------------------------------------------------
// Methods & Logic

/**
 * Loads a single page of filtered anomaly events and discards stale responses.
 */
const loadPage = async (pageIdx: number, gen: number): Promise<void> => {
  // Skip duplicate requests and avoid backend queries when no event type is active
  if (loadingPages.has(pageIdx) || pagesCache.value.has(pageIdx)) {
    return;
  }
  const types = activeEventTypes.value;
  if (types.length === 0) {
    return;
  }

  // Fetch page and replace cache map immutably if the request generation is still current
  loadingPages.add(pageIdx);
  try {
    const page = await fetchAnomalyEvents(props.cid, types, pageIdx * PAGE_SIZE, PAGE_SIZE);
    if (gen !== requestGeneration) {
      return;
    }

    totalCount.value = page.total_count;
    const nextCache = new Map(pagesCache.value);
    nextCache.set(pageIdx, page.items);
    pagesCache.value = nextCache;
  } catch (e) {
    console.error('Failed to load anomaly events page:', e);
  } finally {
    // Guard cleanup by generation so a stale response never clears the next filter's loading state
    if (gen === requestGeneration) {
      loadingPages.delete(pageIdx);
      if (pageIdx === 0) {
        isInitialLoading.value = false;
      }
    }
  }
};

/**
 * Ensures all pages intersecting the current visible row range are scheduled for loading.
 */
const ensureVisiblePages = (delayMs: number): void => {
  // Cancel any pending scroll-debounced fetch before recalculating needed pages
  if (fetchTimer !== null) {
    window.clearTimeout(fetchTimer);
    fetchTimer = null;
  }

  // Abort scheduling when no event filter is selected or no log is loaded
  if (activeEventTypes.value.length === 0 || !logSummaryAnalyzeMode.value) {
    return;
  }

  // Collect uncached pages intersecting the viewport, bootstrapping page 0 before totalCount is known
  const { startIdx, endIdx } = visibleRange.value;
  const neededPages: number[] = [];
  if (totalCount.value === 0 && !pagesCache.value.has(0)) {
    neededPages.push(0);
  } else if (endIdx > startIdx) {
    const firstPage = Math.floor(startIdx / PAGE_SIZE);
    const lastPage = Math.floor((endIdx - 1) / PAGE_SIZE);
    for (let p = firstPage; p <= lastPage; p++) {
      if (!pagesCache.value.has(p) && !loadingPages.has(p)) {
        neededPages.push(p);
      }
    }
  }

  if (neededPages.length === 0) {
    return;
  }

  // Dispatch page fetches immediately on filter reset or after debounce during rapid scrolling
  const gen = requestGeneration;
  const dispatchLoads = (): void => {
    for (const p of neededPages) {
      void loadPage(p, gen);
    }
  };

  if (delayMs === 0) {
    dispatchLoads();
  } else {
    fetchTimer = window.setTimeout(() => {
      fetchTimer = null;
      if (gen !== requestGeneration) {
        return;
      }
      dispatchLoads();
    }, delayMs);
  }
};

/**
 * Formats a client identifier with its configured display name when available.
 */
const formatCidLabel = (cid: number): string => {
  return formatCid(cid, cidToDisplayName.value.get(cid));
};

// -----------------------------------------------------------------------------
// Event Handlers

/**
 * Updates virtual scroll viewport offset and schedules page loading.
 */
const onScroll = (): void => {
  if (!scrollContainerEl.value) {
    return;
  }
  scrollTop.value = scrollContainerEl.value.scrollTop;
  ensureVisiblePages(SCROLL_DEBOUNCE_MS);
};

// -----------------------------------------------------------------------------
// Watchers & Reactive Triggers

// Reset paging cache, scroll position, and request generation whenever target, filter, or log changes
watch(
  [() => props.cid, filterKey, logSummaryAnalyzeMode],
  () => {
    // Advance generation and cancel any scheduled fetch to invalidate in-flight responses
    requestGeneration += 1;
    if (fetchTimer !== null) {
      window.clearTimeout(fetchTimer);
      fetchTimer = null;
    }

    // Clear cached pages and reset scroll offset to top for the new query scope
    pagesCache.value = new Map();
    loadingPages.clear();
    totalCount.value = 0;
    scrollTop.value = 0;
    if (scrollContainerEl.value) {
      scrollContainerEl.value.scrollTop = 0;
    }

    // Skip fetching when empty filter or unloaded log, otherwise fetch page 0 immediately
    if (activeEventTypes.value.length === 0 || !logSummaryAnalyzeMode.value) {
      isInitialLoading.value = false;
    } else {
      isInitialLoading.value = true;
      ensureVisiblePages(0);
    }
  },
  { immediate: true }
);

// -----------------------------------------------------------------------------
// Lifecycle Hooks & Observers

// Track scroll container height changes so newly exposed virtual rows load automatically
const containerResizeObserver = new ResizeObserver(entries => {
  for (const entry of entries) {
    const height = entry.contentRect.height;
    if (height > 0) {
      viewportHeight.value = height;
      ensureVisiblePages(SCROLL_DEBOUNCE_MS);
    }
  }
});

// Re-bind ResizeObserver whenever v-if mounts or unmounts the scroll container element
watch(
  scrollContainerEl,
  (newEl, oldEl) => {
    if (oldEl) {
      containerResizeObserver.unobserve(oldEl);
    }
    if (newEl) {
      viewportHeight.value = newEl.clientHeight || DEFAULT_VIEWPORT_HEIGHT;
      containerResizeObserver.observe(newEl);
    }
  },
  { immediate: true, flush: 'post' }
);

onUnmounted(() => {
  if (fetchTimer !== null) {
    window.clearTimeout(fetchTimer);
    fetchTimer = null;
  }
  containerResizeObserver.disconnect();
});

// -----------------------------------------------------------------------------
// Expose & Exports

// (none)
</script>

<!-- ========================================================================== -->
<!-- Template Section                                                           -->
<!-- ========================================================================== -->
<template>
  <div class="events-list-root">
    <!-- Filter Chips Bar -->
    <div class="events-filter-bar">
      <div class="events-filter-chips">
        <button v-for="type in ANOMALY_EVENT_TYPES" :key="type" type="button" class="events-filter-chip"
          :class="[`event-${type}`, { 'is-active stats-event-fill': eventTypeFilter.has(type) }]"
          @click="toggleEventTypeFilter(type)">
          <span>{{ ANOMALY_EVENT_LABELS[type] }}</span>
          <span class="events-chip-count">{{ eventCounts[type] }}</span>
        </button>
      </div>
      <div class="events-meta-row">
        <span v-if="activeEventTypes.length === 0">No event types selected</span>
        <span v-else-if="isInitialLoading">Loading events...</span>
        <span v-else>Total: {{ totalCount }} events</span>
      </div>
    </div>

    <!-- Virtual Scroll List -->
    <div ref="scrollContainerEl" class="events-scroll-area" @scroll="onScroll">
      <div v-if="activeEventTypes.length === 0" class="stats-empty-state">
        Select at least one anomaly type above to inspect events.
      </div>
      <div v-else-if="isInitialLoading && totalCount === 0" class="stats-empty-state">
        Loading events...
      </div>
      <div v-else-if="totalCount === 0" class="stats-empty-state">
        No matching anomaly events recorded.
      </div>
      <div v-else class="events-virtual-spacer" :style="{ height: `${totalListHeight}px` }">
        <template v-for="row in visibleRows" :key="row.index">
          <button v-if="row.item" type="button" class="events-row" :style="{ top: `${row.index * ROW_HEIGHT}px` }"
            @click="emit('jump', row.item.time_us, row.item.cid)">
            <div class="events-row-top">
              <div class="events-row-badges">
                <span class="stats-event-badge stats-event-fill" :class="`event-${row.item.event_type}`">
                  {{ row.item.event_type.toUpperCase() }}
                </span>
                <span v-if="cid === null" class="events-cid-label" :title="formatCidLabel(row.item.cid)">
                  {{ formatCidLabel(row.item.cid) }}
                </span>
              </div>
              <AppIcon class="events-jump-icon" name="jump" :size="12" />
            </div>
            <div class="events-row-bottom">
              <span>Cycle #{{ row.item.cycle }}</span>
              <span class="events-divider">|</span>
              <span>{{ formatUsAsMs(row.item.time_us) }}</span>
              <span class="events-divider">|</span>
              <span>L{{ row.item.log_line_no }}</span>
            </div>
          </button>
          <div v-else class="events-row is-loading" :style="{ top: `${row.index * ROW_HEIGHT}px` }">
            Loading...
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<!-- ========================================================================== -->
<!-- Style Section                                                              -->
<!-- ========================================================================== -->
<style scoped>
/* -----------------------------------------------------------------------------
 * Layout & Containers
 * ----------------------------------------------------------------------------- */

.events-list-root {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.events-filter-bar {
  display: flex;
  flex-shrink: 0;
  flex-direction: column;
  gap: 6px;
  padding: 10px 12px 8px;
  border-bottom: var(--rt-border-main);
  background-color: var(--rt-color-surface);
}

.events-filter-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.events-scroll-area {
  position: relative;
  flex: 1;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: none;
}

.events-virtual-spacer {
  position: relative;
  width: 100%;
}

/* -----------------------------------------------------------------------------
 * Components & Elements
 * ----------------------------------------------------------------------------- */

.events-filter-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 7px;
  border: 1px solid var(--rt-color-border);
  border-radius: var(--rt-radius-s);
  background-color: transparent;
  font-family: inherit;
  font-size: var(--rt-font-xs);
  font-weight: 600;
  color: var(--rt-color-text-dim);
  opacity: 0.55;
  cursor: pointer;
  transition: opacity 0.15s, background-color 0.15s, border-color 0.15s, color 0.15s;
}

.events-chip-count {
  font-family: var(--rt-font-mono);
  font-size: 10px;
  font-variant-numeric: tabular-nums;
}

.events-meta-row {
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
  font-variant-numeric: tabular-nums;
}

.events-row {
  position: absolute;
  left: 0;
  right: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 4px;
  width: 100%;
  height: v-bind('ROW_HEIGHT + "px"');
  padding: 6px 12px;
  border: none;
  border-bottom: var(--rt-border-main);
  background-color: transparent;
  font-family: inherit;
  text-align: left;
  color: var(--rt-color-text);
  cursor: pointer;
  transition: background-color 0.12s;
}

.events-row-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  min-width: 0;
}

.events-row-badges {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.events-cid-label {
  overflow: hidden;
  font-size: var(--rt-font-xs);
  font-weight: 600;
  color: var(--rt-color-text);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.events-jump-icon {
  flex-shrink: 0;
  color: var(--rt-color-text-dim);
  opacity: 0.6;
  transition: opacity 0.12s, color 0.12s;
}

.events-row-bottom {
  display: flex;
  align-items: center;
  gap: 6px;
  overflow: hidden;
  font-family: var(--rt-font-mono);
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.events-divider {
  opacity: 0.4;
}

/* -----------------------------------------------------------------------------
 * Overlays & Tooltips
 * ----------------------------------------------------------------------------- */

/* (none) */

/* -----------------------------------------------------------------------------
 * States & Modifiers
 * ----------------------------------------------------------------------------- */

.events-filter-chip:hover {
  opacity: 0.85;
}

.events-filter-chip.is-active {
  border-color: transparent;
  opacity: 1;
}

.events-row:not(.is-loading):hover {
  background-color: var(--rt-color-surface-header);
}

.events-row:not(.is-loading):hover .events-jump-icon {
  color: var(--rt-color-primary);
  opacity: 1;
}

.events-row.is-loading {
  font-size: var(--rt-font-xs);
  color: var(--rt-color-text-dim);
  cursor: default;
}
</style>
