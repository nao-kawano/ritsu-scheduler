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

/**
 * Permitted log levels in ascending order of verbosity.
 */
export const LOG_LEVELS = ['off', 'error', 'warn', 'info', 'debug', 'trace'] as const;
export type LogLevel = (typeof LOG_LEVELS)[number];

/**
 * Default log level for console output.
 */
export const DEFAULT_LOG_LEVEL: LogLevel = 'info';

/**
 * Default log level for file output.
 */
export const DEFAULT_LOG_FILE_LEVEL: LogLevel = 'debug';

/**
 * Returns the numeric verbosity priority for a log level (higher number = more verbose).
 * Returns -1 if the level is unknown.
 */
export const getLogLevelPriority = (level: string): number => {
  return LOG_LEVELS.indexOf(level.toLowerCase() as LogLevel);
};

export interface ServerConfig {
  port: number;
  cycle_time_ms: number;
  stats_interval_cycle: number;
  log_level: LogLevel;
  log_file_dir: string;
  log_file_level: LogLevel;
}

export interface ClientConfig {
  client_id: number;
  start_cycle: number;
  interval: number;
  offset: number;
  depends: number[];
  display_name: string;
  expected_duration_ms: number;
}

/**
 * Default baseline configuration for server settings.
 */
export const DEFAULT_SERVER_CONFIG: Readonly<ServerConfig> = Object.freeze({
  port: 7878,
  cycle_time_ms: 50,
  stats_interval_cycle: 0,
  log_level: DEFAULT_LOG_LEVEL,
  log_file_dir: '',
  log_file_level: DEFAULT_LOG_FILE_LEVEL,
});

/**
 * Default baseline template for client configuration.
 */
export const DEFAULT_CLIENT_CONFIG: Readonly<ClientConfig> = Object.freeze({
  client_id: 0,
  start_cycle: 0,
  interval: 1,
  offset: 0,
  depends: [],
  display_name: '',
  expected_duration_ms: 10,
});

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
