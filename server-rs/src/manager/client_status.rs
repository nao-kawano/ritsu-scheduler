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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ClientState {
    None,                   // disconnected.
    Idle,                   // not in ready.
    Ready,                  // waiting for trigger(cycle or depends).
    Running { cycle: u32 }, // running.
    Exitting,               // exit requested.
}

#[derive(Debug, Clone)]
pub struct ClientStatus {
    pub config: ClientConfig,
    pub state: ClientState,
    depends_on: HashMap<u16, bool>,
}

impl ClientStatus {
    pub fn new(config: ClientConfig) -> Self {
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

    pub fn set_client_state(&mut self, state: ClientState) -> bool {
        info!(
            "client state: client={}, state {:?} -> {:?}",
            self.config.client_id, self.state, state
        );
        self.state = state;
        return true; /* always ok */
    }

    pub fn is_depends_ok(&self) -> bool {
        let mut ok = true;
        for depend_value in self.depends_on.values() {
            if *depend_value == false {
                ok = false;
                break;
            }
        }
        return ok;
    }

    pub fn clear_depends(&mut self) {
        for depend_value in self.depends_on.values_mut() {
            *depend_value = false;
        }
    }
}
