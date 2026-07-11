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
#[cfg(test)]
use super::*;

use rt_config::{ClientConfig, ClientRule, SchedulerConfig, ServerConfig};
use std::collections::HashMap;

#[test]
fn test_client_stats_default() {
    let stats = ClientStats::default();
    assert_eq!(stats.trigger_count, 0);
    assert_eq!(stats.success_count, 0);
    assert_eq!(stats.skip_count, 0);
    assert_eq!(stats.late_count, 0);
    assert_eq!(stats.overrun_count, 0);
    assert_eq!(stats.time_min, u32::MAX);
    assert_eq!(stats.time_max, 0);
    assert_eq!(stats.time_sum, 0);
    assert_eq!(stats.overrun_time_min, u32::MAX);
    assert_eq!(stats.overrun_time_max, 0);
    assert_eq!(stats.overrun_time_sum, 0);
}

#[test]
fn test_manager_context_new() {
    // Given
    let server_config = ServerConfig {
        port: 8080,
        cycle_time_ms: 50,
        stats_interval_cycle: 0,
    };
    let client_configs = vec![
        ClientConfig::new(0, 2, 0, vec![], 0).unwrap(),
        ClientConfig::new(1, 2, 0, vec![0], 0).unwrap(),
    ];
    let scheduler_config = SchedulerConfig {
        server_config,
        client_configs,
    };
    let rules = scheduler_config.get_client_rules();

    // When
    let context = ManagerContext::new(scheduler_config.client_configs, rules, 0);

    // Then
    assert_eq!(context.state, ManagerState::Starting);
    assert_eq!(context.state_changed, false);
    assert_eq!(context.clients.len(), 2);
    assert_eq!(context.num_active_clients, 0);
    assert_eq!(context.running_cycle, -1);
    assert_eq!(context.graph_start.len(), 1);
    assert_eq!(context.graph_start.contains(&0), true);
}

#[test]
fn test_manager_context_set_state() {
    // Given
    let server_config = ServerConfig {
        port: 8080,
        cycle_time_ms: 50,
        stats_interval_cycle: 0,
    };
    let client_configs = vec![ClientConfig::new(0, 1, 0, vec![], 0).unwrap()];
    let scheduler_config = SchedulerConfig {
        server_config,
        client_configs,
    };
    let rules = scheduler_config.get_client_rules();
    let mut context = ManagerContext::new(scheduler_config.client_configs, rules, 0);

    // When
    let result = context.set_state(ManagerState::Running);

    // Then
    assert_eq!(result, true);
    assert_eq!(context.state, ManagerState::Running);
    assert_eq!(context.state_changed, true);

    // When (setting the same state again)
    context.state_changed = false;
    context.set_state(ManagerState::Running);

    // Then
    assert_eq!(context.state_changed, false);
}

#[test]
#[should_panic(expected = "Client config is empty")]
fn test_manager_context_new_empty_configs() {
    // Given
    let configs: Vec<ClientConfig> = vec![];
    let rules: HashMap<u16, ClientRule> = HashMap::new();

    // When
    ManagerContext::new(configs, rules, 0);
}

#[test]
#[should_panic]
fn test_manager_context_new_no_rule() {
    // Given
    let configs = vec![ClientConfig::new(1, 2, 0, vec![0], 0).unwrap()];
    let rules: HashMap<u16, ClientRule> = HashMap::new(); // Missing rule for CID 1

    // When
    ManagerContext::new(configs, rules, 0);
}

#[test]
fn test_server_stats_calc_elapsed_times_normal() {
    // Given
    let configs = vec![ClientConfig::new(0, 1, 0, vec![], 0).unwrap()];
    let scheduler_config = SchedulerConfig {
        server_config: ServerConfig {
            port: 8080,
            cycle_time_ms: 50,
            stats_interval_cycle: 0,
        },
        client_configs: configs,
    };
    let rules = scheduler_config.get_client_rules();
    let mut context = ManagerContext::new(scheduler_config.client_configs, rules, 0);

    let t0 = std::time::Instant::now();
    context.stats.created_at = t0;
    context.stats.running_start_at = Some(t0 + std::time::Duration::from_millis(100));
    context.stats.exiting_start_at = Some(t0 + std::time::Duration::from_millis(500));
    let now = t0 + std::time::Duration::from_millis(1000);

    // When
    let (total, starting, running, exiting) = context.calc_elapsed_times(now);

    // Then
    assert_eq!(total, 1000);
    assert_eq!(starting, 100);
    assert_eq!(running, 400);
    assert_eq!(exiting, 500);
}

#[test]
fn test_server_stats_calc_elapsed_times_bypass_running() {
    // Given
    let configs = vec![ClientConfig::new(0, 1, 0, vec![], 0).unwrap()];
    let scheduler_config = SchedulerConfig {
        server_config: ServerConfig {
            port: 8080,
            cycle_time_ms: 50,
            stats_interval_cycle: 0,
        },
        client_configs: configs,
    };
    let rules = scheduler_config.get_client_rules();
    let mut context = ManagerContext::new(scheduler_config.client_configs, rules, 0);

    let t0 = std::time::Instant::now();
    context.stats.created_at = t0;
    context.stats.running_start_at = None;
    context.stats.exiting_start_at = Some(t0 + std::time::Duration::from_millis(500));
    let now = t0 + std::time::Duration::from_millis(1000);

    // When
    let (total, starting, running, exiting) = context.calc_elapsed_times(now);

    // Then
    assert_eq!(total, 1000);
    assert_eq!(starting, 500); // run_start falls back to exit_start
    assert_eq!(running, 0);
    assert_eq!(exiting, 500);
}

#[test]
fn test_server_stats_calc_elapsed_times_direct_exit() {
    // Given
    let configs = vec![ClientConfig::new(0, 1, 0, vec![], 0).unwrap()];
    let scheduler_config = SchedulerConfig {
        server_config: ServerConfig {
            port: 8080,
            cycle_time_ms: 50,
            stats_interval_cycle: 0,
        },
        client_configs: configs,
    };
    let rules = scheduler_config.get_client_rules();
    let mut context = ManagerContext::new(scheduler_config.client_configs, rules, 0);

    let t0 = std::time::Instant::now();
    context.stats.created_at = t0;
    context.stats.running_start_at = None;
    context.stats.exiting_start_at = None;
    let now = t0 + std::time::Duration::from_millis(1000);

    // When
    let (total, starting, running, exiting) = context.calc_elapsed_times(now);

    // Then
    assert_eq!(total, 1000);
    assert_eq!(starting, 1000); // both fall back to now
    assert_eq!(running, 0);
    assert_eq!(exiting, 0);
}
