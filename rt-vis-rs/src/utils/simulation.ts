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

import type { PlannedExecution } from '../types/simulation';

/**
 * Creates a Map indexing PlannedExecution objects by their unique instance_id.
 * Allows O(1) lookup of parent/dependent execution plans.
 */
export function mapPlansByInstanceId(plans: PlannedExecution[]): Map<number, PlannedExecution> {
  const map = new Map<number, PlannedExecution>();
  if (!plans) return map;

  plans.forEach(plan => {
    map.set(plan.instance_id, plan);
  });

  return map;
}
