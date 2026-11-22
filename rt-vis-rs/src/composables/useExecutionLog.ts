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

import { effectScope, ref, watch, type EffectScope } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type {
  AnomalyEventsPage,
  ClientBudgetStats,
  InstantEventType,
  LogRangeData,
  LogSummary
} from '../types/analyze';
import {
  mockGetAnomalyEvents,
  mockGetLogRange,
  mockLoadLog,
  mockRecomputeClientBudget
} from '../utils/mockAnalyzeLog';
import { useConfig } from './useConfig';
import { useSimulation } from './useSimulation';

// =============================================================================
// Types & Constants
// =============================================================================

/**
 * Toggle between real Tauri IPC and in-memory mock log generator.
 * Set to `true` during offline UI prototyping or benchmarking.
 */
const USE_MOCK_LOG = false;

const FETCH_DEBOUNCE_MS = 50;
const MIN_MARGIN_US = 1_000_000;

// =============================================================================
// Module State (Singleton)
// =============================================================================

// Active log and range dataset for Analyze Mode
const currentLogPathAnalyzeMode = ref<string>('');
const logSummaryAnalyzeMode = ref<LogSummary | null>(null);
const logRangeDataAnalyzeMode = ref<LogRangeData | null>(null);
const isLogLoading = ref<boolean>(false);

// Per-client budget overrides computed when expected_duration_ms is edited in Analyze Mode
const budgetOverridesByCid = ref<Record<number, ClientBudgetStats | null>>({});

// Internal range cache and flight state
const cachedWindowRangeAnalyzeMode = ref<{ start_us: number; end_us: number } | null>(null);
const inFlightRangeAnalyzeMode = ref<{ start_us: number; end_us: number } | null>(null);
let fetchLogRangeTimer: number | null = null;

// Session and per-CID sequence counters to discard stale async budget recomputations
let logSessionSeq = 0;
const recomputeSeqByCid = new Map<number, number>();
const lastExpectedDurationMsByCid = new Map<number, number>();
let budgetWatcherScope: EffectScope | null = null;

// =============================================================================
// Internal Helpers
// =============================================================================

// (none)

// =============================================================================
// Composable Implementation
// =============================================================================

export function useExecutionLog() {
  // ---------------------------------------------------------------------------
  // Dependencies & Inject

  const { configAnalyzeMode, restoreConfigFromLog } = useConfig();
  const { simulatePlan } = useSimulation();

  // ---------------------------------------------------------------------------
  // Local State & Computed

  // (none)

  // ---------------------------------------------------------------------------
  // Methods & Actions

  /**
   * Loads log metadata via load_log IPC (or mock in development).
   * Restores configuration embedded in log and triggers simulation.
   */
  const loadLog = async (): Promise<void> => {
    isLogLoading.value = true;
    try {
      let summary: LogSummary;
      let logPath = '';

      if (USE_MOCK_LOG) {
        summary = await mockLoadLog();
        logPath = 'mock://server.log';
      } else {
        const selectedPath = await open({
          title: 'Select Log File',
          filters: [{ name: 'Scheduler Log File', extensions: ['log', 'txt'] }],
          defaultPath: currentLogPathAnalyzeMode.value || undefined
        });

        if (selectedPath === null) {
          return; // User cancelled
        }

        logPath = selectedPath as string;
        summary = await invoke<LogSummary>('load_log', { path: logPath });
      }

      currentLogPathAnalyzeMode.value = logPath;
      logSummaryAnalyzeMode.value = summary;

      // Reset budget overrides and seed expected durations before restoring config to avoid spurious recomputation
      logSessionSeq += 1;
      budgetOverridesByCid.value = {};
      recomputeSeqByCid.clear();
      lastExpectedDurationMsByCid.clear();
      for (const client of summary.config.client_configs) {
        lastExpectedDurationMsByCid.set(client.client_id, client.expected_duration_ms);
      }

      // Sync restored config from log to configAnalyzeMode
      restoreConfigFromLog(summary.config);

      // Trigger simulation for restored configuration to generate planned executions and metrics
      simulatePlan();

      // Reset range cache so view can request new window
      if (fetchLogRangeTimer !== null) {
        window.clearTimeout(fetchLogRangeTimer);
        fetchLogRangeTimer = null;
      }
      cachedWindowRangeAnalyzeMode.value = null;
      inFlightRangeAnalyzeMode.value = null;
      logRangeDataAnalyzeMode.value = null;

      console.log(`Log loaded successfully (${USE_MOCK_LOG ? 'mock' : logPath}).`, summary);
    } catch (e) {
      console.error('Failed to load log:', e);
      alert(`Failed to load log:\n${e}`);
    } finally {
      isLogLoading.value = false;
    }
  };

  /**
   * Fetches range-restricted actual log data via get_log_range IPC (or mock).
   * Uses margin cache padding, in-flight deduplication, and 50ms debounce to prevent redundant IPC calls during scrolling and resizing.
   */
  const fetchLogRange = (startUs: number, endUs: number): void => {
    if (startUs < 0 || endUs <= startUs) return;

    // Cache hit check: if request fits within existing cached range, bypass IPC
    if (cachedWindowRangeAnalyzeMode.value) {
      if (startUs >= cachedWindowRangeAnalyzeMode.value.start_us && endUs <= cachedWindowRangeAnalyzeMode.value.end_us) {
        return;
      }
    }

    // In-flight check: if request is already covered by an ongoing fetch, bypass IPC
    if (inFlightRangeAnalyzeMode.value) {
      if (startUs >= inFlightRangeAnalyzeMode.value.start_us && endUs <= inFlightRangeAnalyzeMode.value.end_us) {
        return;
      }
    }

    if (fetchLogRangeTimer !== null) {
      window.clearTimeout(fetchLogRangeTimer);
    }

    // If no cache exists at all (initial load), fetch immediately; otherwise debounce by 50ms
    const delay = cachedWindowRangeAnalyzeMode.value ? FETCH_DEBOUNCE_MS : 0;

    fetchLogRangeTimer = window.setTimeout(async () => {
      fetchLogRangeTimer = null;

      // Re-check cache in case another fetch completed in the meantime
      if (cachedWindowRangeAnalyzeMode.value) {
        if (startUs >= cachedWindowRangeAnalyzeMode.value.start_us && endUs <= cachedWindowRangeAnalyzeMode.value.end_us) {
          return;
        }
      }

      // Compute 100% margin padding (min 1,000,000us margin) and clamp to log bounds
      const totalDurationUs = logSummaryAnalyzeMode.value?.total_duration_us ?? Infinity;
      const windowWidth = endUs - startUs;
      const margin = Math.max(MIN_MARGIN_US, windowWidth);
      const reqStart = Math.max(0, Math.floor(startUs - margin));
      const reqEnd = Math.min(totalDurationUs, Math.ceil(endUs + margin));

      inFlightRangeAnalyzeMode.value = { start_us: reqStart, end_us: reqEnd };

      try {
        const rangeData = USE_MOCK_LOG
          ? await mockGetLogRange(reqStart, reqEnd)
          : await invoke<LogRangeData>('get_log_range', { startUs: reqStart, endUs: reqEnd });
        logRangeDataAnalyzeMode.value = rangeData;
        cachedWindowRangeAnalyzeMode.value = { start_us: reqStart, end_us: reqEnd };
      } catch (e) {
        console.error('Failed to fetch log range:', e);
      } finally {
        inFlightRangeAnalyzeMode.value = null;
      }
    }, delay);
  };

  /**
   * Fetches a filtered, paginated slice of cross-client anomaly events from backend or mock store.
   */
  const fetchAnomalyEvents = async (
    cid: number | null,
    eventTypes: InstantEventType[],
    offset: number,
    limit: number
  ): Promise<AnomalyEventsPage> => {
    if (USE_MOCK_LOG) {
      return mockGetAnomalyEvents(cid, eventTypes, offset, limit);
    }
    return invoke<AnomalyEventsPage>('get_anomaly_events', {
      cid,
      eventTypes,
      offset,
      limit
    });
  };

  /**
   * Recomputes budget alignment statistics for a client against a candidate expected_duration_ms
   * without mutating the immutable baseline LogSummary.stats.
   */
  const recomputeClientBudget = async (cid: number, expectedDurationMs: number): Promise<void> => {
    if (!logSummaryAnalyzeMode.value || !Number.isFinite(expectedDurationMs)) {
      return;
    }

    const normalizedMs = Math.max(0, Math.round(expectedDurationMs));
    const session = logSessionSeq;
    const seq = (recomputeSeqByCid.get(cid) ?? 0) + 1;
    recomputeSeqByCid.set(cid, seq);

    try {
      const result = USE_MOCK_LOG
        ? await mockRecomputeClientBudget(cid, normalizedMs)
        : await invoke<ClientBudgetStats | null>('recompute_client_budget', {
          cid,
          expectedDurationMs: normalizedMs
        });

      // Discard stale responses if another log was loaded or a newer request was issued for this CID
      if (session !== logSessionSeq || recomputeSeqByCid.get(cid) !== seq) {
        return;
      }

      budgetOverridesByCid.value = {
        ...budgetOverridesByCid.value,
        [cid]: result
      };
    } catch (e) {
      console.error(`Failed to recompute client budget for CID ${cid}:`, e);
    }
  };

  /**
   * Returns the effective budget stats for a client, preferring any live override over the baseline.
   */
  const getEffectiveClientBudget = (cid: number): ClientBudgetStats | null => {
    if (Object.prototype.hasOwnProperty.call(budgetOverridesByCid.value, cid)) {
      return budgetOverridesByCid.value[cid];
    }
    return logSummaryAnalyzeMode.value?.stats.clients.find(c => c.cid === cid)?.budget ?? null;
  };

  // ---------------------------------------------------------------------------
  // Watchers & Reactive Triggers

  // Register a single detached watcher to keep client budget overrides synchronized with Analyze config edits
  if (!budgetWatcherScope) {
    budgetWatcherScope = effectScope(true);
    budgetWatcherScope.run(() => {
      watch(
        () => configAnalyzeMode.client_configs.map(c => [c.data.client_id, c.data.expected_duration_ms] as const),
        entries => {
          const summary = logSummaryAnalyzeMode.value;
          if (!summary) {
            return;
          }

          const currentCids = new Set(entries.map(([cid]) => cid));
          for (const cachedCid of lastExpectedDurationMsByCid.keys()) {
            if (!currentCids.has(cachedCid)) {
              lastExpectedDurationMsByCid.delete(cachedCid);
            }
          }

          for (const [cid, expectedMs] of entries) {
            const prevMs = lastExpectedDurationMsByCid.get(cid);
            lastExpectedDurationMsByCid.set(cid, expectedMs);

            if (prevMs === expectedMs) {
              continue;
            }

            const baselineClient = summary.stats.clients.find(c => c.cid === cid);
            if (!baselineClient || baselineClient.duration === null) {
              continue;
            }

            const normalizedMs = Math.max(0, Math.round(expectedMs));
            if (
              baselineClient.budget !== null &&
              normalizedMs * 1_000 === baselineClient.budget.expected_duration_us
            ) {
              // Invalidate any in-flight recomputation and restore baseline by removing override
              recomputeSeqByCid.set(cid, (recomputeSeqByCid.get(cid) ?? 0) + 1);
              if (Object.prototype.hasOwnProperty.call(budgetOverridesByCid.value, cid)) {
                const { [cid]: _removed, ...rest } = budgetOverridesByCid.value;
                budgetOverridesByCid.value = rest;
              }
            } else {
              void recomputeClientBudget(cid, normalizedMs);
            }
          }
        }
      );
    });
  }

  // ---------------------------------------------------------------------------
  // Lifecycle Hooks & Observers

  // (none)

  // ---------------------------------------------------------------------------
  // Public API (Return)

  return {
    currentLogPathAnalyzeMode,
    logSummaryAnalyzeMode,
    logRangeDataAnalyzeMode,
    isLogLoading,
    budgetOverridesByCid,
    loadLog,
    fetchLogRange,
    fetchAnomalyEvents,
    recomputeClientBudget,
    getEffectiveClientBudget
  };
}
