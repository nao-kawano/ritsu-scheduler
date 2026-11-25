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

/**
 * Selected target scope displayed inside the right-side statistics inspector panel.
 */
export type StatsTarget =
  | { kind: 'system' }
  | { kind: 'client'; cid: number }
  | { kind: 'concurrency' }
  | { kind: 'jitter' };

/**
 * Active tab identifier inside the statistics inspector panel.
 */
export type StatsPanelTab = 'stats' | 'events' | 'outliers';

/**
 * Two-tier anomaly severity classification used by status indicators and client badges.
 */
export type AnomalySeverity = 'error' | 'warning' | 'none';

/**
 * Common anomaly count fields shared by SystemStats and ClientStats.
 */
export interface AnomalyCountFields {
  overrun_count: number;
  skip_count: number;
  late_count: number;
  retransmit_count: number;
  error_count: number;
}

/**
 * Semantic color tone supported by StatsProgressBar segments.
 */
export type ProgressTone = 'none' | 'normal' | 'warning' | 'error';

/**
 * Single segment entry rendered inside StatsProgressBar.
 */
export interface ProgressSegment {
  ratio: number;
  tone: ProgressTone;
}
