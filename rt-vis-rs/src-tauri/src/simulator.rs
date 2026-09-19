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

use rt_config::{ClientConfig, SchedulerConfig};
use rt_core::{ProcessEntry, ProcessState, ProcessStateChange, Scheduler};

use crate::types::{ExecutionStatus, PlannedExecution, PlannedMetricPoint, SimulationResult};

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[cfg(test)]
#[path = "simulator_test.rs"]
mod simulator_test;

/* -------------------------------------------------------------------------- */

const MAX_SIMULATION_LOOPS: u32 = 100_000;
const MIN_DURATION_MS: u32 = 1;

/* -------------------------------------------------------------------------- */

#[derive(Debug, Clone, Eq, PartialEq)]
enum EventKind {
    ServerCycle(i64),      // cycle_index (starts from 0).
    ProcessDone(u16, u32), // cid, instance_id.
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct SimulationEvent {
    time_ms: u32,
    kind: EventKind,
}

impl Ord for SimulationEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order for min-heap based on time.
        match other.time_ms.cmp(&self.time_ms) {
            Ordering::Equal => {
                // If time is equal, prioritize ProcessDone over ServerCycle.
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

/* -------------------------------------------------------------------------- */

/// Context holding static simulation configurations.
struct SimulationContext<'a> {
    cycle_time_ms: u32,
    client_configs: HashMap<u16, &'a ClientConfig>,
    is_floating: HashMap<u16, bool>,
}

/// Tracks running instances and collects simulation outputs.
struct SimulationRecorder {
    instance_counter: u32,
    current_running: HashMap<u16, u32>,   // cid -> instance_id.
    last_instance_ids: HashMap<u16, u32>, // cid -> instance_id.
    instance_anchors: HashMap<u32, i64>,  // instance_id -> anchor_cycle.
    executions: Vec<PlannedExecution>,
    metrics: Vec<PlannedMetricPoint>,
}

impl SimulationRecorder {
    fn new() -> Self {
        Self {
            instance_counter: 0,
            current_running: HashMap::new(),
            last_instance_ids: HashMap::new(),
            instance_anchors: HashMap::new(),
            executions: Vec::new(),
            metrics: Vec::new(),
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

    fn record_change(
        &mut self,
        time_ms: u32,
        change: &ProcessStateChange,
        current_cycle: i64,
        ctx: &SimulationContext,
        events: &mut BinaryHeap<SimulationEvent>,
    ) {
        let config_client = ctx.client_configs.get(&change.cid).unwrap();
        let is_floating = ctx.is_floating.get(&change.cid).copied().unwrap_or(false);

        match change.after {
            ProcessState::Running => {
                // == Process Started Normally
                let duration_ms = config_client.expected_duration_ms.max(MIN_DURATION_MS);
                let i_id = self.instance_counter;
                self.instance_counter += 1;

                // Resolve dependent parent instance IDs.
                let depends_instance_ids: Vec<u32> = config_client
                    .depends
                    .iter()
                    .filter_map(|&d| self.last_instance_ids.get(&d).copied())
                    .collect();
                // Determine anchor cycle: inherit from parent if floating dependency, else current cycle.
                let anchor_cycle = if is_floating {
                    depends_instance_ids
                        .first()
                        .and_then(|&parent_id| self.instance_anchors.get(&parent_id).copied())
                        .unwrap_or(current_cycle)
                } else {
                    current_cycle
                };
                self.instance_anchors.insert(i_id, anchor_cycle);
                // Determine anchor offset: start from anchor cycle.
                let anchor_base_ms = (anchor_cycle * ctx.cycle_time_ms as i64) as u32;
                let anchor_offset_ms = time_ms.saturating_sub(anchor_base_ms);

                // Update current running processes.
                self.current_running.insert(change.cid, i_id);
                // Insert entry and enqueue Done event.
                self.executions.push(PlannedExecution {
                    instance_id: i_id,
                    cid: change.cid,
                    anchor_cycle,
                    anchor_offset_ms,
                    start_ms: time_ms,
                    duration_ms,
                    depends_instance_ids,
                    status: ExecutionStatus::Normal,
                });
                events.push(SimulationEvent {
                    time_ms: time_ms + duration_ms,
                    kind: EventKind::ProcessDone(change.cid, i_id),
                });
            }
            ProcessState::Overrun => {
                // == Overrun Detected (Running -> Overrun)
                // Update instance state to Overrun.
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
                let anchor_base_ms = (current_cycle * ctx.cycle_time_ms as i64) as u32;
                let anchor_offset_ms = time_ms.saturating_sub(anchor_base_ms);
                self.executions.push(PlannedExecution {
                    instance_id: i_id,
                    cid: change.cid,
                    anchor_cycle: current_cycle,
                    anchor_offset_ms,
                    start_ms: time_ms,
                    duration_ms,
                    depends_instance_ids: vec![],
                    status: ExecutionStatus::Skip,
                });
            }
            ProcessState::Late => {
                // NOTE: No need to handle `change.before == Idle` because Ready request is immediately sent in simulation.
                if change.before == ProcessState::Overrun {
                    // == Overrun Process Finished (Overrun -> Late)
                    // Update running processes and last instance_id.
                    if let Some(i_id) = self.current_running.remove(&change.cid) {
                        self.last_instance_ids.insert(change.cid, i_id);
                    }
                }
            }
            ProcessState::Idle => {
                // NOTE: No need to handle `change.before == Late` because already updated.
                if change.before == ProcessState::Running {
                    // == Process Finished Normally (Running -> Idle)
                    // Update running processes and last instance_id.
                    if let Some(i_id) = self.current_running.remove(&change.cid) {
                        self.last_instance_ids.insert(change.cid, i_id);
                    }
                }
            }
            ProcessState::Ready => {}
        }
    }

    fn record_changes(
        &mut self,
        time_ms: u32,
        changes: &[ProcessStateChange],
        current_cycle: i64,
        ctx: &SimulationContext,
        events: &mut BinaryHeap<SimulationEvent>,
    ) {
        // Record the state changes.
        for change in changes {
            self.record_change(time_ms, change, current_cycle, ctx, events);
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
    let mut client_configs_map = HashMap::new();
    let mut is_floating_map = HashMap::new();
    let mut entries = HashMap::new();
    let mut triggers = Vec::new();
    let mut max_cycle: u32 = 1;

    for client in &config.client_configs {
        let rule = rules.get(&client.client_id).unwrap();
        client_configs_map.insert(client.client_id, client);
        is_floating_map.insert(client.client_id, rule.is_floating);
        entries.insert(
            client.client_id,
            ProcessEntry::new(client.client_id, &client.depends, rule.is_floating),
        );
        if !rule.is_floating {
            triggers.push(CycleTrigger {
                cid: client.client_id,
                cycle: client.cycle as i64,
                cycle_offset: client.cycle_offset as i64,
            });
        }
        if client.cycle as u32 > max_cycle {
            max_cycle = client.cycle as u32;
        }
    }

    let ctx = SimulationContext {
        cycle_time_ms: config.server_config.cycle_time_ms as u32,
        client_configs: client_configs_map,
        is_floating: is_floating_map,
    };

    // Set up scheduler.
    let mut scheduler = Scheduler::new(entries);
    for client in &config.client_configs {
        let _ = scheduler.on_ready(client.client_id);
    }

    // Set up scheduling event loop.
    let mut events = BinaryHeap::new();
    events.push(SimulationEvent {
        time_ms: 0,
        kind: EventKind::ServerCycle(0),
    });
    let mut recorder = SimulationRecorder::new();
    let mut manager_cycle: i64 = 0;
    // Simulate for 2x max_cycle to cover offset scenarios.
    // NOTE: Keep in sync with frontend: useCreateModeLayout.ts -> totalCycles.
    let max_manager_cycle = (max_cycle * 2) as i64;
    let mut loop_count = 0;

    while let Some(event) = events.pop() {
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
                    recorder.record_metric(event.time_ms);
                    break;
                }
                // Enqueue next cycle.
                events.push(SimulationEvent {
                    time_ms: event.time_ms + ctx.cycle_time_ms,
                    kind: EventKind::ServerCycle(cycle + 1),
                });
                // Trigger processes for this cycle.
                for t in &triggers {
                    if (manager_cycle % t.cycle) == t.cycle_offset {
                        if let Ok(changes) = scheduler.on_start(t.cid) {
                            // Record process state.
                            recorder.record_changes(
                                event.time_ms,
                                &changes,
                                manager_cycle,
                                &ctx,
                                &mut events,
                            );
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
                if recorder.current_running.get(&cid) == Some(&instance_id) {
                    if let Ok(changes) = scheduler.on_done(cid) {
                        // Record process state.
                        recorder.record_changes(
                            event.time_ms,
                            &changes,
                            manager_cycle,
                            &ctx,
                            &mut events,
                        );
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
        recorder.executions,
        recorder.metrics,
        HashMap::new(),
    ))
}
