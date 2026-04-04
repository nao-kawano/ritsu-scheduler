//!
//! Manager context.
//!

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use rt_config::{ClientConfig, ClientRule};
use rt_core::{ProcessEntry, Scheduler};

use super::ManagerState;

use std::collections::HashMap;
use std::time;

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

#[derive(Debug, Clone)]
pub struct ClientStats {
    pub trigger_count: u32,
    pub success_count: u32,
    pub skip_count: u32,
    pub late_count: u32,
    pub overrun_count: u32,
    pub time_min: u32,         // ms
    pub time_max: u32,         // ms
    pub time_sum: u64,         // ms
    pub overrun_time_min: u32, // ms
    pub overrun_time_max: u32, // ms
    pub overrun_time_sum: u64, // ms
}

impl Default for ClientStats {
    fn default() -> Self {
        Self {
            trigger_count: 0,
            success_count: 0,
            skip_count: 0,
            late_count: 0,
            overrun_count: 0,
            time_min: u32::MAX,
            time_max: 0,
            time_sum: 0,
            overrun_time_min: u32::MAX,
            overrun_time_max: 0,
            overrun_time_sum: 0,
        }
    }
}

pub struct ClientInfo {
    pub config: ClientConfig,
    pub state: ClientState,
    pub last_mid: u8,
    pub stats: ClientStats,
    pub running_start_at: Option<time::Instant>,
}

impl ClientInfo {
    /// Constructor.
    pub fn new(config: ClientConfig) -> Self {
        ClientInfo {
            config,
            state: ClientState::None,
            last_mid: 255,
            stats: ClientStats::default(),
            running_start_at: None,
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

pub struct ServerStats {
    pub interval_cycle: u32,
    pub start_at: Option<time::Instant>,
    pub start_cycle: u64,
    pub last_cycle: u64,
}

pub struct ManagerContext {
    // manager.
    pub state: ManagerState,
    pub state_changed: bool,
    pub exit_reason: Option<Vec<(String, String)>>,
    // for connection.
    pub clients: HashMap<u16, ClientInfo>,
    pub num_active_clients: usize,
    // for execution.
    pub cycle_current: u32, // 0..CYCLE_MAX
    pub sched: Scheduler,
    pub graph_start: Vec<u16>, // shortcut for cycle start.
    // for stats.
    pub stats: ServerStats,
}

impl ManagerContext {
    pub const CYCLE_MAX: u32 = 9999; // must be odd value for wrap-around.

    pub fn new(
        configs: Vec<ClientConfig>,
        rules: HashMap<u16, ClientRule>,
        stats_interval_cycle: u32,
    ) -> Self {
        // At least one client must be provided.
        if configs.len() < 1 {
            panic!("Client config is empty");
        }
        // create clients for connection management.
        let clients: HashMap<u16, ClientInfo> = configs
            .into_iter()
            .map(|config| (config.client_id, ClientInfo::new(config)))
            .collect();
        // Create process entries for execution management.
        let mut graph_start: Vec<u16> = Vec::with_capacity(clients.len());
        let mut entries: HashMap<u16, ProcessEntry> = HashMap::with_capacity(clients.len());
        for client in clients.values() {
            // Retrieve pre-calculated execution rules for this client.
            let rule = rules
                .get(&client.config.client_id)
                .unwrap_or_else(|| panic!("Rule for CID:{:03} not found", client.config.client_id));

            // Create entry with derived floating status.
            entries.insert(
                client.config.client_id,
                ProcessEntry::new(
                    client.config.client_id,
                    &client.config.depends,
                    rule.is_floating,
                ),
            );

            // If the process is not floating, it is a starting point of the execution graph.
            if !rule.is_floating {
                graph_start.push(client.config.client_id);
            }
        }
        let sched: Scheduler = Scheduler::new(entries);
        // create context.
        ManagerContext {
            state: ManagerState::Starting,
            state_changed: false,
            exit_reason: None,
            clients,
            num_active_clients: 0,
            cycle_current: 0,
            sched,
            graph_start,
            stats: ServerStats {
                interval_cycle: stats_interval_cycle,
                start_at: None,
                start_cycle: 0,
                last_cycle: 0,
            },
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

    pub fn dump_stats(&self, current_global_cycle: u64) {
        if self.stats.interval_cycle == 0 {
            return;
        }

        if let Some(start_at) = self.stats.start_at {
            let elapsed = start_at.elapsed().as_millis();
            let cycles = current_global_cycle.saturating_sub(self.stats.start_cycle);
            info!(
                "[STATS] Server: Elapsed Time: {} ms, Cycles: {}",
                elapsed, cycles
            );
        } else {
            info!("[STATS] Server: Elapsed Time: 0 ms, Cycles: 0");
        }

        let mut cids: Vec<&u16> = self.clients.keys().collect();
        cids.sort();
        for cid in cids {
            let client = self.clients.get(cid).unwrap();
            let stats = &client.stats;
            let (avg, min, max) = if stats.success_count > 0 {
                (
                    stats.time_sum / (stats.success_count as u64),
                    stats.time_min,
                    stats.time_max,
                )
            } else {
                (0, 0, 0)
            };
            let (ov_avg, ov_min, ov_max) = if stats.overrun_count > 0 {
                (
                    stats.overrun_time_sum / (stats.overrun_count as u64),
                    stats.overrun_time_min,
                    stats.overrun_time_max,
                )
            } else {
                (0, 0, 0)
            };
            info!(
                "[STATS] CID:{:03}, Trg: {} (Ok: {}, Ov: {}, Skip: {}, Late: {}), Time[ms]: Avg {} ({}-{}), OvTime[ms]: Avg {} ({}-{})",
                cid,
                stats.trigger_count,
                stats.success_count,
                stats.overrun_count,
                stats.skip_count,
                stats.late_count,
                avg,
                min,
                max,
                ov_avg,
                ov_min,
                ov_max
            );
        }
    }

    // -----
    // private methods.
}
