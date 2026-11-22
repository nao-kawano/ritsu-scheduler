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
    ActualCycle, ActualExecution, ActualInstantEvent, ActualMetricPoint, AnomalyEventItem,
    ExecutionStatus, InstantEventType, LogSummary,
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

/// Intermediate tracking state for an in-progress execution.
struct InProgressExecution {
    instance_id: u64,
    cycle: u64,
    start_us: u64,
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
    cycle_time_us: u64,

    origin_time_us: Option<i64>,
    prev_cycle_start_us: Option<u64>,
    last_log_us: u64,
    current_running_count: u32,

    in_progress: HashMap<u16, InProgressExecution>,
    executions_by_cid: HashMap<u16, Vec<ActualExecution>>,
    instant_events_by_cid: HashMap<u16, Vec<ActualInstantEvent>>,
    anomaly_events: Vec<AnomalyEventItem>,
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
            cycle_time_us: 50_000,

            origin_time_us: None,
            prev_cycle_start_us: None,
            last_log_us: 0,
            current_running_count: 0,

            in_progress: HashMap::new(),
            executions_by_cid: HashMap::new(),
            instant_events_by_cid: HashMap::new(),
            anomaly_events: Vec::new(),
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
            // Resolve configured cycle_time_ms into microseconds for accurate cycle jitter calculation
            if self.parsed_config.is_none() {
                let cfg = parse_embedded_config(&self.config_lines)?;
                self.cycle_time_us = (cfg.server_config.cycle_time_ms as u64) * 1_000;
                self.parsed_config = Some(cfg);
            }
        }
        // Parse cycle number and calculate relative elapsed time from the origin timestamp.
        let cycle_num = extract_field::<u64>(stat_body, "CYC:").unwrap_or(0);
        let current_us = calculate_relative_us(timestamp_us, self.origin_time_us);
        self.last_log_us = current_us;

        // Calculate cycle-to-cycle start jitter relative to the previous cycle start and cycle_time_us.
        // Relative timestamps are derived from non-negative i64 diffs and cycle_time_us <= u32::MAX * 1_000,
        // so all operands fit safely within i64.
        let start_jitter_us = match self.prev_cycle_start_us {
            None => 0,
            Some(prev_start) => {
                let elapsed_us = (current_us as i64).saturating_sub(prev_start as i64);
                elapsed_us.saturating_sub(self.cycle_time_us as i64)
            }
        };
        self.prev_cycle_start_us = Some(current_us);

        // Record parsed actual cycle event.
        self.cycles.push(ActualCycle {
            cycle: cycle_num,
            start_us: current_us,
            start_jitter_us,
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
        let current_us = calculate_relative_us(timestamp_us, self.origin_time_us);
        self.last_log_us = current_us;

        // Record retransmission event and skip state updates to prevent duplicate execution tracking.
        if stat_body.contains("(Retransmit)") {
            self.record_instant_event(cid, cycle_num, current_us, InstantEventType::Retransmit);
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
                    start_us: current_us,
                    log_line_no_start: self.line_no,
                    status: ExecutionStatus::Normal,
                },
            );
            self.current_running_count += 1;
            record_metric_point(&mut self.metrics, current_us, self.current_running_count);
        } else if stat_body.contains("Running -> Idle") || stat_body.contains("DONE (Exiting)") {
            // Normal execution completes upon receiving DONE (including server Exiting state);
            // finalize bar and decrement active concurrency.
            if let Some(exec) = self.in_progress.remove(&cid) {
                let duration_us = current_us.saturating_sub(exec.start_us);
                self.executions_by_cid
                    .entry(cid)
                    .or_default()
                    .push(ActualExecution {
                        log_line_no_start: exec.log_line_no_start,
                        log_line_no_end: Some(self.line_no),
                        instance_id: exec.instance_id,
                        cycle: exec.cycle,
                        start_us: exec.start_us,
                        duration_us,
                        status: exec.status,
                    });
                self.current_running_count = self.current_running_count.saturating_sub(1);
                record_metric_point(&mut self.metrics, current_us, self.current_running_count);
            }
        } else if stat_body.contains("Running -> Overrun") {
            // Task exceeded cycle boundary; mark execution status as Overrun.
            if let Some(exec) = self.in_progress.get_mut(&cid) {
                exec.status = ExecutionStatus::Overrun;
            }
            self.record_instant_event(cid, cycle_num, current_us, InstantEventType::Overrun);
        } else if stat_body.contains("Overrun -> Late") {
            // Overrun task completes late upon receiving DONE; finalize bar and record Late event.
            if let Some(exec) = self.in_progress.remove(&cid) {
                let duration_us = current_us.saturating_sub(exec.start_us);
                self.executions_by_cid
                    .entry(cid)
                    .or_default()
                    .push(ActualExecution {
                        log_line_no_start: exec.log_line_no_start,
                        log_line_no_end: Some(self.line_no),
                        instance_id: exec.instance_id,
                        cycle: exec.cycle,
                        start_us: exec.start_us,
                        duration_us,
                        status: ExecutionStatus::Overrun,
                    });
                self.current_running_count = self.current_running_count.saturating_sub(1);
                record_metric_point(&mut self.metrics, current_us, self.current_running_count);
            }
            self.record_instant_event(cid, cycle_num, current_us, InstantEventType::Late);
        } else if stat_body.contains("Ready -> Skip") {
            // Task execution skipped due to dependency delay or prior skip; record Skip event.
            self.record_instant_event(cid, cycle_num, current_us, InstantEventType::Skip);
        } else if stat_body.contains("Idle -> Late") {
            // Client missed cycle start without sending READY in time; record Late event.
            self.record_instant_event(cid, cycle_num, current_us, InstantEventType::Late);
        } else if stat_body.contains("Idle -> Ready")
            || stat_body.contains("Skip -> Ready")
            || stat_body.contains("Late -> Idle")
        {
            // Client transmitted READY to announce readiness or recover from late state; record Ready event.
            self.record_instant_event(cid, cycle_num, current_us, InstantEventType::Ready);
        } else if stat_body.contains("EXIT") {
            // Client disconnected or terminated; record Exit event.
            self.record_instant_event(cid, cycle_num, current_us, InstantEventType::Exit);
        }
    }

    /// Records an instant point event for a client and captures anomalies for the timeline index.
    fn record_instant_event(
        &mut self,
        cid: u16,
        cycle: u64,
        time_us: u64,
        event_type: InstantEventType,
    ) {
        let event_id = self.next_instant_event_id;
        self.next_instant_event_id += 1;
        self.instant_events_by_cid
            .entry(cid)
            .or_default()
            .push(ActualInstantEvent {
                log_line_no: self.line_no,
                instance_id: event_id,
                cycle,
                time_us,
                event_type,
            });
        if event_type.is_anomaly() {
            self.anomaly_events.push(AnomalyEventItem {
                log_line_no: self.line_no,
                instance_id: event_id,
                cid,
                cycle,
                time_us,
                event_type,
            });
        }
    }

    /// Finalizes parsed structures, computes log statistics, and constructs the LogStore.
    fn finish(mut self) -> Result<LogStore, String> {
        // Safely close any unfinished executions (e.g., server aborted or truncated log).
        for (cid, exec) in self.in_progress.into_iter() {
            let duration_us = self.last_log_us.saturating_sub(exec.start_us);
            self.executions_by_cid
                .entry(cid)
                .or_default()
                .push(ActualExecution {
                    log_line_no_start: exec.log_line_no_start,
                    log_line_no_end: None,
                    instance_id: exec.instance_id,
                    cycle: exec.cycle,
                    start_us: exec.start_us,
                    duration_us,
                    status: exec.status,
                });
        }

        // Resolve and validate the scheduler configuration before normalizing client maps.
        let config = match self.parsed_config {
            Some(cfg) => cfg,
            None => parse_embedded_config(&self.config_lines)?,
        };

        // Normalize parsed collections chronologically and ensure entries exist for all configured clients.
        for execs in self.executions_by_cid.values_mut() {
            execs.sort_by_key(|e| e.start_us);
        }
        for evts in self.instant_events_by_cid.values_mut() {
            evts.sort_by_key(|e| e.time_us);
        }
        self.anomaly_events
            .sort_by_key(|item| (item.time_us, item.log_line_no, item.instance_id));
        for client in &config.client_configs {
            self.executions_by_cid.entry(client.client_id).or_default();
            self.instant_events_by_cid
                .entry(client.client_id)
                .or_default();
        }

        // Finalize timeline bounds before computing statistics.
        let total_cycles = self.cycles.len() as u64;
        let cycle_time_us = (config.server_config.cycle_time_ms as u64) * 1_000;
        let final_cycle_end_us = self
            .cycles
            .last()
            .map(|c| c.start_us + cycle_time_us)
            .unwrap_or(0);
        let total_duration_us = final_cycle_end_us.max(self.last_log_us);

        // Compute log statistics and build summary metadata.
        let stats = crate::stats::compute_log_stats(
            &config,
            &self.cycles,
            &self.executions_by_cid,
            &self.instant_events_by_cid,
            &self.anomaly_events,
            &self.metrics,
            total_duration_us,
        );
        let summary = LogSummary {
            config,
            total_duration_us,
            total_cycles,
            stats,
            is_truncated: self.is_truncated,
        };

        Ok(LogStore::new(
            summary,
            self.cycles,
            self.executions_by_cid,
            self.instant_events_by_cid,
            self.anomaly_events,
            self.metrics,
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
fn record_metric_point(metrics: &mut Vec<ActualMetricPoint>, time_us: u64, running_count: u32) {
    if let Some(last) = metrics.last_mut() {
        if last.time_us == time_us {
            last.running_count = running_count;
            return;
        }
    }
    metrics.push(ActualMetricPoint {
        time_us,
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

/// Computes relative microseconds from origin timestamp.
fn calculate_relative_us(timestamp_us: Option<i64>, origin_us: Option<i64>) -> u64 {
    match (timestamp_us, origin_us) {
        (Some(ts), Some(orig)) => {
            let diff = ts.saturating_sub(orig);
            if diff < 0 { 0 } else { diff as u64 }
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
