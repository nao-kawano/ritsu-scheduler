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

use rt_config::{ClientConfig, SchedulerConfig, ServerConfig};

fn create_context() -> ManagerContext {
    let client_configs = vec![
        ClientConfig::new(0, 1, 0, vec![], 0).unwrap(),
        ClientConfig::new(1, 2, 1, vec![], 0).unwrap(),
        ClientConfig::new(2, 1, 0, vec![0], 0).unwrap(),
    ];
    let server_config = ServerConfig {
        port: 8080,
        cycle_time_ms: 50,
        stats_interval_cycle: 0,
    };
    let scheduler_config = SchedulerConfig {
        server_config,
        client_configs,
    };
    let rules = scheduler_config.get_client_rules();

    let mut ctx = ManagerContext::new(scheduler_config.client_configs, rules, 0);
    ctx.state = ManagerState::Starting;
    ctx.clients.get_mut(&0).unwrap().last_mid = 9;
    ctx.clients.get_mut(&1).unwrap().last_mid = 9;
    ctx.clients.get_mut(&2).unwrap().last_mid = 9;
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
    let m = Message::new(
        MessageType::Join,
        0,
        1,
        Some(vec![(
            "version".to_string(),
            rt_message::PROTOCOL_VERSION.to_string(),
        )]),
    )
    .unwrap();
    let result = proc.on_client_join(&mut ctx, &m).unwrap();

    // check result.
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].mtype, MessageType::Joined);
    assert_eq!(
        result[0].get_extra("version"),
        Some(&rt_message::PROTOCOL_VERSION.to_string())
    );
    assert_eq!(result[0].cid, 1);
    assert_eq!(ctx.state, ManagerState::Starting);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::None);
    assert_eq!(ctx.num_active_clients, 2);

    // send event.
    let m = Message::new(
        MessageType::Join,
        0,
        2,
        Some(vec![(
            "version".to_string(),
            rt_message::PROTOCOL_VERSION.to_string(),
        )]),
    )
    .unwrap();
    let result = proc.on_client_join(&mut ctx, &m).unwrap();

    // check result.
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].mtype, MessageType::Joined);
    assert_eq!(
        result[0].get_extra("version"),
        Some(&rt_message::PROTOCOL_VERSION.to_string())
    );
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
    let m = Message::new(MessageType::Ready, 0, 0, None).unwrap();
    let result = proc.on_client_ready(&mut ctx, &m).unwrap();

    // check result.
    assert_eq!(result.len(), 0);
    assert_eq!(ctx.state, ManagerState::Starting);
    assert_eq!(ctx.state_changed, false);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Active);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.num_active_clients, 3);

    // send event.
    let m = Message::new(MessageType::Ready, 0, 2, None).unwrap();
    let result = proc.on_client_ready(&mut ctx, &m).unwrap();

    // check result.
    assert_eq!(result.len(), 0);
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
    let m = Message::new(MessageType::Done, 0, 0, None).unwrap();
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
    let m = Message::new(MessageType::Done, 0, 2, None).unwrap();
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
    let m = Message::new(MessageType::Ready, 0, 1, None).unwrap();
    let _ = proc.on_client_ready(&mut ctx, &m).unwrap();

    // send event.
    let m = Message::new(MessageType::Exit, 0, 0, None).unwrap();
    let result = proc.on_client_exit(&mut ctx, &m).unwrap();

    // check result.
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].mtype, MessageType::Ok);
    assert_eq!(result[0].cid, 0);
    assert_eq!(result[1].mtype, MessageType::Error);
    assert_eq!(
        result[1].get_extra("reason"),
        Some(&"ClientExit".to_string())
    );
    assert_eq!(result[1].get_extra("cid"), Some(&"000".to_string()));
    assert_eq!(result[1].cid, 1);
    assert_eq!(ctx.state, ManagerState::Exiting);
    assert_eq!(ctx.state_changed, true);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::None);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Exiting);
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
    assert_eq!(ctx.state, ManagerState::Exited);
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
    let m = Message::new(MessageType::Ready, 0, 1, None).unwrap();
    let _ = proc.on_client_ready(&mut ctx, &m).unwrap();

    // send event.
    let result = proc.on_shutdown(&mut ctx).unwrap();

    // check result.
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].mtype, MessageType::Error);
    assert_eq!(result[0].get_extra("reason"), Some(&"Shutdown".to_string()));
    assert_eq!(result[0].cid, 1);
    assert_eq!(ctx.state, ManagerState::Exiting);
    assert_eq!(ctx.state_changed, true);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.clients.get(&1).unwrap().state, ClientState::Exiting);
    assert_eq!(ctx.clients.get(&2).unwrap().state, ClientState::Sync);
    assert_eq!(ctx.num_active_clients, 3);
}

#[test]
fn test_on_client_join_incompatible_version() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context();
    let proc = ManagerProcStarting;

    // setup condition.
    ctx.clients.get_mut(&0).unwrap().state = ClientState::None;
    ctx.num_active_clients = 0;

    // send event.
    let m = Message::new(
        MessageType::Join,
        0,
        0,
        Some(vec![("version".to_string(), "999".to_string())]),
    )
    .unwrap();
    let result = proc.on_client_join(&mut ctx, &m).unwrap();

    // check result.
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].mtype, MessageType::Error);
    assert_eq!(
        result[0].get_extra("reason"),
        Some(&"IncompatibleVersion".to_string())
    );
    assert_eq!(result[0].cid, 0);
    assert_eq!(ctx.state, ManagerState::Starting);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::None);
    assert_eq!(ctx.num_active_clients, 0);
}

#[test]
fn test_on_client_join_missing_version() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut ctx = create_context();
    let proc = ManagerProcStarting;

    // setup condition.
    ctx.clients.get_mut(&0).unwrap().state = ClientState::None;
    ctx.num_active_clients = 0;

    // send event.
    let m = Message::new(MessageType::Join, 0, 0, None).unwrap();
    let result = proc.on_client_join(&mut ctx, &m).unwrap();

    // check result.
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].mtype, MessageType::Error);
    assert_eq!(
        result[0].get_extra("reason"),
        Some(&"IncompatibleVersion".to_string())
    );
    assert_eq!(result[0].cid, 0);
    assert_eq!(ctx.state, ManagerState::Starting);
    assert_eq!(ctx.clients.get(&0).unwrap().state, ClientState::None);
    assert_eq!(ctx.num_active_clients, 0);
}
