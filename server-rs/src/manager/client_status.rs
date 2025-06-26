//!
//! Client status in manager.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::config::{ClientConfig, TriggerType};
use std::collections::HashMap;

#[cfg(test)]
#[path = "client_status_test.rs"]
mod client_status_test;

/* -------------------------------------------------------------------------- */

/// Represents the state of a client.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ClientState {
    /// Initial or Final state. Client will send `Join`.
    None,
    /// Before Ready. Client will send `Ready`.
    Idle,
    /// Client is waiting for trigger(cycle or depends).
    Ready,
    /// Client is processing started in `cycle`. Client will send `Done` after processing.
    Running { cycle: u32 },
    /// Client received `Error` and will send `Exit`.
    Exitting,
}

/// Manages the state of a client.
#[derive(Debug, Clone)]
pub struct ClientStatus {
    /// Client information such as id, trigger, etc.
    pub config: ClientConfig,
    /// Current state of the client.
    pub state: ClientState,
    /// Dependency status for 'Depends' trigger.
    /// Client can start when all value is true.
    depends_on: HashMap<u16, bool>,
}

impl ClientStatus {
    /// Constructor.
    pub fn new(config: ClientConfig) -> Self {
        // If the `TriggerType` is `Depends`, initialize the dependency status.
        let mut depends_on: HashMap<u16, bool> = HashMap::new();
        if let TriggerType::Depends { clients } = &&config.trigger_type {
            for client in clients {
                depends_on.insert(*client, false);
            }
        }
        ClientStatus {
            config,
            state: ClientState::None,
            depends_on,
        }
    }

    /// Set the state of the client.
    pub fn set_client_state(&mut self, state: ClientState) -> bool {
        info!(
            "client state: client={}, state {:?} -> {:?}",
            self.config.client_id, self.state, state
        );
        self.state = state;
        return true; /* always ok */
    }

    /// Check if the client is ready to start.
    pub fn is_depends_ok(&self) -> bool {
        let mut is_ready = true;
        for depend_value in self.depends_on.values() {
            if *depend_value == false {
                is_ready = false;
                break;
            }
        }
        return is_ready;
    }

    /// Update the dependency.
    pub fn update_depend(&mut self, client_id: u16) {
        if let Some(depend_value) = self.depends_on.get_mut(&client_id) {
            *depend_value = true;
        }
    }

    /// Clear the dependency status.
    pub fn clear_depends(&mut self) {
        for depend_value in self.depends_on.values_mut() {
            *depend_value = false;
        }
    }

    // -----
    // private methods.
}
