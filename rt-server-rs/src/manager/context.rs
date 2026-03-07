//!
//! Manager context.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use std::collections::HashMap;

use super::ManagerState;
use crate::config::ClientConfig;
use rt_core::ProcessEntry;
use rt_core::Scheduler;

#[cfg(test)]
#[path = "context_test.rs"]
mod context_test;

/* -------------------------------------------------------------------------- */

/// Represents the state of a client.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ClientState {
    /// Initial or Final state. Client will send `Join`.
    None,
    /// Client is Joined but not Ready. Client will send `Ready`.
    Sync,
    /// Client is Ready or Running. Controlling under ProcessGraph.
    Active,
    /// Client received `Error` and will send `Exit`.
    Exiting,
}

pub struct ClientInfo {
    pub config: ClientConfig,
    pub state: ClientState,
    pub last_mid: u8,
}

impl ClientInfo {
    /// Constructor.
    pub fn new(config: ClientConfig) -> Self {
        ClientInfo {
            config,
            state: ClientState::None,
            last_mid: 255,
        }
    }

    /// Set the state of the client.
    pub fn set_client_state(&mut self, state: ClientState, cycle: u32) -> bool {
        info!(
            "<STAT> CYC:{:05} CID:{:03} [Conn] {:?} -> {:?}",
            cycle, self.config.client_id, self.state, state
        );
        self.state = state;
        return true; /* always ok */
    }
}

/* -------------------------------------------------------------------------- */

pub struct ManagerContext {
    // manager.
    pub state: ManagerState,
    pub state_changed: bool,
    // for connection.
    pub clients: HashMap<u16, ClientInfo>,
    pub num_active_clients: usize,
    // for execution.
    pub cycle_current: u32, // 0..CYCLE_MAX
    pub sched: Scheduler,
    pub graph_start: Vec<u16>, // shortcut for cycle start.
}

impl ManagerContext {
    pub const CYCLE_MAX: u32 = 9999; // must be odd value for wrap-around.

    pub fn new(configs: Vec<ClientConfig>) -> Self {
        // at least one client must be provided.
        if configs.len() < 1 {
            panic!("client config is empty");
        }
        // create clients for connection management.
        let clients: HashMap<u16, ClientInfo> = configs
            .into_iter()
            .map(|config| (config.client_id, ClientInfo::new(config)))
            .collect();
        // create process entry for execution management.
        let mut graph_start: Vec<u16> = Vec::with_capacity(clients.len());
        let mut entries: HashMap<u16, ProcessEntry> = HashMap::with_capacity(clients.len());
        for client in clients.values() {
            let mut floating: bool = false;
            // verify dependency.
            for depend in &client.config.depends {
                if let Some(depend_client) = clients.get(depend) {
                    // All specified processes must have the same Cycle.
                    if depend_client.config.cycle != client.config.cycle {
                        panic!(
                            "ClientConfig {:03} dependent process {:03} has different cycle",
                            client.config.client_id, depend
                        );
                    }
                    // All specified processes must have same or smaller CycleOffset.
                    if depend_client.config.cycle_offset > client.config.cycle_offset {
                        panic!(
                            "ClientConfig {:03} dependent process {:03} has larger cycle_offset",
                            client.config.client_id, depend
                        );
                    }
                    // If the dependent process has the same cycle and cycle offset,
                    // this process starts immediately after the dependent process completes.
                    if depend_client.config.cycle_offset == client.config.cycle_offset {
                        floating = true;
                    }
                } else {
                    panic!(
                        "ClientConfig {:03} dependent process {:03} does not exist",
                        client.config.client_id, depend
                    );
                }
            }
            // insert entry.
            entries.insert(
                client.config.client_id,
                ProcessEntry::new(client.config.client_id, &client.config.depends, floating),
            );
            if !floating {
                graph_start.push(client.config.client_id);
            }
        }
        let graph: Scheduler = Scheduler::new(entries);
        // create context.
        ManagerContext {
            state: ManagerState::Starting,
            state_changed: false,
            clients,
            num_active_clients: 0,
            cycle_current: 0,
            sched: graph,
            graph_start,
        }
    }

    pub fn set_state(&mut self, state: ManagerState) -> bool {
        info!(
            "<STAT> CYC:{:05} [Manager] {:?} -> {:?}",
            self.cycle_current, self.state, state
        );
        if self.state != state {
            self.state = state;
            self.state_changed = true;
        }
        return true; /* always ok */
    }

    // -----
    // private methods.
}
