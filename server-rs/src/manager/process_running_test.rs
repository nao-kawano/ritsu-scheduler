#[cfg(test)]
use super::*;
use crate::ManagerState;
use crate::config::*;

use std::collections::HashMap;

fn create_context_simple() -> ManagerContext {
    #[rustfmt::skip]
    let mut ctx = ManagerContext::new(vec![
        ClientConfig::new(0, TriggerType::Cycle(1), 0).unwrap(),
        ClientConfig::new(1, TriggerType::Cycle(3), 0).unwrap(),
        ClientConfig::new(2, TriggerType::Cycle(3), 1).unwrap(),
        ClientConfig::new(3, TriggerType::Cycle(3), 2).unwrap(),
        ClientConfig::new(10, TriggerType::Depends { clients: vec![2] }, 0).unwrap(),
    ]);
    ctx.state = ManagerState::Running;
    ctx.num_active_clients = ctx.clients.len();
    return ctx;
}

/*
fn create_context_complex() -> ManagerContext {
    #[rustfmt::skip]
    let mut ctx = ManagerContext::new(vec![
        ClientConfig::new(0, TriggerType::Cycle(1), 0).unwrap(),
        ClientConfig::new(1, TriggerType::Cycle(2), 1).unwrap(),
        ClientConfig::new(2, TriggerType::Cycle(2), 0).unwrap(),
        ClientConfig::new(3, TriggerType::Cycle(2), 0).unwrap(),
        ClientConfig::new(10, TriggerType::Depends { clients: vec![2] }, 0).unwrap(),
        ClientConfig::new(11, TriggerType::Depends { clients: vec![2, 3] }, 0).unwrap(),
        ClientConfig::new(20, TriggerType::Depends { clients: vec![10, 11] }, 0).unwrap(),
    ]);
    ctx.state = ManagerState::Running;
    ctx.num_active_clients = ctx.clients.len();
    return ctx;
}
*/

#[test]
fn test_enter_state() {
    // create objects.
    let mut ctx = create_context_simple();
    let proc = ManagerProcRunning;

    // setup condition.
    // do nothing.

    // send event.
    proc.enter_state(&mut ctx);

    // check result.
    assert_eq!(ctx.state, ManagerState::Running);
    assert_eq!(ctx.cycle_current, ManagerContext::CYCLE_MAX);
}

#[test]
fn test_on_cycle_start_simple() {
    // create objects.
    let mut ctx = create_context_simple();
    let proc = ManagerProcRunning;

    // setup condition.
    ctx.cycle_current = ManagerContext::CYCLE_MAX;
    ctx.clients.get_mut(&0).unwrap().state = ClientState::Ready; // ok to run.
    ctx.clients.get_mut(&1).unwrap().state = ClientState::Ready; // ok to run.
    ctx.clients.get_mut(&2).unwrap().state = ClientState::Ready; // ok to run.
    ctx.clients.get_mut(&3).unwrap().state = ClientState::Ready; // ok to run.
    ctx.clients.get_mut(&10).unwrap().state = ClientState::Ready; // ok to run.

    // send event: cycle=0
    let result = proc.on_cycle_start(&mut ctx, 123).unwrap();
    let result_map: HashMap<u16, &Message> = result.iter().map(|m| (m.client_id, m)).collect();
    // check result.
    assert_eq!(result.len(), 2);
    assert_eq!(result_map.get(&0).unwrap().message_type, MessageType::Ok);
    assert_eq!(result_map.get(&1).unwrap().message_type, MessageType::Ok);
    assert_eq!(ctx.state, ManagerState::Running);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(
        ctx.clients.get(&0).unwrap().state,
        ClientState::Running { cycle: 0 }
    );
    assert_eq!(
        ctx.clients.get(&1).unwrap().state,
        ClientState::Running { cycle: 0 }
    );
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Ready);
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Ready);

    // send event: cycle=1
    let result = proc.on_cycle_start(&mut ctx, 123).unwrap();
    let result_map: HashMap<u16, &Message> = result.iter().map(|m| (m.client_id, m)).collect();
    // check result.
    assert_eq!(result.len(), 1);
    assert_eq!(result_map.get(&2).unwrap().message_type, MessageType::Ok);
    assert_eq!(ctx.state, ManagerState::Running);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(
        ctx.clients.get(&0).unwrap().state,
        ClientState::Running { cycle: 0 }
    );
    assert_eq!(
        ctx.clients.get(&1).unwrap().state,
        ClientState::Running { cycle: 0 }
    );
    assert_eq!(
        ctx.clients.get(&2).unwrap().state,
        ClientState::Running { cycle: 1 }
    );
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Ready);

    // send event: cycle=2
    let result = proc.on_cycle_start(&mut ctx, 123).unwrap();
    let result_map: HashMap<u16, &Message> = result.iter().map(|m| (m.client_id, m)).collect();
    // check result.
    assert_eq!(result.len(), 1);
    assert_eq!(result_map.get(&3).unwrap().message_type, MessageType::Ok);
    assert_eq!(ctx.state, ManagerState::Running);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(
        ctx.clients.get(&0).unwrap().state,
        ClientState::Running { cycle: 0 }
    );
    assert_eq!(
        ctx.clients.get(&1).unwrap().state,
        ClientState::Running { cycle: 0 }
    );
    assert_eq!(
        ctx.clients.get(&2).unwrap().state,
        ClientState::Running { cycle: 1 }
    );
    assert_eq!(
        ctx.clients.get(&3).unwrap().state,
        ClientState::Running { cycle: 2 }
    );

    // send event: cycle=3, all client still running.
    let result = proc.on_cycle_start(&mut ctx, 123).unwrap();
    // check result.
    assert_eq!(result.len(), 0);
    assert_eq!(ctx.state, ManagerState::Running);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(
        ctx.clients.get(&0).unwrap().state,
        ClientState::Running { cycle: 0 }
    );
    assert_eq!(
        ctx.clients.get(&1).unwrap().state,
        ClientState::Running { cycle: 0 }
    );
    assert_eq!(
        ctx.clients.get(&2).unwrap().state,
        ClientState::Running { cycle: 1 }
    );
    assert_eq!(
        ctx.clients.get(&3).unwrap().state,
        ClientState::Running { cycle: 2 }
    );
}

#[test]
fn test_on_client_join() {
    // create objects.
    let mut ctx = create_context_simple();
    let proc = ManagerProcRunning;

    // setup condition.
    // do nothing.

    // send event.
    let m = Message::new(MessageType::Join, 0, None).unwrap();
    let result = proc.on_client_join(&mut ctx, &m);

    // check result.
    assert_eq!(result.is_err(), true);
}

#[test]
fn test_on_client_ready_simple() {
    // create objects.
    let mut ctx = create_context_simple();
    let proc = ManagerProcRunning;

    // setup condition.
    ctx.cycle_current = 0;
    ctx.clients.get_mut(&0).unwrap().state = ClientState::Ready; // ok to run.
    ctx.clients.get_mut(&1).unwrap().state = ClientState::Idle; // waiting ready.
    ctx.clients.get_mut(&2).unwrap().state = ClientState::Idle; // ok to run.
    ctx.clients.get_mut(&3).unwrap().state = ClientState::Idle; // ok to run.
    ctx.clients.get_mut(&10).unwrap().state = ClientState::Idle; // ok to run.

    // send event.
    let m = Message::new(MessageType::Ready, 0, None).unwrap();
    let result = proc.on_client_ready(&mut ctx, &m).unwrap();

    // check result.
    assert_eq!(result.len(), 0);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Ready);

    // send event.
    let m = Message::new(MessageType::Ready, 1, None).unwrap();
    let result = proc.on_client_ready(&mut ctx, &m).unwrap();
    let result_map: HashMap<u16, &Message> = result.iter().map(|m| (m.client_id, m)).collect();

    // check result.
    assert_eq!(result.len(), 1);
    assert_eq!(result_map.get(&1).unwrap().message_type, MessageType::Ok);
    assert_eq!(ctx.state, ManagerState::Running);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Ready);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Ready);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Idle);
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Idle);
    assert_eq!(ctx.clients.get(&10).unwrap().state, ClientState::Idle);
}

#[test]
fn test_on_client_done_simple() {
    // create objects.
    let mut ctx = create_context_simple();
    let proc = ManagerProcRunning;

    // setup condition.
    ctx.cycle_current = 0;
    ctx.clients.get_mut(&0).unwrap().state = ClientState::Ready; // ok to run.
    ctx.clients.get_mut(&1).unwrap().state = ClientState::Ready; // ok to run.
    ctx.clients.get_mut(&2).unwrap().state = ClientState::Running { cycle: 0 }; // running.
    ctx.clients.get_mut(&3).unwrap().state = ClientState::Ready; // ok to run.
    ctx.clients.get_mut(&10).unwrap().state = ClientState::Ready; // ok to run.

    // send event.
    let m = Message::new(MessageType::Done, 2, None).unwrap();
    let result = proc.on_client_done(&mut ctx, &m).unwrap();
    let result_map: HashMap<u16, &Message> = result.iter().map(|m| (m.client_id, m)).collect();
    // check result.
    assert_eq!(result.len(), 2);
    assert_eq!(result_map.get(&2).unwrap().message_type, MessageType::Ok);
    assert_eq!(result_map.get(&10).unwrap().message_type, MessageType::Ok);
    assert_eq!(ctx.state, ManagerState::Running);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Ready);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Ready);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Idle);
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Ready);
    assert_eq!(
        ctx.clients.get(&10).unwrap().state,
        ClientState::Running { cycle: 0 }
    );
}

#[test]
fn test_on_client_exit() {
    // create objects.
    let mut ctx = create_context_simple();
    let proc = ManagerProcRunning;

    // setup condition.
    ctx.clients.get_mut(&0).unwrap().state = ClientState::Idle;
    ctx.clients.get_mut(&1).unwrap().state = ClientState::Running { cycle: 0 };
    ctx.clients.get_mut(&2).unwrap().state = ClientState::Ready;
    ctx.clients.get_mut(&3).unwrap().state = ClientState::Ready;
    ctx.clients.get_mut(&10).unwrap().state = ClientState::Ready;

    // send event.
    let m = Message::new(MessageType::Exit, 0, None).unwrap();
    let result = proc.on_client_exit(&mut ctx, &m).unwrap();
    let result_map: HashMap<u16, &Message> = result.iter().map(|m| (m.client_id, m)).collect();

    // check result.
    assert_eq!(result.len(), 4);
    assert_eq!(result_map.get(&0).unwrap().message_type, MessageType::Ok);
    assert_eq!(result_map.get(&2).unwrap().message_type, MessageType::Error);
    assert_eq!(result_map.get(&3).unwrap().message_type, MessageType::Error);
    assert_eq!(
        result_map.get(&10).unwrap().message_type,
        MessageType::Error
    );
    assert_eq!(ctx.state, ManagerState::Exitting);
    assert_eq!(ctx.state_changed, true);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::None);
    assert_eq!(
        ctx.clients.get(&1).unwrap().state,
        ClientState::Running { cycle: 0 }
    );
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Exitting);
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Exitting);
    assert_eq!(ctx.clients.get(&10).unwrap().state, ClientState::Exitting);
    assert_eq!(ctx.num_active_clients, 4);
}

#[test]
fn test_on_shutdown() {
    // create objects.
    let mut ctx = create_context_simple();
    let proc = ManagerProcRunning;

    // setup condition.
    ctx.clients.get_mut(&0).unwrap().state = ClientState::Idle;
    ctx.clients.get_mut(&1).unwrap().state = ClientState::Idle;
    ctx.clients.get_mut(&2).unwrap().state = ClientState::Running { cycle: 0 };
    ctx.clients.get_mut(&3).unwrap().state = ClientState::Ready;
    ctx.clients.get_mut(&10).unwrap().state = ClientState::Ready;

    // send event.
    let result = proc.on_shutdown(&mut ctx).unwrap();
    let result_map: HashMap<u16, &Message> = result.iter().map(|m| (m.client_id, m)).collect();

    // check result.
    assert_eq!(result.len(), 2);
    assert_eq!(result_map.get(&3).unwrap().message_type, MessageType::Error);
    assert_eq!(
        result_map.get(&10).unwrap().message_type,
        MessageType::Error
    );
    assert_eq!(ctx.state, ManagerState::Exitting);
    assert_eq!(ctx.state_changed, true);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Idle);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Idle);
    assert_eq!(
        ctx.clients.get(&2).unwrap().state,
        ClientState::Running { cycle: 0 }
    );
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Exitting);
    assert_eq!(ctx.clients.get(&10).unwrap().state, ClientState::Exitting);
    assert_eq!(ctx.num_active_clients, 5);
}
