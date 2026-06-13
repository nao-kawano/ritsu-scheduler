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
import { Ref } from 'vue';

/**
 * Provides strict scroll synchronization between the main panes of the visualizer.
 *
 * Uses a "driving element" lock pattern to completely eliminate infinite echo loops
 * while allowing pixel-perfect native scrolling and momentum.
 */
export function useScrollSync(
  processListScroll: Ref<HTMLElement | null>,
  timelineHeaderScroll: Ref<HTMLElement | null>,
  timelineScroll: Ref<HTMLElement | null>,
  metricsHeaderScroll: Ref<HTMLElement | null>,
  metricsChartScroll: Ref<HTMLElement | null>
) {
  let driving: HTMLElement | null = null;
  let drivingTimeout: number | null = null;

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
      if (target === processListScroll.value) {
        if (timelineScroll.value) timelineScroll.value.scrollTop = target.scrollTop;
      }
      else if (target === timelineScroll.value) {
        if (processListScroll.value) processListScroll.value.scrollTop = target.scrollTop;
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

  return {
    onProcessListScroll: onScroll,
    onTimelineScroll: onScroll,
    onMetricsScroll: onScroll
  };
}
