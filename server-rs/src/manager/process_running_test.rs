#[cfg(test)]
use super::*;
use crate::ManagerState;
use crate::config::*;
use crate::manager::context::ClientState;

use std::collections::HashMap;

fn create_context_simple() -> ManagerContext {
    let mut ctx = ManagerContext::new(vec![
        ClientConfig::new(0, 3, 0, vec![]).unwrap(),
        ClientConfig::new(1, 3, 0, vec![]).unwrap(),
        ClientConfig::new(2, 3, 1, vec![]).unwrap(),
        ClientConfig::new(3, 3, 2, vec![]).unwrap(),
        ClientConfig::new(10, 3, 1, vec![2]).unwrap(),
    ]);
    ctx.state = ManagerState::Running;
    ctx.num_active_clients = ctx.clients.len();
    ctx.clients.get_mut(&0).unwrap().state = ClientState::Active;
    ctx.clients.get_mut(&1).unwrap().state = ClientState::Active;
    ctx.clients.get_mut(&2).unwrap().state = ClientState::Active;
    ctx.clients.get_mut(&3).unwrap().state = ClientState::Active;
    ctx.clients.get_mut(&10).unwrap().state = ClientState::Active;
    ctx.clients.get_mut(&0).unwrap().last_mid = 9;
    ctx.clients.get_mut(&1).unwrap().last_mid = 9;
    ctx.clients.get_mut(&2).unwrap().last_mid = 9;
    ctx.clients.get_mut(&3).unwrap().last_mid = 9;
    ctx.clients.get_mut(&10).unwrap().last_mid = 9;
    return ctx;
}

#[test]
fn test_enter_state() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
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
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context_simple();
    let proc = ManagerProcRunning;

    // setup condition.
    ctx.cycle_current = ManagerContext::CYCLE_MAX;
    let _ = ctx.sched.on_ready(0);
    let _ = ctx.sched.on_ready(1);
    let _ = ctx.sched.on_ready(2);
    let _ = ctx.sched.on_ready(3);
    let _ = ctx.sched.on_ready(10);

    // send event: cycle=0
    let result = proc.on_cycle_start(&mut ctx, 123).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();
    // check result.
    assert_eq!(result.len(), 2);
    assert_eq!(rmap.get(&0).unwrap().mtype, MessageType::Ok);
    assert_eq!(rmap.get(&1).unwrap().mtype, MessageType::Ok);
    assert_eq!(ctx.state, ManagerState::Running);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&10).unwrap().state, ClientState::Active);

    // send event: cycle=1
    let result = proc.on_cycle_start(&mut ctx, 123).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();
    // check result.
    assert_eq!(result.len(), 1);
    assert_eq!(rmap.get(&2).unwrap().mtype, MessageType::Ok);
    assert_eq!(ctx.state, ManagerState::Running);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&10).unwrap().state, ClientState::Active);

    // send event: cycle=2
    let result = proc.on_cycle_start(&mut ctx, 123).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();
    // check result.
    assert_eq!(result.len(), 1);
    assert_eq!(rmap.get(&3).unwrap().mtype, MessageType::Ok);
    assert_eq!(ctx.state, ManagerState::Running);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&10).unwrap().state, ClientState::Active);

    // send event: cycle=3, client 10 skipped, all other clients still running.
    let result = proc.on_cycle_start(&mut ctx, 123).unwrap();
    // check result.
    assert_eq!(result.len(), 0);
    assert_eq!(ctx.state, ManagerState::Running);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&10).unwrap().state, ClientState::Active);

    // send event: cycle=4, client 10 skipped, all other clients still running.
    let result = proc.on_cycle_start(&mut ctx, 123).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();
    // check result.
    assert_eq!(result.len(), 1);
    assert_eq!(rmap.get(&10).unwrap().mtype, MessageType::Skip);
    assert_eq!(ctx.state, ManagerState::Running);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&10).unwrap().state, ClientState::Active);
}

#[test]
fn test_on_client_join() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context_simple();
    let proc = ManagerProcRunning;

    // setup condition.
    // do nothing.

    // send event.
    let m = Message::new(MessageType::Join, 0, 0, None).unwrap();
    let result = proc.on_client_join(&mut ctx, &m);

    // check result.
    assert_eq!(result.is_err(), true);
}

#[test]
fn test_on_client_ready_simple() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context_simple();
    let proc = ManagerProcRunning;

    // setup condition.
    ctx.cycle_current = 0;

    // send event.
    let m = Message::new(MessageType::Ready, 0, 0, None).unwrap();
    let result = proc.on_client_ready(&mut ctx, &m).unwrap();

    // check result.
    assert_eq!(result.len(), 0);
    assert_eq!(ctx.state, ManagerState::Running);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&10).unwrap().state, ClientState::Active);
}

#[test]
fn test_on_client_done_simple() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context_simple();
    let proc = ManagerProcRunning;

    // setup condition.
    ctx.cycle_current = 1;
    let _ = ctx.sched.on_ready(0);
    let _ = ctx.sched.on_ready(1);
    let _ = ctx.sched.on_ready(2);
    let _ = ctx.sched.on_ready(3);
    let _ = ctx.sched.on_ready(10);
    let _ = ctx.sched.on_start(2);

    // send event.
    let m = Message::new(MessageType::Done, 0, 2, None).unwrap();
    let result = proc.on_client_done(&mut ctx, &m).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();

    // check result.
    assert_eq!(result.len(), 2);
    assert_eq!(rmap.get(&2).unwrap().mtype, MessageType::Ok);
    assert_eq!(rmap.get(&10).unwrap().mtype, MessageType::Ok);
    assert_eq!(ctx.state, ManagerState::Running);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&10).unwrap().state, ClientState::Active);
}

#[test]
fn test_on_client_exit() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context_simple();
    let proc = ManagerProcRunning;

    // setup condition.
    let _ = ctx.sched.on_start(1);
    let _ = ctx.sched.on_ready(2);
    let _ = ctx.sched.on_ready(3);
    let _ = ctx.sched.on_ready(10);

    // send event.
    let m = Message::new(MessageType::Exit, 0, 0, None).unwrap();
    let result = proc.on_client_exit(&mut ctx, &m).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();

    // check result.
    assert_eq!(result.len(), 4);
    assert_eq!(rmap.get(&0).unwrap().mtype, MessageType::Ok);
    assert_eq!(rmap.get(&2).unwrap().mtype, MessageType::Error);
    assert_eq!(rmap.get(&3).unwrap().mtype, MessageType::Error);
    assert_eq!(rmap.get(&10).unwrap().mtype, MessageType::Error);
    assert_eq!(ctx.state, ManagerState::Exiting);
    assert_eq!(ctx.state_changed, true);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::None);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Exiting);
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Exiting);
    assert_eq!(ctx.clients.get(&10).unwrap().state, ClientState::Exiting);
    assert_eq!(ctx.num_active_clients, 4);
}

#[test]
fn test_on_shutdown() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context_simple();
    let proc = ManagerProcRunning;

    // setup condition.
    let _ = ctx.sched.on_ready(2);
    let _ = ctx.sched.on_start(3);
    let _ = ctx.sched.on_ready(10);

    // send event.
    let result = proc.on_shutdown(&mut ctx).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();

    // check result.
    assert_eq!(result.len(), 2);
    assert_eq!(rmap.get(&2).unwrap().mtype, MessageType::Error);
    assert_eq!(rmap.get(&10).unwrap().mtype, MessageType::Error);
    assert_eq!(ctx.state, ManagerState::Exiting);
    assert_eq!(ctx.state_changed, true);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Exiting);
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&10).unwrap().state, ClientState::Exiting);
    assert_eq!(ctx.num_active_clients, 5);
}
