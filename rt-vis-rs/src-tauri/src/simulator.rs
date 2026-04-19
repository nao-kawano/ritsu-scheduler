//!
//! Simulation Engine.
//!

use serde::{Deserialize, Serialize};

use rt_config::SchedulerConfig;
use rt_core::{ProcessEntry, ProcessStateChange, Scheduler};

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::u32;

#[cfg(test)]
#[path = "simulator_test.rs"]
mod simulator_test;

/* -------------------------------------------------------------------------- */

const MAX_SIMULATION_LOOPS: u32 = 100_000;
const MIN_DURATION_MS: u32 = 5;

/* -------------------------------------------------------------------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedExecution {
    pub instance_id: u32,
    pub cid: u16,
    pub cycle: u32,
    pub cycle_offset_ms: u32,
    pub start_ms: u32,
    pub duration_ms: u32,
    pub depends_instance_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedMetricPoint {
    pub time_ms: u32,
    pub running_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulationResult {
    pub executions: Vec<PlannedExecution>,
    pub metrics: Vec<PlannedMetricPoint>,
}

/* -------------------------------------------------------------------------- */

#[derive(Debug, Clone, Eq, PartialEq)]
enum EventKind {
    ServerCycle(u32),      // cycle_index (starts from 0)
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

    fn process_changes(
        &mut self,
        time_ms: u32,
        changes: Vec<ProcessStateChange>,
        manager_cycle: u32,
        config: &SchedulerConfig,
    ) {
        use rt_core::ProcessState::*;
        let cycle_time = config.server_config.cycle_time_ms as u32;

        for change in changes {
            let before_running = matches!(change.before, Running | Overrun);
            let after_running = matches!(change.after, Running | Overrun);
            // TODO: detect overrun and skip. (invalid configuration)

            match (before_running, after_running) {
                (false, true) => {
                    // == Process Started

                    // Find client_config for this process.
                    let config_client = config
                        .client_configs
                        .iter()
                        .find(|c| c.client_id == change.cid)
                        .unwrap();

                    // Calculate the duration.
                    let duration_ms = config_client.expected_duration_ms.max(MIN_DURATION_MS);

                    // Assign a unique instance_id and add the process to the running list.
                    let i_id = self.instance_counter;
                    self.instance_counter += 1;

                    // Push execution and schedule the end of this execution.
                    self.current_running.insert(change.cid, i_id);
                    self.executions.push(PlannedExecution {
                        instance_id: i_id,
                        cid: change.cid,
                        cycle: manager_cycle,
                        cycle_offset_ms: time_ms % cycle_time,
                        start_ms: time_ms,
                        duration_ms,
                        depends_instance_ids: config_client
                            .depends
                            .iter()
                            .filter_map(|&d| self.last_instance_ids.get(&d).copied())
                            .collect(),
                    });
                    self.events.push(SimulationEvent {
                        time_ms: time_ms + duration_ms,
                        kind: EventKind::ProcessDone(change.cid, i_id),
                    });
                }
                (true, false) => {
                    // == Process Finished
                    if let Some(i_id) = self.current_running.remove(&change.cid) {
                        self.last_instance_ids.insert(change.cid, i_id);
                    }
                }
                _ => {}
            }
        }

        // Record the count of currently running processes.
        self.record_metric(time_ms);
    }
}

/* -------------------------------------------------------------------------- */

struct CycleTrigger {
    cid: u16,
    cycle: u32,
    cycle_offset: u32,
}

#[tauri::command]
pub fn simulate_plan(config: SchedulerConfig) -> Result<SimulationResult, String> {
    // If there are no processes, return empty results immediately.
    if config.client_configs.is_empty() {
        return Ok(SimulationResult {
            executions: Vec::new(),
            metrics: Vec::new(),
        });
    }

    // Build entries for scheduler and triggers.
    let mut entries = HashMap::new();
    let mut max_cycle: u32 = 1;
    let mut triggers = Vec::new();
    let rules = config.get_client_rules().map_err(|errs| errs.join(", "))?;
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
                cycle: client.cycle as u32,
                cycle_offset: client.cycle_offset as u32,
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
    let mut manager_cycle: u32 = 0;
    let max_manager_cycle = max_cycle + 1;
    let mut loop_count = 0;

    while let Some(event) = state.events.pop() {
        // Prevent infinite loop.
        loop_count += 1;
        if loop_count > MAX_SIMULATION_LOOPS {
            return Err("Simulation reached loop limit".to_string());
        }
        // Process events.
        match event.kind {
            EventKind::ServerCycle(cycle) => {
                // Update the manager cycle and check if the simulation limit is reached.
                manager_cycle = cycle;
                if manager_cycle >= max_manager_cycle {
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
                            state.process_changes(event.time_ms, changes, manager_cycle, &config);
                        }
                    }
                }
            }
            EventKind::ProcessDone(cid, instance_id) => {
                // Ensure this is the currently running instance of this process.
                if state.current_running.get(&cid) == Some(&instance_id) {
                    if let Ok(changes) = scheduler.on_done(cid) {
                        state.process_changes(event.time_ms, changes, manager_cycle, &config);
                    }
                    // Set to ready for next cycle.
                    let _ = scheduler.on_ready(cid);
                }
            }
        }
    }

    Ok(SimulationResult {
        executions: state.executions,
        metrics: state.metrics,
    })
}
