#[cfg(test)]
use super::*;

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
    let configs = vec![
        ClientConfig::new(0, 2, 0, vec![]).unwrap(),
        ClientConfig::new(1, 2, 0, vec![0]).unwrap(),
    ];

    // When
    let context = ManagerContext::new(configs, 0);

    // Then
    assert_eq!(context.state, ManagerState::Starting);
    assert_eq!(context.state_changed, false);
    assert_eq!(context.clients.len(), 2);
    assert_eq!(context.num_active_clients, 0);
    assert_eq!(context.cycle_current, 0);
    assert_eq!(context.graph_start.len(), 1);
    assert_eq!(context.graph_start.contains(&0), true);
}

#[test]
fn test_manager_context_set_state() {
    // Given
    let configs = vec![ClientConfig::new(0, 1, 0, vec![]).unwrap()];
    let mut context = ManagerContext::new(configs, 0);

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
#[should_panic(expected = "client config is empty")]
fn test_manager_context_new_empty_configs() {
    // Given
    let configs: Vec<ClientConfig> = vec![];

    // When
    ManagerContext::new(configs, 0);
}

#[test]
#[should_panic]
fn test_manager_context_new_no_cycle_trigger() {
    // Given
    let configs = vec![ClientConfig::new(1, 2, 0, vec![0]).unwrap()];

    // When
    ManagerContext::new(configs, 0);
}
