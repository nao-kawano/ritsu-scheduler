#[cfg(test)]
use super::*;
use crate::config::*;

fn create_context() -> ManagerContext {
    let mut ctx = ManagerContext::new(vec![
        ClientConfig::new(0, 1, 0, vec![]).unwrap(),
        ClientConfig::new(1, 2, 1, vec![]).unwrap(),
        ClientConfig::new(2, 1, 0, vec![0]).unwrap(),
    ]);
    ctx.state = ManagerState::Starting;
    return ctx;
}

#[test]
fn test_enter_state() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context();
    let proc = ManagerProcStarting;

    // setup condition.
    // do nothing.

    // send event.
    proc.enter_state(&mut ctx);

    // check result.
    assert_eq!(ctx.state, ManagerState::Starting);
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
    let proc = ManagerProcStarting;

    // setup condition.
    ctx.clients.get_mut(&0).unwrap().state = ClientState::Sync;
    ctx.num_active_clients = 1;

    // send event.
    let result = proc.on_cycle_start(&mut ctx, 100).unwrap();

    // check result.
    assert_eq!(result.len(), 0);
    assert_eq!(ctx.state, ManagerState::Starting);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::None);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::None);
    assert_eq!(ctx.num_active_clients, 1);
}

#[test]
fn test_on_client_join() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context();
    let proc = ManagerProcStarting;

    // setup condition.
    ctx.clients.get_mut(&0).unwrap().state = ClientState::Sync;
    ctx.num_active_clients = 1;

    // send event.
    let m = Message::new(MessageType::Join, 1, None).unwrap();
    let result = proc.on_client_join(&mut ctx, &m).unwrap();

    // check result.
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].mtype, MessageType::Ok);
    assert_eq!(result[0].cid, 1);
    assert_eq!(ctx.state, ManagerState::Starting);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::None);
    assert_eq!(ctx.num_active_clients, 2);

    // send event.
    let m = Message::new(MessageType::Join, 2, None).unwrap();
    let result = proc.on_client_join(&mut ctx, &m).unwrap();

    // check result.
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].mtype, MessageType::Ok);
    assert_eq!(result[0].cid, 2);
    assert_eq!(ctx.state, ManagerState::Starting);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.num_active_clients, 3);
}

#[test]
fn test_on_client_ready() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context();
    let proc = ManagerProcStarting;

    // setup condition.
    ctx.clients.get_mut(&0).unwrap().state = ClientState::Sync;
    ctx.clients.get_mut(&1).unwrap().state = ClientState::Active;
    ctx.clients.get_mut(&2).unwrap().state = ClientState::Sync;
    ctx.num_active_clients = 3;

    // send event.
    let m = Message::new(MessageType::Ready, 0, None).unwrap();
    let result = proc.on_client_ready(&mut ctx, &m).unwrap();

    // check result.
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].mtype, MessageType::Ok);
    assert_eq!(result[0].cid, 0);
    assert_eq!(ctx.state, ManagerState::Starting);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.num_active_clients, 3);

    // send event.
    let m = Message::new(MessageType::Ready, 2, None).unwrap();
    let result = proc.on_client_ready(&mut ctx, &m).unwrap();

    // check result.
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].mtype, MessageType::Ok);
    assert_eq!(result[0].cid, 2);
    assert_eq!(ctx.state, ManagerState::Running);
    assert_eq!(ctx.state_changed, true);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Active);
    assert_eq!(ctx.num_active_clients, 3);
}

#[test]
fn test_on_client_done() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context();
    let proc = ManagerProcStarting;

    // setup condition.
    ctx.clients.get_mut(&0).unwrap().state = ClientState::Sync;
    ctx.clients.get_mut(&1).unwrap().state = ClientState::Active;
    ctx.clients.get_mut(&2).unwrap().state = ClientState::Sync;
    ctx.num_active_clients = 3;

    // send event.
    let m = Message::new(MessageType::Done, 0, None).unwrap();
    let result = proc.on_client_done(&mut ctx, &m).unwrap();

    // check result.
    assert_eq!(result.len(), 0);
    assert_eq!(ctx.state, ManagerState::Starting);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.num_active_clients, 3);

    // send event.
    let m = Message::new(MessageType::Done, 2, None).unwrap();
    let result = proc.on_client_done(&mut ctx, &m).unwrap();

    // check result.
    assert_eq!(result.len(), 0);
    assert_eq!(ctx.state, ManagerState::Starting);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.num_active_clients, 3);
}

#[test]
fn test_on_client_exit() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context();
    let proc = ManagerProcStarting;

    // setup condition.
    ctx.clients.get_mut(&0).unwrap().state = ClientState::Sync;
    ctx.clients.get_mut(&1).unwrap().state = ClientState::Sync;
    ctx.clients.get_mut(&2).unwrap().state = ClientState::Sync;
    ctx.num_active_clients = 3;
    let m = Message::new(MessageType::Ready, 1, None).unwrap();
    let _ = proc.on_client_ready(&mut ctx, &m).unwrap();

    // send event.
    let m = Message::new(MessageType::Exit, 0, None).unwrap();
    let result = proc.on_client_exit(&mut ctx, &m).unwrap();

    // check result.
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].mtype, MessageType::Ok);
    assert_eq!(result[0].cid, 0);
    assert_eq!(result[1].mtype, MessageType::Error);
    assert_eq!(result[1].cid, 1);
    assert_eq!(ctx.state, ManagerState::Exitting);
    assert_eq!(ctx.state_changed, true);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::None);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Exitting);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.num_active_clients, 2);
}

#[test]
fn test_on_shutdown_no_client() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context();
    let proc = ManagerProcStarting;

    // setup condition.
    ctx.clients.get_mut(&0).unwrap().state = ClientState::None;
    ctx.clients.get_mut(&1).unwrap().state = ClientState::None;
    ctx.clients.get_mut(&2).unwrap().state = ClientState::None;
    ctx.num_active_clients = 0;

    // send event.
    let result = proc.on_shutdown(&mut ctx).unwrap();

    // check result.
    assert_eq!(result.len(), 0);
    assert_eq!(ctx.state, ManagerState::Exitted);
    assert_eq!(ctx.state_changed, true);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::None);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::None);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::None);
    assert_eq!(ctx.num_active_clients, 0);
}

#[test]
fn test_on_shutdown() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context();
    let proc = ManagerProcStarting;

    // setup condition.
    ctx.clients.get_mut(&0).unwrap().state = ClientState::Sync;
    ctx.clients.get_mut(&1).unwrap().state = ClientState::Sync;
    ctx.clients.get_mut(&2).unwrap().state = ClientState::Sync;
    ctx.num_active_clients = 3;
    let m = Message::new(MessageType::Ready, 1, None).unwrap();
    let _ = proc.on_client_ready(&mut ctx, &m).unwrap();

    // send event.
    let result = proc.on_shutdown(&mut ctx).unwrap();

    // check result.
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].mtype, MessageType::Error);
    assert_eq!(result[0].cid, 1);
    assert_eq!(ctx.state, ManagerState::Exitting);
    assert_eq!(ctx.state_changed, true);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Exitting);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.num_active_clients, 3);
}
