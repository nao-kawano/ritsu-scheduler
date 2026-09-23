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

import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { LogSummary, LogRangeData } from '../types/analyze';
import { mockLoadLog, mockGetLogRange } from '../utils/mockAnalyzeLog';
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
const MIN_MARGIN_MS = 1000;

// =============================================================================
// Module State (Singleton)
// =============================================================================

// Active log and range dataset for Analyze Mode
const currentLogPathAnalyzeMode = ref<string>('');
const logSummaryAnalyzeMode = ref<LogSummary | null>(null);
const logRangeDataAnalyzeMode = ref<LogRangeData | null>(null);
const isLogLoading = ref<boolean>(false);

// Internal range cache and flight state
const cachedWindowRangeAnalyzeMode = ref<{ start_ms: number; end_ms: number } | null>(null);
const inFlightRangeAnalyzeMode = ref<{ start_ms: number; end_ms: number } | null>(null);
let fetchLogRangeTimer: number | null = null;

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

  const { restoreConfigFromLog } = useConfig();
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
  const fetchLogRange = (startMs: number, endMs: number): void => {
    if (startMs < 0 || endMs <= startMs) return;

    // Cache hit check: if request fits within existing cached range, bypass IPC
    if (cachedWindowRangeAnalyzeMode.value) {
      if (startMs >= cachedWindowRangeAnalyzeMode.value.start_ms && endMs <= cachedWindowRangeAnalyzeMode.value.end_ms) {
        return;
      }
    }

    // In-flight check: if request is already covered by an ongoing fetch, bypass IPC
    if (inFlightRangeAnalyzeMode.value) {
      if (startMs >= inFlightRangeAnalyzeMode.value.start_ms && endMs <= inFlightRangeAnalyzeMode.value.end_ms) {
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
        if (startMs >= cachedWindowRangeAnalyzeMode.value.start_ms && endMs <= cachedWindowRangeAnalyzeMode.value.end_ms) {
          return;
        }
      }

      // Compute 100% margin padding (min 1000ms margin) and clamp to log bounds
      const totalDurationMs = logSummaryAnalyzeMode.value?.total_duration_ms ?? Infinity;
      const windowWidth = endMs - startMs;
      const margin = Math.max(MIN_MARGIN_MS, windowWidth);
      const reqStart = Math.max(0, Math.floor(startMs - margin));
      const reqEnd = Math.min(totalDurationMs, Math.ceil(endMs + margin));

      inFlightRangeAnalyzeMode.value = { start_ms: reqStart, end_ms: reqEnd };

      try {
        const rangeData = USE_MOCK_LOG
          ? await mockGetLogRange(reqStart, reqEnd)
          : await invoke<LogRangeData>('get_log_range', { startMs: reqStart, endMs: reqEnd });
        logRangeDataAnalyzeMode.value = rangeData;
        cachedWindowRangeAnalyzeMode.value = { start_ms: reqStart, end_ms: reqEnd };
      } catch (e) {
        console.error('Failed to fetch log range:', e);
      } finally {
        inFlightRangeAnalyzeMode.value = null;
      }
    }, delay);
  };

  // ---------------------------------------------------------------------------
  // Watchers & Reactive Triggers

  // (none)

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
    loadLog,
    fetchLogRange
  };
}
