//!
//! Configuration for scheduler.
//!

use rt_message::CLIENT_ID_MAX;

use std::collections::{HashMap, HashSet};

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
    /// Basic validation of the client configuration.
    pub fn validate(&self) -> Vec<String> {
        let mut errs = Vec::new();
        // Validate client_id.
        if self.client_id > CLIENT_ID_MAX {
            errs.push(format!("Client ID {:03} is too large", self.client_id));
        }
        // Validate cycle.
        if self.cycle == 0 {
            errs.push("Cycle must not be zero".to_string());
        }
        // Validate cycle_offset.
        if self.cycle_offset >= self.cycle {
            errs.push(format!(
                "CycleOffset ({}) must be less than trigger cycle ({})",
                self.cycle_offset, self.cycle
            ));
        }
        // Validate depends.
        let mut seen_depends = HashSet::new();
        for depend in &self.depends {
            if *depend > CLIENT_ID_MAX {
                errs.push(format!("Depends CID:{:03} is too large", depend));
            }
            if *depend == self.client_id {
                errs.push(format!(
                    "Self-dependency is not allowed (CID:{:03})",
                    depend
                ));
            }
            if !seen_depends.insert(*depend) {
                errs.push(format!("Duplicate dependency CID:{:03}", depend));
            }
        }
        errs
    }

    /// Create a new ClientConfig with basic validation.
    pub fn new(
        client_id: u16,
        cycle: u8,
        cycle_offset: u8,
        depends: Vec<u16>,
        expected_duration_ms: u32,
    ) -> Result<Self, String> {
        let config = Self {
            client_id,
            cycle,
            cycle_offset,
            depends,
            expected_duration_ms,
        };
        let errs = config.validate();
        if errs.is_empty() {
            Ok(config)
        } else {
            Err(format!(
                "[ClientConfig CID:{:03}] {}",
                client_id,
                errs.join(", ")
            ))
        }
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
    /// Validate all client configurations.
    /// Returns Ok(()) if all configurations are valid, or a map of error messages per Client ID.
    pub fn validate(&self) -> Result<(), HashMap<u16, Vec<String>>> {
        let mut errors: HashMap<u16, Vec<String>> = HashMap::new();

        // Check for duplicate Client IDs and create a lookup map.
        let mut configs: HashMap<u16, &ClientConfig> =
            HashMap::with_capacity(self.client_configs.len());
        for client in &self.client_configs {
            if configs.insert(client.client_id, client).is_some() {
                errors
                    .entry(client.client_id)
                    .or_default()
                    .push(format!("Duplicate client ID {:03}", client.client_id));
            }
        }

        // Validate each client's dependencies and individual settings.
        for client in &self.client_configs {
            let mut client_errors = client.validate();

            for depend_id in &client.depends {
                match configs.get(depend_id) {
                    Some(dep_config) => {
                        // All dependent processes must have the same cycle.
                        if dep_config.cycle != client.cycle {
                            client_errors.push(format!(
                                "Dependent process CID:{:03} has different cycle ({} vs {})",
                                dep_config.client_id, dep_config.cycle, client.cycle
                            ));
                        }
                        // Dependent processes must not have a future cycle offset.
                        if dep_config.cycle_offset > client.cycle_offset {
                            client_errors.push(format!(
                                "Dependent process CID:{:03} has larger cycle_offset ({} > {})",
                                dep_config.client_id, dep_config.cycle_offset, client.cycle_offset
                            ));
                        }
                    }
                    None => {
                        client_errors.push(format!(
                            "Dependent process CID:{:03} does not exist",
                            depend_id
                        ));
                    }
                }
            }

            if !client_errors.is_empty() {
                errors.insert(client.client_id, client_errors);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Derive execution rules for all client configurations.
    /// This should be called after successful validation.
    pub fn get_client_rules(&self) -> HashMap<u16, ClientRule> {
        let mut rules = HashMap::with_capacity(self.client_configs.len());

        // Create a lookup map.
        let configs: HashMap<u16, &ClientConfig> = self
            .client_configs
            .iter()
            .map(|c| (c.client_id, c))
            .collect();

        for client in &self.client_configs {
            let mut is_floating = false;

            for depend_id in &client.depends {
                if let Some(dep_config) = configs.get(depend_id) {
                    // If the dependent process has the same cycle and cycle offset,
                    // this process starts immediately after the dependent process completes.
                    if dep_config.cycle_offset == client.cycle_offset {
                        is_floating = true;
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

        rules
    }
}
