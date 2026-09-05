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
//! Log Parser for Scheduler Logs.
//!

use rt_config::SchedulerConfig;

use crate::log_store::LogStore;
use crate::types::{
    ActualCycle, ActualExecution, ActualInstantEvent, ActualMetricPoint, ExecutionStatus,
    InstantEventType, LogSummary,
};

use chrono::NaiveDateTime;
use std::collections::HashMap;
use std::io::BufRead;

#[cfg(test)]
#[path = "log_parser_test.rs"]
mod log_parser_test;

/* -------------------------------------------------------------------------- */

const MAX_CYCLES_SAFETY_CAP: usize = 500_000;
const TAG_CONFIG: &str = "<CONFIG>";
const TAG_STAT: &str = "<STAT>";

/* -------------------------------------------------------------------------- */

/// Intermediate tracking state for an in-progress process execution.
struct InProgressExecution {
    instance_id: u64,
    cycle: u64,
    start_ms: u64,
    log_line_no_start: u64,
    status: ExecutionStatus,
}

/// Internal parser context managing state variables during log parsing.
struct LogParserContext {
    line_no: u64,
    next_execution_id: u64,
    next_instant_event_id: u64,

    config_lines: Vec<String>,
    parsed_config: Option<SchedulerConfig>,
    cycle_time_ms: u64,

    origin_time_us: Option<i64>,
    prev_cycle_start_ms: Option<u64>,
    last_log_ms: u64,
    current_running_count: u32,

    in_progress: HashMap<u16, InProgressExecution>,
    executions_by_cid: HashMap<u16, Vec<ActualExecution>>,
    instant_events_by_cid: HashMap<u16, Vec<ActualInstantEvent>>,
    metrics: Vec<ActualMetricPoint>,
    cycles: Vec<ActualCycle>,

    is_truncated: bool,
}

impl LogParserContext {
    /// Initializes a blank parser context.
    fn new() -> Self {
        Self {
            line_no: 0,
            next_execution_id: 1,
            next_instant_event_id: 1,

            config_lines: Vec::new(),
            parsed_config: None,
            cycle_time_ms: 50,

            origin_time_us: None,
            prev_cycle_start_ms: None,
            last_log_ms: 0,
            current_running_count: 0,

            in_progress: HashMap::new(),
            executions_by_cid: HashMap::new(),
            instant_events_by_cid: HashMap::new(),
            metrics: Vec::new(),
            cycles: Vec::new(),

            is_truncated: false,
        }
    }

    /// Records an embedded configuration line.
    fn handle_config_line(&mut self, config_str: &str) {
        self.config_lines.push(config_str.to_string());
    }

    /// Handles a cycle start event.
    fn handle_cycle_start(
        &mut self,
        stat_body: &str,
        timestamp_us: Option<i64>,
    ) -> Result<(), String> {
        // Establish time origin at the first START cycle event
        if self.origin_time_us.is_none() {
            if let Some(ts_us) = timestamp_us {
                self.origin_time_us = Some(ts_us);
            }
            // Resolve configured cycle_time_ms for accurate cycle jitter calculation
            if self.parsed_config.is_none() {
                let cfg = parse_embedded_config(&self.config_lines)?;
                self.cycle_time_ms = cfg.server_config.cycle_time_ms as u64;
                self.parsed_config = Some(cfg);
            }
        }
        // Parse cycle number and calculate relative elapsed time from the origin timestamp.
        let cycle_num = extract_field::<u64>(stat_body, "CYC:").unwrap_or(0);
        let current_ms = calculate_relative_ms(timestamp_us, self.origin_time_us);
        self.last_log_ms = current_ms;

        // Calculate cycle-to-cycle start jitter relative to the previous cycle start and cycle_time_ms.
        let start_jitter_ms = match self.prev_cycle_start_ms {
            None => 0.0,
            Some(prev_start) => {
                (current_ms as f64) - (prev_start as f64 + self.cycle_time_ms as f64)
            }
        };
        self.prev_cycle_start_ms = Some(current_ms);

        // Record parsed actual cycle event.
        self.cycles.push(ActualCycle {
            cycle: cycle_num,
            start_ms: current_ms,
            start_jitter_ms,
        });

        // Mark log as truncated when reaching the safety cycle limit to prevent excessive memory usage.
        if self.cycles.len() >= MAX_CYCLES_SAFETY_CAP {
            self.is_truncated = true;
        }

        Ok(())
    }

    /// Handles client-specific state transitions.
    fn handle_client_stat(&mut self, cid: u16, stat_body: &str, timestamp_us: Option<i64>) {
        // Ignore client events occurring prior to the initial cycle START (e.g. startup JOIN sequence).
        if self.origin_time_us.is_none() {
            return;
        }

        // Parse cycle number and calculate relative elapsed time from the origin timestamp.
        let cycle_num = extract_field::<u64>(stat_body, "CYC:").unwrap_or(0);
        let current_ms = calculate_relative_ms(timestamp_us, self.origin_time_us);
        self.last_log_ms = current_ms;

        // Record retransmission event and skip state updates to prevent duplicate execution tracking.
        if stat_body.contains("(Retransmit)") {
            self.record_instant_event(cid, current_ms, InstantEventType::Retransmit);
            return;
        }

        // Dispatch and track execution bars and instant events based on client state transitions.
        if stat_body.contains("Ready -> Running") {
            // Task starts executing; instantiate tracking bar and increment active concurrency.
            let execution_id = self.next_execution_id;
            self.next_execution_id += 1;
            self.in_progress.insert(
                cid,
                InProgressExecution {
                    instance_id: execution_id,
                    cycle: cycle_num,
                    start_ms: current_ms,
                    log_line_no_start: self.line_no,
                    status: ExecutionStatus::Normal,
                },
            );
            self.current_running_count += 1;
            record_metric_point(&mut self.metrics, current_ms, self.current_running_count);
        } else if stat_body.contains("Running -> Idle") {
            // Normal execution completes upon receiving DONE; finalize bar and decrement active concurrency.
            if let Some(exec) = self.in_progress.remove(&cid) {
                let duration_ms = (current_ms.saturating_sub(exec.start_ms)) as u32;
                self.executions_by_cid
                    .entry(cid)
                    .or_default()
                    .push(ActualExecution {
                        log_line_no_start: exec.log_line_no_start,
                        log_line_no_end: Some(self.line_no),
                        instance_id: exec.instance_id,
                        cycle: exec.cycle,
                        start_ms: exec.start_ms,
                        duration_ms,
                        status: exec.status,
                    });
                self.current_running_count = self.current_running_count.saturating_sub(1);
                record_metric_point(&mut self.metrics, current_ms, self.current_running_count);
            }
        } else if stat_body.contains("Running -> Overrun") {
            // Task exceeded cycle boundary; mark execution status as Overrun.
            if let Some(exec) = self.in_progress.get_mut(&cid) {
                exec.status = ExecutionStatus::Overrun;
            }
            self.record_instant_event(cid, current_ms, InstantEventType::Overrun);
        } else if stat_body.contains("Overrun -> Late") {
            // Overrun task completes late upon receiving DONE; finalize bar and record Late event.
            if let Some(exec) = self.in_progress.remove(&cid) {
                let duration_ms = (current_ms.saturating_sub(exec.start_ms)) as u32;
                self.executions_by_cid
                    .entry(cid)
                    .or_default()
                    .push(ActualExecution {
                        log_line_no_start: exec.log_line_no_start,
                        log_line_no_end: Some(self.line_no),
                        instance_id: exec.instance_id,
                        cycle: exec.cycle,
                        start_ms: exec.start_ms,
                        duration_ms,
                        status: ExecutionStatus::Overrun,
                    });
                self.current_running_count = self.current_running_count.saturating_sub(1);
                record_metric_point(&mut self.metrics, current_ms, self.current_running_count);
            }
            self.record_instant_event(cid, current_ms, InstantEventType::Late);
        } else if stat_body.contains("Ready -> Skip") {
            // Task execution skipped due to dependency delay or prior skip; record Skip event.
            self.record_instant_event(cid, current_ms, InstantEventType::Skip);
        } else if stat_body.contains("Idle -> Late") {
            // Client missed cycle start without sending READY in time; record Late event.
            self.record_instant_event(cid, current_ms, InstantEventType::Late);
        } else if stat_body.contains("Idle -> Ready")
            || stat_body.contains("Skip -> Ready")
            || stat_body.contains("Late -> Idle")
        {
            // Client transmitted READY to announce readiness or recover from late state; record Ready event.
            self.record_instant_event(cid, current_ms, InstantEventType::Ready);
        } else if stat_body.contains("EXIT") {
            // Client process disconnected or terminated; record Exit event.
            self.record_instant_event(cid, current_ms, InstantEventType::Exit);
        }
    }

    /// Records an instant point event for a client.
    fn record_instant_event(&mut self, cid: u16, time_ms: u64, event_type: InstantEventType) {
        let event_id = self.next_instant_event_id;
        self.next_instant_event_id += 1;
        self.instant_events_by_cid
            .entry(cid)
            .or_default()
            .push(ActualInstantEvent {
                log_line_no: self.line_no,
                instance_id: event_id,
                time_ms,
                event_type,
            });
    }

    /// Finalizes parsing, safely closes open executions, and builds the LogStore.
    fn finish(mut self) -> Result<LogStore, String> {
        // Safely close any unfinished executions (e.g., server aborted or truncated log)
        for (cid, exec) in self.in_progress.into_iter() {
            let duration_ms = (self.last_log_ms.saturating_sub(exec.start_ms)) as u32;
            self.executions_by_cid
                .entry(cid)
                .or_default()
                .push(ActualExecution {
                    log_line_no_start: exec.log_line_no_start,
                    log_line_no_end: None,
                    instance_id: exec.instance_id,
                    cycle: exec.cycle,
                    start_ms: exec.start_ms,
                    duration_ms,
                    status: exec.status,
                });
        }
        // Ensure scheduler configuration is parsed and validated before building the store.
        let config = match self.parsed_config {
            Some(cfg) => cfg,
            None => parse_embedded_config(&self.config_lines)?,
        };

        // Normalize execution bars and instant events by sorting chronologically and ensuring entries exist for all configured clients.
        for execs in self.executions_by_cid.values_mut() {
            execs.sort_by_key(|e| e.start_ms);
        }
        for evts in self.instant_events_by_cid.values_mut() {
            evts.sort_by_key(|e| e.time_ms);
        }
        for client in &config.client_configs {
            self.executions_by_cid.entry(client.client_id).or_default();
            self.instant_events_by_cid
                .entry(client.client_id)
                .or_default();
        }

        // Compute summary metrics across all parsed cycles and concurrency points.
        let max_concurrency = self
            .metrics
            .iter()
            .map(|m| m.running_count)
            .max()
            .unwrap_or(0);
        let total_cycles = self.cycles.len() as u64;

        let min_start_jitter_ms = self
            .cycles
            .iter()
            .map(|c| c.start_jitter_ms)
            .fold(f64::INFINITY, f64::min);
        let max_start_jitter_ms = self
            .cycles
            .iter()
            .map(|c| c.start_jitter_ms)
            .fold(f64::NEG_INFINITY, f64::max);

        let final_cycle_end_ms = self
            .cycles
            .last()
            .map(|c| c.start_ms + (config.server_config.cycle_time_ms as u64))
            .unwrap_or(0);
        let total_duration_ms = final_cycle_end_ms.max(self.last_log_ms);

        // Construct log summary metadata.
        let summary = LogSummary {
            config,
            max_concurrency,
            total_duration_ms,
            total_cycles,
            min_start_jitter_ms: if min_start_jitter_ms.is_finite() {
                min_start_jitter_ms
            } else {
                0.0
            },
            max_start_jitter_ms: if max_start_jitter_ms.is_finite() {
                max_start_jitter_ms
            } else {
                0.0
            },
            is_truncated: self.is_truncated,
        };

        Ok(LogStore::new(
            summary,
            self.executions_by_cid,
            self.instant_events_by_cid,
            self.metrics,
            self.cycles,
        ))
    }
}

/* -------------------------------------------------------------------------- */

/// Parses scheduler execution log from a buffered reader and constructs a populated LogStore.
pub fn parse_log<R: BufRead>(reader: R) -> Result<LogStore, String> {
    let mut ctx = LogParserContext::new();

    for line_result in reader.lines() {
        ctx.line_no += 1;
        let line =
            line_result.map_err(|e| format!("Failed to read line {}: {}", ctx.line_no, e))?;

        // Handle embedded configuration lines.
        if let Some(config_str) = extract_config_line(&line) {
            ctx.handle_config_line(config_str);
            continue;
        }

        // Handle state transition lines.
        if let Some(stat_body) = extract_stat_body(&line) {
            let timestamp_us = parse_line_timestamp_micros(&line);

            if is_cycle_start(stat_body) {
                ctx.handle_cycle_start(stat_body, timestamp_us)?;
                if ctx.is_truncated {
                    break;
                }
                continue;
            }

            if let Some(cid) = extract_field::<u16>(stat_body, "CID:") {
                ctx.handle_client_stat(cid, stat_body, timestamp_us);
                continue;
            }

            continue;
        }
    }

    ctx.finish()
}

/// Helper to parse log from an in-memory string.
pub fn parse_log_str(content: &str) -> Result<LogStore, String> {
    parse_log(content.as_bytes())
}

/* -------------------------------------------------------------------------- */

/// Extracts configuration line content if the line contains the <CONFIG> tag.
fn extract_config_line(line: &str) -> Option<&str> {
    line.find(TAG_CONFIG)
        .map(|pos| line[pos + TAG_CONFIG.len()..].trim_start())
}

/// Parses and validates embedded configuration lines into SchedulerConfig.
fn parse_embedded_config(config_lines: &[String]) -> Result<SchedulerConfig, String> {
    if config_lines.is_empty() {
        return Err("No <CONFIG> lines found in log file".to_string());
    }
    let config_toml = config_lines.join("\n");
    let cfg: SchedulerConfig = toml::from_str(&config_toml)
        .map_err(|e| format!("Failed to parse embedded config: {}", e))?;
    cfg.validate()
        .map_err(|e| format!("Config validation failed: {:?}", e))?;
    Ok(cfg)
}

/// Extracts state transition message body if the line contains the <STAT> tag.
fn extract_stat_body(line: &str) -> Option<&str> {
    line.find(TAG_STAT)
        .map(|pos| line[pos + TAG_STAT.len()..].trim())
}

/// Checks if the state message indicates a cycle start event.
fn is_cycle_start(stat_body: &str) -> bool {
    !stat_body.contains("[Manager]")
        && !stat_body.contains("CID:")
        && stat_body.contains("CYC:")
        && stat_body.contains("START")
}

/// Records concurrency metric point with same-timestamp deduplication.
fn record_metric_point(metrics: &mut Vec<ActualMetricPoint>, time_ms: u64, running_count: u32) {
    if let Some(last) = metrics.last_mut() {
        if last.time_ms == time_ms {
            last.running_count = running_count;
            return;
        }
    }
    metrics.push(ActualMetricPoint {
        time_ms,
        running_count,
    });
}

/// Parses timestamp from the beginning of log line into microseconds.
fn parse_line_timestamp_micros(line: &str) -> Option<i64> {
    // Safely slices ASCII timestamp without panicking on UTF-8 char boundary in case of malformed lines.
    let date_str = line.get(0..26)?;
    NaiveDateTime::parse_from_str(date_str, "%Y/%m/%d %H:%M:%S%.6f")
        .ok()
        .map(|dt| dt.and_utc().timestamp_micros())
}

/// Computes relative milliseconds from origin timestamp.
fn calculate_relative_ms(timestamp_us: Option<i64>, origin_us: Option<i64>) -> u64 {
    match (timestamp_us, origin_us) {
        (Some(ts), Some(orig)) => {
            let diff = ts.saturating_sub(orig);
            if diff < 0 { 0 } else { (diff / 1000) as u64 }
        }
        _ => 0,
    }
}

/// Extracts a parsed value of type T following a specified key (e.g., "CYC:", "CID:").
fn extract_field<T: std::str::FromStr>(s: &str, key: &str) -> Option<T> {
    let pos = s.find(key)?;
    let start = pos + key.len();
    let end = s[start..]
        .find(|c: char| !c.is_ascii_digit())
        .map(|p| start + p)
        .unwrap_or(s.len());
    s[start..end].parse::<T>().ok()
}
