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
