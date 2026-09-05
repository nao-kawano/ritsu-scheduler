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
//! In-Memory Log Store & Viewport Slice Operations.
//!

use crate::types::{
    ActualCycle, ActualExecution, ActualInstantEvent, ActualMetricPoint, LogSummary,
};

use std::collections::HashMap;

/* -------------------------------------------------------------------------- */

/// In-memory dataset holding the full parsed log for lifetime of the session.
#[derive(Debug, Clone)]
pub struct LogStore {
    pub summary: LogSummary,
    pub executions_by_cid: HashMap<u16, Vec<ActualExecution>>,
    pub instant_events_by_cid: HashMap<u16, Vec<ActualInstantEvent>>,
    pub metrics: Vec<ActualMetricPoint>,
    pub cycles: Vec<ActualCycle>,
}

impl LogStore {
    /// Constructs a new populated LogStore from parsed components.
    pub fn new(
        summary: LogSummary,
        executions_by_cid: HashMap<u16, Vec<ActualExecution>>,
        instant_events_by_cid: HashMap<u16, Vec<ActualInstantEvent>>,
        metrics: Vec<ActualMetricPoint>,
        cycles: Vec<ActualCycle>,
    ) -> Self {
        Self {
            summary,
            executions_by_cid,
            instant_events_by_cid,
            metrics,
            cycles,
        }
    }
}
