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
 * Format microsecond duration or timestamp as rounded integer milliseconds (e.g. "50 ms").
 * Normalizes negative zero to "0 ms".
 */
export function formatUsAsMsInt(us: number): string {
  const ms = Math.round(us / 1000);
  const normalized = ms === 0 ? 0 : ms;
  return `${normalized} ms`;
}

/**
 * Format microsecond duration or timestamp with fixed 3 decimal places in milliseconds (e.g. "10.913 ms").
 * Normalizes zero and sub-half-microsecond residuals to "0.000 ms".
 */
export function formatUsAsMs(us: number): string {
  if (Math.abs(us) < 0.5) {
    return '0.000 ms';
  }
  const ms = us / 1000;
  return `${ms.toFixed(3)} ms`;
}

/**
 * Format microsecond delta value with explicit sign and 3 decimal places in milliseconds.
 * Normalizes zero and sub-half-microsecond residuals to "+0.000 ms".
 */
export function formatDeltaUsAsMs(deltaUs: number): string {
  if (Math.abs(deltaUs) < 0.5) {
    return '+0.000 ms';
  }
  const ms = deltaUs / 1000;
  const sign = deltaUs > 0 ? '+' : '';
  return `${sign}${ms.toFixed(3)} ms`;
}

/**
 * Format delta count with explicit positive sign for concurrency delta (including "+0").
 */
export function formatDeltaCount(delta: number): string {
  if (delta === 0) {
    return '+0';
  }
  const sign = delta > 0 ? '+' : '';
  return `${sign}${delta}`;
}

/**
 * Format a numeric client identifier as zero-padded CID label (e.g. "CID: 001" or "CID: 001 (sensor)").
 */
export function formatCid(cid: number, displayName?: string): string {
  const padded = String(cid).padStart(3, '0');
  return displayName ? `CID: ${padded} (${displayName})` : `CID: ${padded}`;
}

/**
 * Format a [0, 1] ratio as percentage with a space before "%" (e.g. "100.00 %").
 * Normalizes negative zero residuals to "0.00 %".
 */
export function formatRatioAsPercent(ratio: number, decimals: number = 2): string {
  const pct = ratio * 100;
  const normalized = Math.abs(pct) < 1e-12 ? 0 : pct;
  return `${normalized.toFixed(decimals)} %`;
}
