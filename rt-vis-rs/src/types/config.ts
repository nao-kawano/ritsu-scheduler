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
 * Ritsu Configuration Types
 * Based on rt-config-rs structure.
 */

export interface ServerConfig {
  port: number;
  cycle_time_ms: number;
  stats_interval_cycle?: number;
}

export interface ClientConfig {
  client_id: number;
  display_name: string;
  cycle: number;
  cycle_offset: number;
  depends: number[];
  expected_duration_ms?: number;
}

export interface SchedulerConfig {
  server_config: ServerConfig;
  client_configs: ClientConfig[];
}

// -- Wrapped configuration for Vue instance management.

export interface ClientConfigUI {
  configId: number;
  data: ClientConfig;
}

export interface SchedulerConfigUI {
  sessionId: number;
  server_config: ServerConfig;
  client_configs: ClientConfigUI[];
}
