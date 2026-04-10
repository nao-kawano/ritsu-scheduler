import { Ref } from 'vue';

/**
 * Provides strict scroll synchronization between the 5 main panes of the visualizer.
 *
 * - Timeline Scroll (Main) -> syncs Process List (Vertical)
 * - Timeline Scroll (Main) -> syncs Timeline Header, Metrics Header & Metrics Chart (Horizontal)
 * - Metrics Chart Scroll   -> syncs Timeline Scroll, Timeline Header & Metrics Header (Horizontal)
 *
 * Uses `requestAnimationFrame` to prevent layout thrashing and an `isSyncing` flag
 * to prevent infinite event loops between mutually synced elements.
 */
export function useScrollSync(
  processListScroll: Ref<HTMLElement | null>,
  timelineHeaderScroll: Ref<HTMLElement | null>,
  timelineScroll: Ref<HTMLElement | null>,
  metricsHeaderScroll: Ref<HTMLElement | null>,
  metricsChartScroll: Ref<HTMLElement | null>
) {
  let isSyncing = false;

  const onTimelineScroll = (e: Event) => {
    if (isSyncing) return;
    isSyncing = true;

    const target = e.target as HTMLElement;

    window.requestAnimationFrame(() => {
      // Vertical sync
      if (processListScroll.value) {
        processListScroll.value.scrollTop = target.scrollTop;
      }

      // Horizontal sync
      if (timelineHeaderScroll.value) {
        timelineHeaderScroll.value.scrollLeft = target.scrollLeft;
      }
      if (metricsHeaderScroll.value) {
        metricsHeaderScroll.value.scrollLeft = target.scrollLeft;
      }
      if (metricsChartScroll.value) {
        metricsChartScroll.value.scrollLeft = target.scrollLeft;
      }
      isSyncing = false;
    });
  };

  const onMetricsScroll = (e: Event) => {
    if (isSyncing) return;
    isSyncing = true;

    const target = e.target as HTMLElement;

    window.requestAnimationFrame(() => {
      // Horizontal sync
      if (timelineScroll.value) {
        timelineScroll.value.scrollLeft = target.scrollLeft;
      }
      if (timelineHeaderScroll.value) {
        timelineHeaderScroll.value.scrollLeft = target.scrollLeft;
      }
      if (metricsHeaderScroll.value) {
        metricsHeaderScroll.value.scrollLeft = target.scrollLeft;
      }
      isSyncing = false;
    });
  };

  return {
    onTimelineScroll,
    onMetricsScroll
  };
}
