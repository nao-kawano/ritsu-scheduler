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

import {
  ANOMALY_EVENT_LABELS,
  ANOMALY_EVENT_TYPES,
  type AnomalyEventType
} from '../types/analyze';
import type { AnomalyCountFields, AnomalySeverity } from '../types/stats';

/**
 * Frozen zero-initialized anomaly event count map used as a non-null fallback.
 */
export const EMPTY_ANOMALY_EVENT_COUNTS: Readonly<Record<AnomalyEventType, number>> = Object.freeze({
  overrun: 0,
  skip: 0,
  late: 0,
  retransmit: 0,
  error: 0
});

/**
 * Maps flat anomaly count fields onto a keyed Record<AnomalyEventType, number>.
 */
export function toAnomalyEventCounts(counts: AnomalyCountFields): Record<AnomalyEventType, number> {
  return {
    overrun: counts.overrun_count,
    skip: counts.skip_count,
    late: counts.late_count,
    retransmit: counts.retransmit_count,
    error: counts.error_count
  };
}

/**
 * Returns the sum of all five anomaly event counts.
 */
export function getTotalAnomalyCount(counts: AnomalyCountFields): number {
  return (
    counts.overrun_count +
    counts.skip_count +
    counts.late_count +
    counts.retransmit_count +
    counts.error_count
  );
}

/**
 * Classifies anomaly counts into two-tier severity ('error' > 'warning' > 'none').
 */
export function getAnomalySeverity(counts: AnomalyCountFields): AnomalySeverity {
  if (counts.overrun_count > 0 || counts.error_count > 0) {
    return 'error';
  } else if (counts.skip_count > 0 || counts.late_count > 0 || counts.retransmit_count > 0) {
    return 'warning';
  } else {
    return 'none';
  }
}

/**
 * Formats non-zero anomaly counts into a concise comma-separated summary string.
 */
export function formatAnomalySummary(counts: AnomalyCountFields): string {
  const byType = toAnomalyEventCounts(counts);
  const parts: string[] = [];
  for (const type of ANOMALY_EVENT_TYPES) {
    if (byType[type] > 0) {
      parts.push(`${ANOMALY_EVENT_LABELS[type]}: ${byType[type]}`);
    }
  }
  return parts.join(', ');
}
