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

import { watch, onUnmounted, type Ref } from 'vue';

// =============================================================================
// Types & Constants
// =============================================================================

// Standard line height (px) used when normalizing DOM_DELTA_LINE wheel events
const WHEEL_LINE_HEIGHT_PX = 40;

// =============================================================================
// Module State (Singleton)
// =============================================================================

// (none)

// =============================================================================
// Internal Helpers
// =============================================================================

/**
 * Normalizes a horizontal wheel delta (native deltaX or Shift + deltaY) into pixels.
 */
function normalizeHorizontalWheelDelta(event: WheelEvent, container: HTMLElement): number {
  const rawDelta = event.deltaX !== 0 ? event.deltaX : (event.shiftKey ? event.deltaY : 0);
  if (rawDelta === 0) return 0;

  if (event.deltaMode === WheelEvent.DOM_DELTA_LINE) {
    return rawDelta * WHEEL_LINE_HEIGHT_PX;
  }
  if (event.deltaMode === WheelEvent.DOM_DELTA_PAGE) {
    return rawDelta * container.clientWidth;
  }
  return rawDelta;
}

/**
 * Reactively binds a DOM event listener to an element ref and re-binds on DOM replacement.
 * Returns a cleanup function that detaches the currently bound listener.
 */
function bindElementEvent<K extends keyof HTMLElementEventMap>(
  elRef: Ref<HTMLElement | null>,
  type: K,
  listener: (event: HTMLElementEventMap[K]) => void,
  options: AddEventListenerOptions,
  onUnbind?: (oldEl: HTMLElement) => void
): () => void {
  let boundEl: HTMLElement | null = null;

  const unbind = () => {
    if (boundEl) {
      boundEl.removeEventListener(type, listener);
      onUnbind?.(boundEl);
      boundEl = null;
    }
  };

  watch(
    elRef,
    (newEl) => {
      if (boundEl === newEl) return;
      unbind();
      boundEl = newEl;
      if (boundEl) {
        boundEl.addEventListener(type, listener, options);
      }
    },
    { immediate: true, flush: 'post' }
  );

  return unbind;
}

// =============================================================================
// Composable Implementation
// =============================================================================

/**
 * Provides strict scroll synchronization between the main panes of the visualizer.
 *
 * Reactively binds passive `scroll` listeners to scrollable panes and horizontal `wheel`
 * forwarders to time headers, using a "driving element" lock pattern to eliminate echo loops.
 */
export function useScrollSync(
  clientListScroll: Ref<HTMLElement | null>,
  timelineHeaderScroll: Ref<HTMLElement | null>,
  timelineScroll: Ref<HTMLElement | null>,
  metricsHeaderScroll: Ref<HTMLElement | null>,
  metricsChartScroll: Ref<HTMLElement | null>
) {
  // ---------------------------------------------------------------------------
  // Dependencies & Inject

  // (none)

  // ---------------------------------------------------------------------------
  // Local State & Computed

  let driving: HTMLElement | null = null;
  let drivingTimeout: number | null = null;

  // ---------------------------------------------------------------------------
  // Methods & Actions

  const releaseDrivingLockIf = (el: HTMLElement) => {
    if (driving !== el) return;
    driving = null;
    if (drivingTimeout !== null) {
      window.clearTimeout(drivingTimeout);
      drivingTimeout = null;
    }
  };

  const onScroll = (e: Event) => {
    const target = e.target as HTMLElement;

    // Ignore echoed scroll events from slave elements
    if (driving && driving !== target) {
      return;
    }

    // Set this element as the current driving master
    driving = target;

    // Clear any existing timeout
    if (drivingTimeout !== null) {
      window.clearTimeout(drivingTimeout);
    }

    // Release the lock 50ms after the last scroll event from the driver
    drivingTimeout = window.setTimeout(() => {
      driving = null;
      drivingTimeout = null;
    }, 50);

    window.requestAnimationFrame(() => {
      if (target === clientListScroll.value) {
        if (timelineScroll.value) timelineScroll.value.scrollTop = target.scrollTop;
      }
      else if (target === timelineScroll.value) {
        if (clientListScroll.value) clientListScroll.value.scrollTop = target.scrollTop;
        if (timelineHeaderScroll.value) timelineHeaderScroll.value.scrollLeft = target.scrollLeft;
        if (metricsHeaderScroll.value) metricsHeaderScroll.value.scrollLeft = target.scrollLeft;
        if (metricsChartScroll.value) metricsChartScroll.value.scrollLeft = target.scrollLeft;
      }
      else if (target === metricsChartScroll.value) {
        if (timelineScroll.value) timelineScroll.value.scrollLeft = target.scrollLeft;
        if (timelineHeaderScroll.value) timelineHeaderScroll.value.scrollLeft = target.scrollLeft;
        if (metricsHeaderScroll.value) metricsHeaderScroll.value.scrollLeft = target.scrollLeft;
      }
    });
  };

  const forwardHorizontalWheel = (event: WheelEvent, targetEl: HTMLElement | null) => {
    if (!targetEl) return;

    const deltaPx = normalizeHorizontalWheelDelta(event, targetEl);
    if (deltaPx === 0) return;

    // Prevent browser back/forward swipe navigation on horizontal overscroll
    event.preventDefault();
    targetEl.scrollLeft += deltaPx;
  };

  const onTimelineHeaderWheel = (event: WheelEvent) => {
    forwardHorizontalWheel(event, timelineScroll.value);
  };

  const onMetricsHeaderWheel = (event: WheelEvent) => {
    forwardHorizontalWheel(event, metricsChartScroll.value);
  };

  // ---------------------------------------------------------------------------
  // Watchers & Reactive Triggers

  const cleanups: Array<() => void> = [
    bindElementEvent(clientListScroll, 'scroll', onScroll, { passive: true }, releaseDrivingLockIf),
    bindElementEvent(timelineScroll, 'scroll', onScroll, { passive: true }, releaseDrivingLockIf),
    bindElementEvent(metricsChartScroll, 'scroll', onScroll, { passive: true }, releaseDrivingLockIf),
    bindElementEvent(timelineHeaderScroll, 'wheel', onTimelineHeaderWheel, { passive: false }),
    bindElementEvent(metricsHeaderScroll, 'wheel', onMetricsHeaderWheel, { passive: false })
  ];

  // ---------------------------------------------------------------------------
  // Lifecycle Hooks & Observers

  onUnmounted(() => {
    driving = null;
    if (drivingTimeout !== null) {
      window.clearTimeout(drivingTimeout);
      drivingTimeout = null;
    }
    for (const cleanup of cleanups) {
      cleanup();
    }
  });

  // ---------------------------------------------------------------------------
  // Public API (Return)

  // (none)
}
