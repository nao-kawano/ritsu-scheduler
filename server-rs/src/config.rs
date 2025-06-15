//!
//! Configuration for scheduler.
//!

use dps_message::CLIENT_ID_MAX;

pub struct ServerConfig {
    pub port: u16,
    pub cycle_time: u16,
}

#[derive(Clone, Debug)]
pub enum TriggerType {
    Cycle(u8),
    Depends {
        clients: Vec<u16>, /* len() must be > 0 */
    },
}

#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub client_id: u16,
    pub trigger_type: TriggerType,
    pub cycle_offset: u8,
}

impl ClientConfig {
    pub fn new(
        client_id: u16,
        trigger_type: TriggerType,
        cycle_offset: u8,
    ) -> Result<Self, String> {
        // validate client_id.
        if client_id > CLIENT_ID_MAX {
            return Err(format!(
                "[ClientConfig {}] Client ID {} is too large",
                client_id, client_id
            ));
        }
        // validate trigger_type.
        match &trigger_type {
            TriggerType::Cycle(cycle) => {
                if *cycle == 0 {
                    return Err(format!(
                        "[ClientConfig {}] Cycle must not be zero",
                        client_id
                    ));
                }
            }
            TriggerType::Depends { clients } => {
                if clients.is_empty() {
                    return Err(format!(
                        "[ClientConfig {}] Clients must not be empty",
                        client_id
                    ));
                }
                for client in clients {
                    if *client > CLIENT_ID_MAX {
                        return Err(format!(
                            "[ClientConfig {}] Depends {} is too large",
                            client_id, client
                        ));
                    }
                }
            }
        }
        // validate cycle_offset.
        match &trigger_type {
            TriggerType::Cycle(cycle) => {
                if cycle_offset >= *cycle {
                    return Err(format!(
                        "[ClientConfig {}] CycleOffset must be less than trigger cycle",
                        client_id
                    ));
                }
            }
            TriggerType::Depends { .. } => {
                if cycle_offset != 0 {
                    return Err(format!(
                        "[ClientConfig {}] CycleOffset must be zero when trigger type is Depends",
                        client_id
                    ));
                }
            }
        }
        // pass.
        Ok(Self {
            client_id,
            trigger_type,
            cycle_offset,
        })
    }

    // -----
    // private methods.
}

pub struct SchedulerConfig {
    pub server_config: ServerConfig,
    pub client_configs: Vec<ClientConfig>,
}
