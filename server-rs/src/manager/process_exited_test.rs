#[cfg(test)]
use super::*;
use crate::config::*;
use crate::manager::ManagerState;
use crate::manager::context::{ClientState, ManagerContext};

use dps_message::{Message, MessageType};

fn create_context() -> ManagerContext {
    let mut ctx = ManagerContext::new(vec![
        ClientConfig::new(0, 1, 0, vec![]).unwrap(),
        ClientConfig::new(1, 2, 1, vec![]).unwrap(),
        ClientConfig::new(2, 1, 0, vec![0]).unwrap(),
    ]);
    ctx.state = ManagerState::Exited;
    return ctx;
}

fn check_context_changed(ctx: &ManagerContext) -> bool {
    let mut ret = false;
    ret |= ctx.state != ManagerState::Exited;
    ret |= ctx.state_changed != false;
    ret |= ctx.clients.get(&0).unwrap().state != ClientState::None;
    ret |= ctx.clients.get(&1).unwrap().state != ClientState::None;
    ret |= ctx.clients.get(&2).unwrap().state != ClientState::None;
    ret |= ctx.num_active_clients != 0;
    return ret;
}

#[test]
fn test_enter_state() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context();
    let proc = ManagerProcExited;

    // setup condition.
    // do nothing.

    // send event.
    proc.enter_state(&mut ctx);

    // check result.
    assert_eq!(ctx.state, ManagerState::Exited);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::None);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::None);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::None);
    assert_eq!(ctx.num_active_clients, 0);
}

#[test]
fn test_on_cycle_start() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context();
    let proc = ManagerProcExited;

    // setup condition.
    // do nothing.

    // send event and check no-effect.
    let result = proc.on_cycle_start(&mut ctx, 100);
    assert_eq!(result.is_err(), true);
    assert_eq!(check_context_changed(&ctx), false);
}

#[test]
fn test_on_client_join() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context();
    let proc = ManagerProcExited;

    // setup condition.
    // do nothing.

    // send event and check no-effect.
    let m = Message::new(MessageType::Join, 0, 0, None).unwrap();
    let result = proc.on_client_join(&mut ctx, &m);
    assert_eq!(result.is_err(), true);
    assert_eq!(check_context_changed(&ctx), false);
}

#[test]
fn test_on_client_ready() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context();
    let proc = ManagerProcExited;

    // setup condition.
    // do nothing.

    // send event and check no-effect.
    let m = Message::new(MessageType::Ready, 0, 0, None).unwrap();
    let result = proc.on_client_join(&mut ctx, &m);
    assert_eq!(result.is_err(), true);
    assert_eq!(check_context_changed(&ctx), false);
}

#[test]
fn test_on_client_done() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context();
    let proc = ManagerProcExited;

    // setup condition.
    // do nothing.

    // send event and check no-effect.
    let m = Message::new(MessageType::Done, 0, 0, None).unwrap();
    let result = proc.on_client_join(&mut ctx, &m);
    assert_eq!(result.is_err(), true);
    assert_eq!(check_context_changed(&ctx), false);
}

#[test]
fn test_on_client_exit() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context();
    let proc = ManagerProcExited;

    // setup condition.
    // do nothing.

    // send event and check no-effect.
    let m = Message::new(MessageType::Exit, 0, 0, None).unwrap();
    let result = proc.on_client_join(&mut ctx, &m);
    assert_eq!(result.is_err(), true);
    assert_eq!(check_context_changed(&ctx), false);
}

#[test]
fn test_on_shutdown() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context();
    let proc = ManagerProcExited;

    // setup condition.
    // do nothing.

    // send event and check no-effect.
    let result = proc.on_shutdown(&mut ctx);
    assert_eq!(result.is_err(), true);
    assert_eq!(check_context_changed(&ctx), false);
}
