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
import { ANOMALY_EVENT_TYPES, type AnomalyEventType } from '../types/analyze';
import { useApp } from './useApp';
import { useExecutionLog } from './useExecutionLog';

// =============================================================================
// Types & Constants
// =============================================================================

export type StatsTarget =
  | { kind: 'system' }
  | { kind: 'client'; cid: number }
  | { kind: 'concurrency' }
  | { kind: 'jitter' };

export type StatsPanelTab = 'stats' | 'events' | 'outliers';

// =============================================================================
// Module State (Singleton)
// =============================================================================

// Right-side statistics inspector panel visibility and active selection
const isOpen = ref<boolean>(false);
const target = ref<StatsTarget | null>(null);
const activeTab = ref<StatsPanelTab>('stats');
const eventTypeFilter = ref<Set<AnomalyEventType>>(new Set(ANOMALY_EVENT_TYPES));

let panelWatcherScope: EffectScope | null = null;

// =============================================================================
// Internal Helpers
// =============================================================================

/**
 * Checks whether a tab is valid for the given statistics panel target.
 */
const isValidTabForTarget = (currentTarget: StatsTarget, tab: StatsPanelTab): boolean => {
  switch (currentTarget.kind) {
    case 'system':
    case 'client':
      return tab === 'stats' || tab === 'events';
    case 'concurrency':
      return tab === 'stats';
    case 'jitter':
      return tab === 'stats' || tab === 'outliers';
  }
};

// =============================================================================
// Composable Implementation
// =============================================================================

export function useStatsPanel() {
  // ---------------------------------------------------------------------------
  // Dependencies & Inject

  const { mode } = useApp();
  const { logSummaryAnalyzeMode } = useExecutionLog();

  // ---------------------------------------------------------------------------
  // Local State & Computed

  // (none)

  // ---------------------------------------------------------------------------
  // Methods & Actions

  /**
   * Opens or switches the inspector to the System (All Clients) view.
   * Idempotent: re-clicking an active trigger keeps the panel open.
   */
  const openSystem = (
    tab: 'stats' | 'events' = 'stats',
    filterTypes?: readonly AnomalyEventType[]
  ): void => {
    target.value = { kind: 'system' };
    activeTab.value = tab;
    eventTypeFilter.value = new Set(filterTypes ?? ANOMALY_EVENT_TYPES);
    isOpen.value = true;
  };

  /**
   * Opens or switches the inspector to a specific Client view.
   * Idempotent: re-clicking an active trigger keeps the panel open.
   */
  const openClient = (
    cid: number,
    tab: 'stats' | 'events' = 'stats',
    filterTypes?: readonly AnomalyEventType[]
  ): void => {
    target.value = { kind: 'client', cid };
    activeTab.value = tab;
    eventTypeFilter.value = new Set(filterTypes ?? ANOMALY_EVENT_TYPES);
    isOpen.value = true;
  };

  /**
   * Opens or switches the inspector to the Concurrent Clients utilization view.
   * Idempotent: re-clicking an active trigger keeps the panel open.
   */
  const openConcurrency = (): void => {
    target.value = { kind: 'concurrency' };
    activeTab.value = 'stats';
    isOpen.value = true;
  };

  /**
   * Opens or switches the inspector to the Cycle Jitter statistics view.
   * Idempotent: re-clicking an active trigger keeps the panel open.
   */
  const openJitter = (tab: 'stats' | 'outliers' = 'stats'): void => {
    target.value = { kind: 'jitter' };
    activeTab.value = tab;
    isOpen.value = true;
  };

  /**
   * Closes the statistics inspector panel and clears the active target.
   */
  const closePanel = (): void => {
    isOpen.value = false;
    target.value = null;
  };

  /**
   * Switches the active tab inside the panel if valid for the current target.
   */
  const setActiveTab = (tab: StatsPanelTab): void => {
    if (!target.value || !isValidTabForTarget(target.value, tab)) {
      return;
    }
    activeTab.value = tab;
  };

  /**
   * Toggles a single anomaly event type in the active event filter set.
   */
  const toggleEventTypeFilter = (eventType: AnomalyEventType): void => {
    const next = new Set(eventTypeFilter.value);
    if (next.has(eventType)) {
      next.delete(eventType);
    } else {
      next.add(eventType);
    }
    eventTypeFilter.value = next;
  };

  /**
   * Replaces the active anomaly event type filter set.
   */
  const setEventTypeFilter = (eventTypes: readonly AnomalyEventType[]): void => {
    eventTypeFilter.value = new Set(eventTypes);
  };

  /**
   * Resets the anomaly event filter to include all five anomaly types.
   */
  const resetEventTypeFilter = (): void => {
    eventTypeFilter.value = new Set(ANOMALY_EVENT_TYPES);
  };

  /**
   * Returns true if the panel is currently open on the specified target (and CID for client targets).
   */
  const isTargetActive = (kind: StatsTarget['kind'], cid?: number): boolean => {
    if (!isOpen.value || !target.value || target.value.kind !== kind) {
      return false;
    }
    if (kind === 'client') {
      return target.value.kind === 'client' && target.value.cid === cid;
    }
    return true;
  };

  // ---------------------------------------------------------------------------
  // Watchers & Reactive Triggers

  // Register a single detached watcher to close incompatible targets across mode switches or log reloads
  if (!panelWatcherScope) {
    panelWatcherScope = effectScope(true);
    panelWatcherScope.run(() => {
      watch([mode, logSummaryAnalyzeMode], () => {
        if (!isOpen.value || !target.value) {
          return;
        }

        const currentTarget = target.value;
        if (mode.value === 'Create') {
          if (currentTarget.kind !== 'concurrency') {
            closePanel();
          }
          return;
        }

        const summary = logSummaryAnalyzeMode.value;
        if (!summary) {
          closePanel();
          return;
        }

        if (
          currentTarget.kind === 'client' &&
          !summary.stats.clients.some(c => c.cid === currentTarget.cid)
        ) {
          closePanel();
        }
      });
    });
  }

  // ---------------------------------------------------------------------------
  // Lifecycle Hooks & Observers

  // (none)

  // ---------------------------------------------------------------------------
  // Public API (Return)

  return {
    isOpen,
    target,
    activeTab,
    eventTypeFilter,
    openSystem,
    openClient,
    openConcurrency,
    openJitter,
    closePanel,
    setActiveTab,
    toggleEventTypeFilter,
    setEventTypeFilter,
    resetEventTypeFilter,
    isTargetActive
  };
}
