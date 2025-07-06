//!
//! Manager context.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use std::collections::{HashMap, HashSet};

use super::ManagerState;
use super::client_status::ClientStatus;
use crate::config::ClientConfig;

#[cfg(test)]
#[path = "context_test.rs"]
mod context_test;

/* -------------------------------------------------------------------------- */

pub struct ManagerContext {
    pub state: ManagerState,
    pub state_changed: bool,
    pub clients: HashMap<u16, ClientStatus>,
    pub num_active_clients: usize,
    pub cycle_current: u32, // 0..CYCLE_MAX
    // dependency graph.
    pub graph_start: HashSet<u16>,
    pub graph_forward: HashMap<u16, HashSet<u16>>,
}

impl ManagerContext {
    pub const CYCLE_MAX: u32 = 9999; // must be odd value for wrap-around.

    pub fn new(configs: Vec<ClientConfig>) -> Self {
        let (graph_start, graph_forward) = ManagerContext::create_graph(&configs);
        let clients: HashMap<u16, ClientStatus> = configs
            .into_iter()
            .map(|config| (config.client_id, ClientStatus::new(config)))
            .collect();
        ManagerContext {
            state: ManagerState::Starting,
            state_changed: false,
            clients,
            num_active_clients: 0,
            cycle_current: 0,
            graph_start,
            graph_forward,
        }
    }

    pub fn set_state(&mut self, state: ManagerState) -> bool {
        info!("state: {:?} -> {:?}", self.state, state);
        if self.state != state {
            self.state = state;
            self.state_changed = true;
        }
        return true; /* always ok */
    }

    // -----
    // private methods.

    fn create_graph(configs: &Vec<ClientConfig>) -> (HashSet<u16>, HashMap<u16, HashSet<u16>>) {
        // at least one client must be provided.
        if configs.len() < 1 {
            panic!("client config is empty");
        }
        // build id list for verify.
        let config_map: HashMap<u16, &ClientConfig> =
            configs.iter().map(|c| (c.client_id, c)).collect();
        // find start points.
        let mut start_points: HashSet<u16> = HashSet::new();
        for config in configs.iter().filter(|c| c.depends.len() == 0) {
            start_points.insert(config.client_id);
        }
        // - verify that at least one start point is exist.
        if start_points.len() < 1 {
            panic!("client config has no start point");
        }
        // create forward dependency by reverse.
        let mut forward_dependencies: HashMap<u16, HashSet<u16>> = HashMap::new();
        for config in configs {
            if config.depends.len() > 0 {
                let mut is_cyclic = true;
                for depend in &config.depends {
                    // - verify that dependent client exists.
                    let Some(dependent_config) = config_map.get(depend) else {
                        panic!("dependent client {} does not exist", depend);
                    };
                    // - verify that dependent client has same cycle.
                    if dependent_config.cycle != config.cycle {
                        panic!(
                            "dependent client {} has different cycle {}",
                            depend, config.cycle
                        );
                    }
                    // check if it is floating.
                    if dependent_config.cycle_offset == config.cycle_offset {
                        is_cyclic = false;
                    }
                    // add forward dependency.
                    forward_dependencies
                        .entry(*depend)
                        .or_insert(HashSet::new())
                        .insert(config.client_id);
                }
                // add to start_points if it is cyclic with dependency.
                if is_cyclic {
                    start_points.insert(config.client_id);
                }
            }
        }
        // ok.
        return (start_points, forward_dependencies);
    }
}
