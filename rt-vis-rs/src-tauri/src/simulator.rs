//!
//! Simulation Engine.
//!

use rt_config::SchedulerConfig;
use serde::{Deserialize, Serialize};

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

#[tauri::command]
pub fn simulate_plan(config: SchedulerConfig) -> Result<SimulationResult, String> {
    // TODO: implement time-hopping simulation logic
    Ok(SimulationResult {
        executions: Vec::new(),
        metrics: Vec::new(),
    })
}
