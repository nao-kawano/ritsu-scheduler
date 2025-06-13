//!
//! Manager context.
//!

use std::cmp::max;
use std::collections::HashMap;

use super::ManagerState;
use super::client_status::ClientStatus;
use crate::config::{ClientConfig, TriggerType};

/* -------------------------------------------------------------------------- */

pub struct ManagerContext {
    pub state: ManagerState,
    pub state_changed: bool,
    clients: HashMap<u16, ClientStatus>,
    num_active_clients: usize,
    cycle_current: u32,
    cycle_max: u32,
}

impl ManagerContext {
    pub fn new(configs: Vec<ClientConfig>) -> Self {
        let (clients, cycle_max) = ManagerContext::create_client_status(configs);
        ManagerContext {
            state: ManagerState::Starting,
            state_changed: false,
            clients,
            num_active_clients: 0,
            cycle_current: 0,
            cycle_max,
        }
    }

    pub fn set_state(&mut self, state: ManagerState) -> bool {
        println!("state: {:?} -> {:?}", self.state, state);
        if self.state != state {
            self.state = state;
            self.state_changed = true;
        }
        return true; /* always ok */
    }

    // -----
    // private methods.

    fn create_client_status(configs: Vec<ClientConfig>) -> (HashMap<u16, ClientStatus>, u32) {
        // at least one client config must be provided.
        if configs.len() < 1 {
            panic!("client config is empty");
        }
        // at least one client has a trigger=Cycle.
        if !configs
            .iter()
            .any(|c| matches!(c.trigger_type, TriggerType::Cycle(_)))
        {
            panic!("client config has no trigger=Cycle");
        }
        // create status.
        let mut clients: HashMap<u16, ClientStatus> = HashMap::with_capacity(configs.len());
        let mut cycle_max: u32 = 0;
        for config in configs {
            if let TriggerType::Cycle(c) = config.trigger_type {
                let cmax = c as u32 * 10; // with mergin.
                cycle_max = max(cycle_max, cmax);
            }
            clients.insert(config.client_id, ClientStatus::new(config));
        }
        // TODO: verify that depend id is exist.
        // TODO: verify that dependencies are initiated by a cycle trigger.
        return (clients, cycle_max);
    }
}
