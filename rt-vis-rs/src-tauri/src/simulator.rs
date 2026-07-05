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
//! Simulation Engine.
//!

use serde::{Deserialize, Serialize};

use rt_config::{ClientConfig, SchedulerConfig};
use rt_core::{ProcessEntry, ProcessState, ProcessStateChange, Scheduler};

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::u32;

#[cfg(test)]
#[path = "simulator_test.rs"]
mod simulator_test;

/* -------------------------------------------------------------------------- */

const MAX_SIMULATION_LOOPS: u32 = 100_000;
const MIN_DURATION_MS: u32 = 1;

/* -------------------------------------------------------------------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Normal,
    Overrun,
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PlannedExecution {
    pub instance_id: u32,
    pub cid: u16,
    pub cycle: i64,
    pub cycle_offset_ms: u32,
    pub start_ms: u32,
    pub duration_ms: u32,
    pub depends_instance_ids: Vec<u32>,
    pub status: ExecutionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PlannedMetricPoint {
    pub time_ms: u32,
    pub running_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SimulationResult {
    pub executions: Vec<PlannedExecution>,
    pub metrics: Vec<PlannedMetricPoint>,
    pub config_errors: HashMap<u16, Vec<String>>,
}

impl SimulationResult {
    fn new(
        executions: Vec<PlannedExecution>,
        metrics: Vec<PlannedMetricPoint>,
        config_errors: HashMap<u16, Vec<String>>,
    ) -> Self {
        Self {
            executions,
            metrics,
            config_errors,
        }
    }

    /// Creates a result with no executions or errors.
    fn empty() -> Self {
        Self::new(Vec::new(), Vec::new(), HashMap::new())
    }

    /// Creates a result representing static configuration errors.
    fn error(config_errors: HashMap<u16, Vec<String>>) -> Self {
        Self::new(Vec::new(), Vec::new(), config_errors)
    }
}

/* -------------------------------------------------------------------------- */

#[derive(Debug, Clone, Eq, PartialEq)]
enum EventKind {
    ServerCycle(i64),      // cycle_index (starts from 0)
    ProcessDone(u16, u32), // cid, instance_id
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct SimulationEvent {
    time_ms: u32,
    kind: EventKind,
}

impl Ord for SimulationEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order for min-heap based on time
        match other.time_ms.cmp(&self.time_ms) {
            Ordering::Equal => {
                // If time is equal, prioritize ProcessDone over ServerCycle
                match (&self.kind, &other.kind) {
                    (EventKind::ProcessDone(_, _), EventKind::ServerCycle(_)) => Ordering::Greater,
                    (EventKind::ServerCycle(_), EventKind::ProcessDone(_, _)) => Ordering::Less,
                    _ => Ordering::Equal,
                }
            }
            ord => ord,
        }
    }
}

impl PartialOrd for SimulationEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct SimulationState {
    instance_counter: u32,
    executions: Vec<PlannedExecution>,
    metrics: Vec<PlannedMetricPoint>,
    current_running: HashMap<u16, u32>,
    last_instance_ids: HashMap<u16, u32>,
    events: BinaryHeap<SimulationEvent>,
}

impl SimulationState {
    fn new() -> Self {
        let mut events = BinaryHeap::new();
        events.push(SimulationEvent {
            time_ms: 0,
            kind: EventKind::ServerCycle(0),
        });
        Self {
            instance_counter: 0,
            executions: Vec::new(),
            metrics: Vec::new(),
            current_running: HashMap::new(),
            last_instance_ids: HashMap::new(),
            events,
        }
    }

    fn record_metric(&mut self, time_ms: u32) {
        let running_count = self.current_running.len() as u32;
        if let Some(last) = self.metrics.last_mut() {
            if last.time_ms == time_ms {
                last.running_count = running_count;
                return;
            }
        }
        self.metrics.push(PlannedMetricPoint {
            time_ms,
            running_count,
        });
    }

    fn process_change(
        &mut self,
        time_ms: u32,
        change: &ProcessStateChange,
        manager_cycle_time: u32,
        manager_cycle: i64,
        config_client: &ClientConfig,
    ) {
        match change.after {
            ProcessState::Running => {
                // == Process Started Normally
                let duration_ms = config_client.expected_duration_ms.max(MIN_DURATION_MS);
                let i_id = self.instance_counter;
                self.instance_counter += 1;

                self.current_running.insert(change.cid, i_id);
                self.executions.push(PlannedExecution {
                    instance_id: i_id,
                    cid: change.cid,
                    cycle: manager_cycle,
                    cycle_offset_ms: time_ms % manager_cycle_time,
                    start_ms: time_ms,
                    duration_ms,
                    depends_instance_ids: config_client
                        .depends
                        .iter()
                        .filter_map(|&d| self.last_instance_ids.get(&d).copied())
                        .collect(),
                    status: ExecutionStatus::Normal,
                });
                self.events.push(SimulationEvent {
                    time_ms: time_ms + duration_ms,
                    kind: EventKind::ProcessDone(change.cid, i_id),
                });
            }
            ProcessState::Overrun => {
                // == Overrun Detected (Running -> Overrun)
                if let Some(&i_id) = self.current_running.get(&change.cid) {
                    if let Some(exec) = self.executions.iter_mut().find(|e| e.instance_id == i_id) {
                        exec.status = ExecutionStatus::Overrun;
                    }
                }
            }
            ProcessState::Skip => {
                // == Skip Detected (Dependency unmet)
                let duration_ms = config_client.expected_duration_ms.max(MIN_DURATION_MS);
                let i_id = self.instance_counter;
                self.instance_counter += 1;

                // Insert dummy entry without Done event.
                self.executions.push(PlannedExecution {
                    instance_id: i_id,
                    cid: change.cid,
                    cycle: manager_cycle,
                    cycle_offset_ms: time_ms % manager_cycle_time,
                    start_ms: time_ms,
                    duration_ms,
                    depends_instance_ids: vec![],
                    status: ExecutionStatus::Skip,
                });
            }
            ProcessState::Late => {
                if change.before == ProcessState::Overrun {
                    // == Overrun Process Finished (Overrun -> Late)
                    if let Some(i_id) = self.current_running.remove(&change.cid) {
                        self.last_instance_ids.insert(change.cid, i_id);
                    }
                }
            }
            ProcessState::Idle => {
                if change.before == ProcessState::Running {
                    // == Process Finished Normally (Running -> Idle)
                    if let Some(i_id) = self.current_running.remove(&change.cid) {
                        self.last_instance_ids.insert(change.cid, i_id);
                    }
                }
            }
            ProcessState::Ready => {}
        }
    }

    fn process_changes(
        &mut self,
        time_ms: u32,
        changes: &Vec<ProcessStateChange>,
        manager_cycle: i64,
        config: &SchedulerConfig,
    ) {
        let manager_cycle_time = config.server_config.cycle_time_ms as u32;
        // Process state changes.
        for change in changes {
            let config_client = config
                .client_configs
                .iter()
                .find(|c| c.client_id == change.cid)
                .unwrap();
            self.process_change(
                time_ms,
                change,
                manager_cycle_time,
                manager_cycle,
                config_client,
            );
        }
        // Record the count of currently running processes.
        self.record_metric(time_ms);
    }
}

/* -------------------------------------------------------------------------- */

struct CycleTrigger {
    cid: u16,
    cycle: i64,
    cycle_offset: i64,
}

#[tauri::command]
pub fn simulate_plan(config: SchedulerConfig) -> Result<SimulationResult, String> {
    // If there are no processes, return empty results immediately.
    if config.client_configs.is_empty() {
        return Ok(SimulationResult::empty());
    }

    // Static validation: Check rules and collect errors.
    if let Err(errs) = config.validate() {
        log::warn!(
            "Static validation failed: {} processes have errors",
            errs.len()
        );
        return Ok(SimulationResult::error(errs));
    }

    log::info!("Starting plan simulation...");
    let start_time = std::time::Instant::now();

    // Derive execution rules.
    let rules = config.get_client_rules();

    // Build entries for scheduler and triggers.
    let mut entries = HashMap::new();
    let mut triggers = Vec::new();
    let mut max_cycle: u32 = 1;

    for client in &config.client_configs {
        let rule = rules.get(&client.client_id).unwrap();
        entries.insert(
            client.client_id,
            ProcessEntry::new(client.client_id, &client.depends, rule.is_floating),
        );
        if client.cycle as u32 > max_cycle {
            max_cycle = client.cycle as u32;
        }
        if !rule.is_floating {
            triggers.push(CycleTrigger {
                cid: client.client_id,
                cycle: client.cycle as i64,
                cycle_offset: client.cycle_offset as i64,
            });
        }
    }

    // Setup scheduler.
    let mut scheduler = Scheduler::new(entries);
    for client in &config.client_configs {
        let _ = scheduler.on_ready(client.client_id);
    }

    // Setup scheduling data.
    let mut state = SimulationState::new();
    let mut manager_cycle: i64 = 0;
    // Simulate for 2x max_cycle to cover offset scenarios.
    // NOTE: Keep in sync with frontend: useCreateModeLayout.ts -> totalCycles
    let max_manager_cycle = (max_cycle * 2) as i64;
    let mut loop_count = 0;

    while let Some(event) = state.events.pop() {
        // Prevent infinite loop.
        loop_count += 1;
        if loop_count > MAX_SIMULATION_LOOPS {
            log::warn!(
                "Simulation reached loop limit in {:?}",
                start_time.elapsed()
            );
            return Err("Simulation reached loop limit".to_string());
        }
        // Process events.
        match event.kind {
            EventKind::ServerCycle(cycle) => {
                // Update the manager cycle and check if the simulation limit is reached.
                manager_cycle = cycle;
                if manager_cycle >= max_manager_cycle {
                    state.record_metric(event.time_ms);
                    break;
                }
                // Enqueue next cycle.
                state.events.push(SimulationEvent {
                    time_ms: event.time_ms + config.server_config.cycle_time_ms as u32,
                    kind: EventKind::ServerCycle(cycle + 1),
                });
                // Trigger processes for this cycle.
                for t in &triggers {
                    if (manager_cycle % t.cycle) == t.cycle_offset {
                        if let Ok(changes) = scheduler.on_start(t.cid) {
                            // Records process state.
                            state.process_changes(event.time_ms, &changes, manager_cycle, &config);
                            // Set Skipped process to Ready for next cycle.
                            for change in &changes {
                                match change.after {
                                    ProcessState::Skip => {
                                        let _ = scheduler.on_ready(change.cid); // Skip -> Ready.
                                    }
                                    ProcessState::Late => {
                                        let _ = scheduler.on_ready(change.cid); // Late -> Idle.
                                        let _ = scheduler.on_ready(change.cid); // Idle -> Ready.
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            EventKind::ProcessDone(cid, instance_id) => {
                // Ensure this is the currently running instance of this process.
                if state.current_running.get(&cid) == Some(&instance_id) {
                    if let Ok(changes) = scheduler.on_done(cid) {
                        // Records process state.
                        state.process_changes(event.time_ms, &changes, manager_cycle, &config);
                        // Set to ready for next cycle.
                        if let Some(change) = changes.iter().find(|c| c.cid == cid) {
                            match change.after {
                                ProcessState::Idle => {
                                    let _ = scheduler.on_ready(cid); // Idle -> Ready.
                                }
                                ProcessState::Late => {
                                    let _ = scheduler.on_ready(cid); // Late -> Idle.
                                    let _ = scheduler.on_ready(cid); // Idle -> Ready.
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    log::info!("Finished plan simulation in {:?}", start_time.elapsed());
    Ok(SimulationResult::new(
        state.executions,
        state.metrics,
        HashMap::new(),
    ))
}
