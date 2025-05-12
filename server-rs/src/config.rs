//!
//! Configuration for scheduler.
//!

pub struct ServerConfig {
    pub port: u16,
    pub cycle_time: u16,
}

pub enum TriggerType {
    Cycle(u8),
    Depends {
        clients: Vec<u16>, /* len() must be > 0 */
    },
}

pub struct ClientConfig {
    pub client_id: u16,
    pub trigger_type: TriggerType,
    pub cycle_offset: u32,
}

pub struct SchedulerConfig {
    pub server_config: ServerConfig,
    pub client_configs: Vec<ClientConfig>,
}
