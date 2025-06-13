#[cfg(test)]
use super::*;

#[test]
fn test_manager_context_new() {
    // Given
    let configs = vec![
        ClientConfig::new(0, TriggerType::Cycle(2), 0).unwrap(),
        ClientConfig::new(1, TriggerType::Depends { clients: vec![0] }, 0).unwrap(),
    ];

    // When
    let context = ManagerContext::new(configs);

    // Then
    assert_eq!(context.state, ManagerState::Starting);
    assert_eq!(context.state_changed, false);
    assert_eq!(context.clients.len(), 2);
    assert_eq!(context.num_active_clients, 0);
    assert_eq!(context.cycle_current, 0);
    assert_eq!(context.cycle_max, 20); // 2 * 10
}

#[test]
fn test_manager_context_set_state() {
    // Given
    let configs = vec![ClientConfig::new(0, TriggerType::Cycle(1), 0).unwrap()];
    let mut context = ManagerContext::new(configs);

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
    ManagerContext::new(configs);
}

#[test]
#[should_panic(expected = "client config has no trigger=Cycle")]
fn test_manager_context_new_no_cycle_trigger() {
    // Given
    let configs = vec![ClientConfig::new(1, TriggerType::Depends { clients: vec![0] }, 0).unwrap()];

    // When
    ManagerContext::new(configs);
}

#[test]
fn test_create_client_status() {
    // Given
    let configs = vec![
        ClientConfig::new(0, TriggerType::Cycle(1), 0).unwrap(),
        ClientConfig::new(1, TriggerType::Depends { clients: vec![0] }, 0).unwrap(),
    ];

    // When
    let (clients, cycle_max) = ManagerContext::create_client_status(configs);

    // Then
    assert_eq!(clients.len(), 2);
    assert_eq!(cycle_max, 10);
    assert!(clients.contains_key(&0));
    assert!(clients.contains_key(&1));
}
