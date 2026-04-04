//!
//! Configuration for scheduler.
//!

use rt_message::CLIENT_ID_MAX;

use std::collections::HashMap;

#[cfg(test)]
#[path = "config_test.rs"]
mod config_test;

/* -------------------------------------------------------------------------- */

/// Configuration for the server.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ServerConfig {
    pub port: u16,
    pub cycle_time_ms: u32,
    #[serde(default)]
    pub stats_interval_cycle: u32,
}

/// Configuration for a client process.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ClientConfig {
    pub client_id: u16,
    pub cycle: u8,
    pub cycle_offset: u8,
    pub depends: Vec<u16>,
    #[serde(default)]
    pub expected_duration_ms: u32,
}

impl ClientConfig {
    /// Create a new ClientConfig with basic validation.
    pub fn new(
        client_id: u16,
        cycle: u8,
        cycle_offset: u8,
        depends: Vec<u16>,
        expected_duration_ms: u32,
    ) -> Result<Self, String> {
        // Validate client_id.
        if client_id > CLIENT_ID_MAX {
            return Err(format!(
                "[ClientConfig {:03}] Client ID {:03} is too large",
                client_id, client_id
            ));
        }
        // Validate cycle.
        if cycle == 0 {
            return Err(format!(
                "[ClientConfig {:03}] Cycle must not be zero",
                client_id
            ));
        }
        // Validate cycle_offset.
        if cycle_offset >= cycle {
            return Err(format!(
                "[ClientConfig {:03}] CycleOffset must be less than trigger cycle",
                client_id
            ));
        }
        // Validate depends.
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
            expected_duration_ms,
        })
    }
}

/// Derived execution rules for a client process.
#[derive(Clone, Debug)]
pub struct ClientRule {
    pub client_id: u16,
    pub is_floating: bool,
}

/// Root configuration for the scheduler.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct SchedulerConfig {
    pub server_config: ServerConfig,
    pub client_configs: Vec<ClientConfig>,
}

impl SchedulerConfig {
    /// Validate all client configurations and derive execution rules.
    /// Returns a map of rules indexed by Client ID, or a list of error messages.
    pub fn get_client_rules(&self) -> Result<HashMap<u16, ClientRule>, Vec<String>> {
        let mut errors = Vec::new();
        let mut rules = HashMap::with_capacity(self.client_configs.len());

        // Create a lookup map for faster access.
        let configs: HashMap<u16, &ClientConfig> = self
            .client_configs
            .iter()
            .map(|c| (c.client_id, c))
            .collect();

        for client in &self.client_configs {
            let mut is_floating = false;
            let prefix = format!("[ClientConfig CID:{:03}]", client.client_id);

            for depend_id in &client.depends {
                match configs.get(depend_id) {
                    Some(dep_config) => {
                        // All dependent processes must have the same cycle.
                        if dep_config.cycle != client.cycle {
                            errors.push(format!(
                                "{} Dependent process CID:{:03} has different cycle ({} vs {})",
                                prefix, dep_config.client_id, dep_config.cycle, client.cycle
                            ));
                        }
                        // Dependent processes must not have a future cycle offset.
                        if dep_config.cycle_offset > client.cycle_offset {
                            errors.push(format!(
                                "{} Dependent process CID:{:03} has larger cycle_offset ({} > {})",
                                prefix,
                                dep_config.client_id,
                                dep_config.cycle_offset,
                                client.cycle_offset
                            ));
                        }
                        // If the dependent process has the same cycle and cycle offset,
                        // this process starts immediately after the dependent process completes.
                        if dep_config.cycle_offset == client.cycle_offset {
                            is_floating = true;
                        }
                    }
                    None => {
                        errors.push(format!(
                            "{} Dependent process CID:{:03} does not exist",
                            prefix, depend_id
                        ));
                    }
                }
            }
            rules.insert(
                client.client_id,
                ClientRule {
                    client_id: client.client_id,
                    is_floating,
                },
            );
        }

        if errors.is_empty() {
            Ok(rules)
        } else {
            Err(errors)
        }
    }
}
