//!
//! Configuration for scheduler.
//!

use dps_message::CLIENT_ID_MAX;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub port: u16,
    pub cycle_time: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClientConfig {
    pub client_id: u16,
    pub cycle: u8,
    pub cycle_offset: u8,
    pub depends: Vec<u16>,
}

impl ClientConfig {
    pub fn new(
        client_id: u16,
        cycle: u8,
        cycle_offset: u8,
        depends: Vec<u16>,
    ) -> Result<Self, String> {
        // validate client_id.
        if client_id > CLIENT_ID_MAX {
            return Err(format!(
                "[ClientConfig {:03}] Client ID {:03} is too large",
                client_id, client_id
            ));
        }
        // validate cycle.
        if cycle == 0 {
            return Err(format!(
                "[ClientConfig {:03}] Cycle must not be zero",
                client_id
            ));
        }
        // validate cycle_offset.
        if cycle_offset >= cycle {
            return Err(format!(
                "[ClientConfig {:03}] CycleOffset must be less than trigger cycle",
                client_id
            ));
        }
        // validate depends.
        for depend in &depends {
            if *depend > CLIENT_ID_MAX {
                return Err(format!(
                    "[ClientConfig {:03}] Depends {:03} is too large",
                    client_id, depend
                ));
            }
        }
        // pass.
        Ok(Self {
            client_id,
            cycle,
            cycle_offset,
            depends,
        })
    }

    // -----
    // private methods.
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SchedulerConfig {
    pub server_config: ServerConfig,
    pub client_configs: Vec<ClientConfig>,
}
