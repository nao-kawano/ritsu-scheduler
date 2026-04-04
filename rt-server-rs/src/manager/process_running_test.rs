#[cfg(test)]
use super::*;

use crate::ManagerState;
use crate::manager::context::ClientState;

use std::collections::HashMap;

fn create_context_scenarios() -> ManagerContext {
    let mut ctx = ManagerContext::new(
        vec![
            // CID, Cycle, Offset, Depends
            ClientConfig::new(0, 2, 0, vec![], 0).unwrap(),
            ClientConfig::new(1, 2, 0, vec![0], 0).unwrap(), // Floating (depends on 0, same offset)
            ClientConfig::new(2, 2, 1, vec![1], 0).unwrap(), // Non-Floating (depends on 1, different offset)
            ClientConfig::new(3, 2, 1, vec![], 0).unwrap(),  // Root (offset 1)
        ],
        0,
    );
    ctx.state = ManagerState::Running;
    ctx.num_active_clients = ctx.clients.len();
    for client in ctx.clients.values_mut() {
        client.state = ClientState::Active;
        client.last_mid = 9;
    }
    ctx
}

#[test]
fn test_enter_state() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context_scenarios();
    let proc = ManagerProcRunning;

    // setup condition.
    ctx.cycle_current = 5;

    // send event.
    proc.enter_state(&mut ctx);

    // check result.
    assert_eq!(ctx.state, ManagerState::Running);
    assert_eq!(ctx.cycle_current, ManagerContext::CYCLE_MAX);
}

#[test]
fn test_normal_cycle_flow() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context_scenarios();
    let proc = ManagerProcRunning;

    // setup condition.
    proc.enter_state(&mut ctx);
    for i in 0..=3 {
        let _ = ctx.sched.on_ready(i);
    }

    // --- Cycle 0 ---
    // CID 0 should start.
    let result = proc.on_cycle_start(&mut ctx, 123).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();
    assert_eq!(result.len(), 1);
    assert_eq!(rmap.get(&0).unwrap().mtype, MessageType::Start);
    assert_eq!(
        rmap.get(&0).unwrap().get_extra("cycle"),
        Some(&"0".to_string())
    );

    // CID 0 sends DONE -> CID 1 should start immediately (Floating).
    let m_done0 = Message::new(MessageType::Done, 1, 0, None).unwrap();
    let result = proc.on_client_done(&mut ctx, &m_done0).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();
    assert_eq!(result.len(), 2);
    assert_eq!(rmap.get(&0).unwrap().mtype, MessageType::Ok); // Ack for 0
    assert_eq!(rmap.get(&1).unwrap().mtype, MessageType::Start); // Start for 1
    assert_eq!(
        rmap.get(&1).unwrap().get_extra("cycle"),
        Some(&"0".to_string())
    );

    // CID 1 sends DONE -> CID 2 is Ready but waits for Offset 1 (Non-Floating).
    let m_done1 = Message::new(MessageType::Done, 5, 1, None).unwrap();
    let result = proc.on_client_done(&mut ctx, &m_done1).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();
    assert_eq!(result.len(), 1);
    assert_eq!(rmap.get(&1).unwrap().mtype, MessageType::Ok); // Ack for 1

    // --- Cycle 1 ---
    // CID 2 (dependency met) and CID 3 (root) should start.
    let result = proc.on_cycle_start(&mut ctx, 124).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();
    assert_eq!(result.len(), 2);
    assert_eq!(rmap.get(&2).unwrap().mtype, MessageType::Start); // Start for 2
    assert_eq!(
        rmap.get(&2).unwrap().get_extra("cycle"),
        Some(&"1".to_string())
    );
    assert_eq!(rmap.get(&3).unwrap().mtype, MessageType::Start); // Start for 3
    assert_eq!(
        rmap.get(&3).unwrap().get_extra("cycle"),
        Some(&"1".to_string())
    );
}

#[test]
fn test_client_ready_late() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context_scenarios();
    let proc = ManagerProcRunning;

    // setup condition.
    proc.enter_state(&mut ctx);
    for i in 1..=3 {
        let _ = ctx.sched.on_ready(i);
    }

    // --- Cycle 0 ---
    // CID 0 is in Idle, mark as Late.
    // CID 1, 2 is Skip.
    let result = proc.on_cycle_start(&mut ctx, 123).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();
    assert_eq!(result.len(), 2);
    assert_eq!(rmap.get(&1).unwrap().mtype, MessageType::Skip);
    assert_eq!(rmap.get(&2).unwrap().mtype, MessageType::Skip);

    // CID 1, 2 sends READY again.
    let m_ready1 = Message::new(MessageType::Ready, 1, 1, None).unwrap();
    let result = proc.on_client_ready(&mut ctx, &m_ready1).unwrap();
    assert_eq!(result.len(), 0);
    let m_ready2 = Message::new(MessageType::Ready, 1, 2, None).unwrap();
    let result = proc.on_client_ready(&mut ctx, &m_ready2).unwrap();
    assert_eq!(result.len(), 0);

    // Late Ready arrives from CID 0.
    let m_ready0 = Message::new(MessageType::Ready, 1, 0, None).unwrap();
    let result = proc.on_client_ready(&mut ctx, &m_ready0).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();
    assert_eq!(result.len(), 1);
    assert_eq!(rmap.get(&0).unwrap().mtype, MessageType::Late);

    // Rtry Ready arrives from CID 0.
    let m_ready0 = Message::new(MessageType::Ready, 2, 0, None).unwrap();
    let result = proc.on_client_ready(&mut ctx, &m_ready0).unwrap();
    assert_eq!(result.len(), 0);
}

#[test]
fn test_overrun_and_skip_chain() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context_scenarios();
    let proc = ManagerProcRunning;

    // setup condition.
    proc.enter_state(&mut ctx);
    for i in 0..=3 {
        let _ = ctx.sched.on_ready(i);
    }

    // --- Cycle 0 ---
    // CID 0 starts.
    let _ = proc.on_cycle_start(&mut ctx, 123).unwrap();
    // CID 0 is still RUNNING (Overrun trigger).

    // --- Cycle 1 ---
    // CID 3 starts, CID 2 waits for 1.
    let _ = proc.on_cycle_start(&mut ctx, 124).unwrap();

    // CID 3 completed.
    let m_done3 = Message::new(MessageType::Done, 1, 3, None).unwrap();
    let _ = proc.on_client_done(&mut ctx, &m_done3).unwrap();
    let m_ready3 = Message::new(MessageType::Ready, 2, 3, None).unwrap();
    let _ = proc.on_client_ready(&mut ctx, &m_ready3).unwrap();

    // --- Cycle 2 ---
    // CID 0 is still Running -> Overrun.
    // Dependents (CID 1, and eventually CID 2) should be Skipped.
    let result = proc.on_cycle_start(&mut ctx, 125).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();
    assert_eq!(result.len(), 2);
    assert_eq!(rmap.get(&1).unwrap().mtype, MessageType::Skip);
    assert_eq!(rmap.get(&2).unwrap().mtype, MessageType::Skip);
}

#[test]
fn test_retransmission_handling() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context_scenarios();
    let proc = ManagerProcRunning;

    // setup condition.
    proc.enter_state(&mut ctx);
    for i in 0..=3 {
        let _ = ctx.sched.on_ready(i);
    }

    // 1. Ready retransmission while Running.
    let _ = proc.on_cycle_start(&mut ctx, 123); // CID 0 is Running.
    let m_ready0 = Message::new(MessageType::Ready, 1, 0, None).unwrap();
    let result = proc.on_client_ready(&mut ctx, &m_ready0).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();
    assert_eq!(result.len(), 1);
    assert_eq!(rmap.get(&0).unwrap().mtype, MessageType::Start); // Should allow to continue.
    assert_eq!(
        rmap.get(&0).unwrap().get_extra("cycle"),
        Some(&"0".to_string())
    );

    // 2. Done retransmission while Idle.
    let m_done0 = Message::new(MessageType::Done, 2, 0, None).unwrap();
    let _ = proc.on_client_done(&mut ctx, &m_done0); // CID 0 becomes Idle.
    let result = proc.on_client_done(&mut ctx, &m_done0).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();
    assert_eq!(result.len(), 1);
    assert_eq!(rmap.get(&0).unwrap().mtype, MessageType::Ok); // Ack for retransmission.
}

#[test]
fn test_on_client_join() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context_scenarios();
    let proc = ManagerProcRunning;

    // setup condition.
    proc.enter_state(&mut ctx);

    // ignore JOIN.
    let m = Message::new(MessageType::Join, 1, 0, None).unwrap();
    let result = proc.on_client_join(&mut ctx, &m);
    assert!(result.is_err());
}

#[test]
fn test_on_client_exit() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context_scenarios();
    let proc = ManagerProcRunning;

    // setup condition.
    proc.enter_state(&mut ctx);
    for i in 0..=3 {
        let _ = ctx.sched.on_ready(i);
    }

    // --- Cycle 0 ---
    // CID 0 starts.
    let _ = proc.on_cycle_start(&mut ctx, 123).unwrap();

    // CID 0 exits.
    // Manager should transition to Exiting and notify others.
    let m_exit0 = Message::new(MessageType::Exit, 5, 0, None).unwrap();
    let result = proc.on_client_exit(&mut ctx, &m_exit0).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();
    assert_eq!(result.len(), 4);
    assert_eq!(rmap.get(&0).unwrap().mtype, MessageType::Ok);
    assert_eq!(rmap.get(&1).unwrap().mtype, MessageType::Error);
    assert_eq!(
        rmap.get(&1).unwrap().get_extra("reason"),
        Some(&"ClientExit".to_string())
    );
    assert_eq!(
        rmap.get(&1).unwrap().get_extra("cid"),
        Some(&"000".to_string())
    );
    assert_eq!(rmap.get(&2).unwrap().mtype, MessageType::Error);
    assert_eq!(rmap.get(&3).unwrap().mtype, MessageType::Error);
    // Manager state change to Exitting.
    assert_eq!(ctx.state, ManagerState::Exiting);
    assert_eq!(ctx.state_changed, true);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::None);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Exiting);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Exiting);
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Exiting);
    assert_eq!(ctx.num_active_clients, 3);
}

#[test]
fn test_on_shutdown() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context_scenarios();
    let proc = ManagerProcRunning;

    // setup condition.
    proc.enter_state(&mut ctx);
    for i in 2..=3 {
        let _ = ctx.sched.on_ready(i);
    }

    // send event.
    let result = proc.on_shutdown(&mut ctx).unwrap();
    let rmap: HashMap<u16, &Message> = result.iter().map(|m| (m.cid, m)).collect();
    assert_eq!(result.len(), 2);
    assert_eq!(rmap.get(&2).unwrap().mtype, MessageType::Error);
    assert_eq!(
        rmap.get(&2).unwrap().get_extra("reason"),
        Some(&"Shutdown".to_string())
    );
    assert_eq!(rmap.get(&3).unwrap().mtype, MessageType::Error);
    // Manager should transition to Exiting and notify others.
    assert_eq!(ctx.state, ManagerState::Exiting);
    assert_eq!(ctx.state_changed, true);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Exiting);
    assert_eq!(ctx.clients.get(&3).unwrap().state, ClientState::Exiting);
    assert_eq!(ctx.num_active_clients, 4);
}
