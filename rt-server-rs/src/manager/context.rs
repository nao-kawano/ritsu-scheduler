// Copyright 2026 Naoyuki Kawano
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// =============================================================================
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
    pub fn set_client_state(&mut self, state: ClientState, cycle: i64) -> bool {
        info!(
            "<STAT> CYC:{:012} CID:{:03} [Conn] {:?} -> {:?}",
            cycle, self.config.client_id, self.state, state
        );
        self.state = state;
        return true; /* always ok */
    }
}

/* -------------------------------------------------------------------------- */

pub struct ServerStats {
    pub interval_cycle: u32,
    pub created_at: time::Instant,
    pub running_start_at: Option<time::Instant>,
    pub exiting_start_at: Option<time::Instant>,
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
    pub running_cycle: i64, // -1..CYCLE_MAX
    pub sched: Scheduler,
    pub graph_start: Vec<u16>, // shortcut for cycle start.
    // for stats.
    pub stats: ServerStats,
}

impl ManagerContext {
    pub const CYCLE_MAX: i64 = 999_999_999_999; // must be odd value for wrap-around.

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
            running_cycle: -1,
            sched,
            graph_start,
            stats: ServerStats {
                interval_cycle: stats_interval_cycle,
                created_at: time::Instant::now(),
                running_start_at: None,
                exiting_start_at: None,
            },
        }
    }

    pub fn set_state(&mut self, state: ManagerState) -> bool {
        info!(
            "<STAT> CYC:{:012} [Manager] {:?} -> {:?}",
            self.running_cycle, self.state, state
        );
        if self.state != state {
            self.state = state;
            self.state_changed = true;
        }
        return true; /* always ok */
    }

    pub fn calc_elapsed_times(&self, now: time::Instant) -> (u64, u64, u64, u64) {
        let exit_start = self.stats.exiting_start_at.unwrap_or(now);
        let run_start = self.stats.running_start_at.unwrap_or(exit_start);

        let total = now
            .saturating_duration_since(self.stats.created_at)
            .as_millis() as u64;
        let starting = run_start
            .saturating_duration_since(self.stats.created_at)
            .as_millis() as u64;
        let running = exit_start.saturating_duration_since(run_start).as_millis() as u64;
        let exiting = now.saturating_duration_since(exit_start).as_millis() as u64;

        (total, starting, running, exiting)
    }

    pub fn dump_stats(&self) {
        if self.stats.interval_cycle == 0 {
            return;
        }

        let now = time::Instant::now();
        let (total, starting, running, exiting) = self.calc_elapsed_times(now);

        info!(
            "[STATS] Server Elapsed: {}ms (S:{}, R:{}, E:{}), Cycles: {}",
            total,
            starting,
            running,
            exiting,
            self.running_cycle.max(0)
        );

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
