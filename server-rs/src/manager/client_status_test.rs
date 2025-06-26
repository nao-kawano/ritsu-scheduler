#[cfg(test)]
use super::*;

#[test]
fn test_new() {
    // cycle trigger.
    let config = ClientConfig::new(1, TriggerType::Cycle(2), 1).unwrap();
    let status = ClientStatus::new(config);
    assert_eq!(status.config.client_id, 1);
    assert_eq!(status.state, ClientState::None);
    assert_eq!(status.depends_on.len(), 0);

    // depends trigger.
    let config = ClientConfig::new(
        2,
        TriggerType::Depends {
            clients: vec![1, 3],
        },
        0,
    )
    .unwrap();
    let status = ClientStatus::new(config);
    assert_eq!(status.config.client_id, 2);
    assert_eq!(status.state, ClientState::None);
    assert_eq!(status.depends_on.len(), 2);
    assert_eq!(status.depends_on.get(&1), Some(&false));
    assert_eq!(status.depends_on.get(&3), Some(&false));
}

#[test]
fn test_set_client_state() {
    let config = ClientConfig::new(1, TriggerType::Cycle(2), 1).unwrap();
    let mut status = ClientStatus::new(config);
    assert_eq!(status.state, ClientState::None);

    status.set_client_state(ClientState::Idle);
    assert_eq!(status.state, ClientState::Idle);

    status.set_client_state(ClientState::Running { cycle: 1 });
    assert_eq!(status.state, ClientState::Running { cycle: 1 });
}

#[test]
fn test_is_depends_ok() {
    let config = ClientConfig::new(
        2,
        TriggerType::Depends {
            clients: vec![1, 3],
        },
        0,
    )
    .unwrap();
    let mut status = ClientStatus::new(config);
    assert_eq!(status.is_depends_ok(), false);

    status.depends_on.insert(1, true);
    assert_eq!(status.is_depends_ok(), false);

    status.depends_on.insert(3, true);
    assert_eq!(status.is_depends_ok(), true);
}

#[test]
fn test_update_depend() {
    let config = ClientConfig::new(
        2,
        TriggerType::Depends {
            clients: vec![1, 3],
        },
        0,
    )
    .unwrap();
    let mut status = ClientStatus::new(config);
    assert_eq!(status.is_depends_ok(), false);

    status.update_depend(1);
    assert_eq!(status.is_depends_ok(), false);

    status.update_depend(2);
    assert_eq!(status.is_depends_ok(), false);

    status.update_depend(3);
    assert_eq!(status.is_depends_ok(), true);
}

#[test]
fn test_clear_depends() {
    let config = ClientConfig::new(
        2,
        TriggerType::Depends {
            clients: vec![1, 3],
        },
        0,
    )
    .unwrap();
    let mut status = ClientStatus::new(config);
    status.depends_on.insert(1, true);
    status.depends_on.insert(3, true);
    assert_eq!(status.is_depends_ok(), true);

    status.clear_depends();
    assert_eq!(status.is_depends_ok(), false);
}
