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
//! Manager state.
//!

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use rt_config::ClientConfig;
use rt_core::{ProcessState, ProcessStateChange};
use rt_message::{Message, MessageType};

use super::EventResult;
use super::context::{ClientInfo, ManagerContext};
use super::process::ManagerProc;

use std::time;

#[cfg(test)]
#[path = "process_running_test.rs"]
mod process_running_test;

/* -------------------------------------------------------------------------- */

pub struct ManagerProcRunning;
impl ManagerProc for ManagerProcRunning {
    fn enter_state(&self, context: &mut ManagerContext) {
        trace!("enter_state");
        context.cycle_current = -1;
    }

    fn on_cycle_start(&self, context: &mut ManagerContext, global_cycle: u64) -> EventResult {
        if context.stats.start_at.is_none() {
            context.stats.start_at = Some(time::Instant::now());
            context.stats.start_cycle = global_cycle;
        }
        context.stats.last_cycle = global_cycle;

        // Increment cycle and check safety limit.
        context.cycle_current += 1;
        if context.cycle_current >= ManagerContext::CYCLE_MAX {
            warn!(
                "<STAT> CYC:{:012} ABORT (Reason: CycleLimitReached)",
                context.cycle_current
            );
            let mut responses: Vec<Message> = Vec::new();
            context.exit_reason = Some(vec![(
                "reason".to_string(),
                "CycleLimitReached".to_string(),
            )]);
            self.going_to_exit(context, &mut responses);
            return Ok(responses);
        }
        let cycle = context.cycle_current;
        debug!("<STAT> CYC:{:012} START", cycle);
        let mut responses: Vec<Message> = Vec::new();
        // update state: check and start trigger=cycle clients.
        let mut changes: Vec<ProcessStateChange> = Vec::new();
        for client_id in &context.graph_start {
            let client = context.clients.get_mut(client_id).unwrap();
            if self.is_target_cycle(context.cycle_current, &client.config) {
                let r = context.sched.on_start(*client_id);
                if let Ok(cs) = r {
                    changes.extend(cs);
                } else {
                    return Err(r.unwrap_err());
                }
            }
        }

        self.update_stats(context, &changes);

        // convert changes to response.
        for change in changes {
            let c = context.clients.get(&change.cid).unwrap();
            match change.after {
                ProcessState::Running => {
                    debug!(
                        "<STAT> CYC:{:012} CID:{:03} MID:{} {:?} -> {:?} (Cycle)",
                        context.cycle_current, change.cid, c.last_mid, change.before, change.after
                    );
                    responses.push(
                        Message::new(
                            MessageType::Start,
                            c.last_mid,
                            change.cid,
                            Some(vec![(
                                "cycle".to_string(),
                                context.cycle_current.to_string(),
                            )]),
                        )
                        .unwrap(),
                    );
                }
                ProcessState::Overrun => {
                    warn!(
                        "<STAT> CYC:{:012} CID:{:03} MID:{} {:?} -> {:?}",
                        context.cycle_current, change.cid, c.last_mid, change.before, change.after
                    );
                }
                ProcessState::Skip => {
                    warn!(
                        "<STAT> CYC:{:012} CID:{:03} MID:{} {:?} -> {:?}",
                        context.cycle_current, change.cid, c.last_mid, change.before, change.after
                    );
                    responses.push(
                        Message::new(
                            MessageType::Skip,
                            c.last_mid,
                            change.cid,
                            Some(vec![(
                                "cycle".to_string(),
                                context.cycle_current.to_string(),
                            )]),
                        )
                        .unwrap(),
                    );
                }
                ProcessState::Late => {
                    warn!(
                        "<STAT> CYC:{:012} CID:{:03} MID:{} {:?} -> {:?}",
                        context.cycle_current, change.cid, c.last_mid, change.before, change.after
                    );
                }
                ProcessState::Ready | ProcessState::Idle => {
                    warn!(
                        "CYC:{:012} CID:{:03} invalid state change by start {:?}",
                        context.cycle_current, change.cid, change
                    );
                }
            }
        }

        let running_cycles = global_cycle.saturating_sub(context.stats.start_cycle);
        if context.stats.interval_cycle > 0
            && running_cycles > 0
            && running_cycles % (context.stats.interval_cycle as u64) == 0
        {
            context.dump_stats(global_cycle);
        }

        Ok(responses)
    }

    fn on_client_join(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_join CID:{:03} MID:{}", message.cid, message.mid);
        Err(format!("invalid Join from CID:{:03}", message.cid))
    }

    fn on_client_ready(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_ready CID:{:03} MID:{}", message.cid, message.mid);
        let mut responses: Vec<Message> = Vec::new();
        // update state.
        let r = context.sched.on_ready(message.cid);
        let Ok(changes) = r else {
            return Err(r.unwrap_err());
        };

        self.update_stats(context, &changes);

        // convert changes to response.
        for change in changes {
            let c = context.clients.get(&change.cid).unwrap();
            match change.after {
                ProcessState::Ready => {
                    if change.before == ProcessState::Ready {
                        warn!(
                            "<STAT> CYC:{:012} CID:{:03} MID:{} {:?} -> {:?} (Retransmit)",
                            context.cycle_current,
                            change.cid,
                            c.last_mid,
                            change.before,
                            change.after
                        );
                    } else {
                        debug!(
                            "<STAT> CYC:{:012} CID:{:03} MID:{} {:?} -> {:?}",
                            context.cycle_current,
                            change.cid,
                            c.last_mid,
                            change.before,
                            change.after
                        );
                    }
                }
                ProcessState::Running => {
                    // maybe retransmission, send START to start immediately.
                    warn!(
                        "<STAT> CYC:{:012} CID:{:03} MID:{} {:?} -> {:?} (Retransmit)",
                        context.cycle_current, change.cid, c.last_mid, change.before, change.after
                    );
                    responses.push(
                        Message::new(
                            MessageType::Start,
                            c.last_mid,
                            change.cid,
                            Some(vec![(
                                "cycle".to_string(),
                                context.cycle_current.to_string(),
                            )]),
                        )
                        .unwrap(),
                    );
                }
                ProcessState::Idle => {
                    if change.before == ProcessState::Late {
                        debug!(
                            "<STAT> CYC:{:012} CID:{:03} MID:{} {:?} -> {:?} (Late)",
                            context.cycle_current,
                            change.cid,
                            c.last_mid,
                            change.before,
                            change.after
                        );
                        responses.push(
                            Message::new(
                                MessageType::Late,
                                c.last_mid,
                                change.cid,
                                Some(vec![(
                                    "cycle".to_string(),
                                    context.cycle_current.to_string(),
                                )]),
                            )
                            .unwrap(),
                        );
                    } else {
                        // already idle or unexpected.
                        warn!(
                            "CYC:{:012} CID:{:03} MID:{} ignore READY in {:?}",
                            context.cycle_current, change.cid, c.last_mid, change.before
                        );
                    }
                }
                ProcessState::Overrun | ProcessState::Skip | ProcessState::Late => {
                    warn!(
                        "CID:{:03} invalid state change by ready {:?}",
                        change.cid, change
                    );
                }
            }
        }
        Ok(responses)
    }

    fn on_client_done(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_done CID:{:03} MID:{}", message.cid, message.mid);
        let mut responses: Vec<Message> = Vec::new();
        // update state.
        let r = context.sched.on_done(message.cid);
        let Ok(changes) = r else {
            return Err(r.unwrap_err());
        };

        self.update_stats(context, &changes);

        // convert changes to response.
        for change in changes {
            let c = context.clients.get(&change.cid).unwrap();
            match change.after {
                ProcessState::Running => {
                    debug!(
                        "<STAT> CYC:{:012} CID:{:03} MID:{} {:?} -> {:?} (Dependency)",
                        context.cycle_current, change.cid, c.last_mid, change.before, change.after
                    );
                    responses.push(
                        Message::new(
                            MessageType::Start,
                            c.last_mid,
                            change.cid,
                            Some(vec![(
                                "cycle".to_string(),
                                context.cycle_current.to_string(),
                            )]),
                        )
                        .unwrap(),
                    );
                }
                ProcessState::Idle => {
                    // normal case or retransmission.
                    if change.before == ProcessState::Running {
                        debug!(
                            "<STAT> CYC:{:012} CID:{:03} MID:{} {:?} -> {:?}",
                            context.cycle_current,
                            change.cid,
                            c.last_mid,
                            change.before,
                            change.after
                        );
                    } else {
                        // maybe retransmission.
                        warn!(
                            "<STAT> CYC:{:012} CID:{:03} MID:{} {:?} -> {:?} (Retransmit)",
                            context.cycle_current,
                            change.cid,
                            c.last_mid,
                            change.before,
                            change.after
                        );
                    }
                    responses
                        .push(Message::new(MessageType::Ok, c.last_mid, change.cid, None).unwrap());
                }
                ProcessState::Late => {
                    warn!(
                        "<STAT> CYC:{:012} CID:{:03} MID:{} {:?} -> {:?} (Late)",
                        context.cycle_current, change.cid, c.last_mid, change.before, change.after
                    );
                    responses
                        .push(Message::new(MessageType::Ok, c.last_mid, change.cid, None).unwrap());
                }
                ProcessState::Ready | ProcessState::Overrun | ProcessState::Skip => {
                    warn!(
                        "CID:{:03} invalid state change by done {:?}",
                        change.cid, change
                    );
                }
            }
        }
        Ok(responses)
    }

    fn on_client_exit(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_exit CID:{:03} MID:{}", message.cid, message.mid);
        return self.handle_client_exit(context, message);
    }

    fn on_shutdown(&self, context: &mut ManagerContext) -> EventResult {
        trace!("on_shutdown");
        return self.handle_shutdown(context);
    }
}

impl ManagerProcRunning {
    // -----
    // private methods.

    fn is_target_cycle(&self, cycle: i64, config: &ClientConfig) -> bool {
        let target_cycle = config.cycle as i64;
        let target_cycle_offset = config.cycle_offset as i64;
        return cycle % target_cycle == target_cycle_offset;
    }

    fn update_stats(&self, context: &mut ManagerContext, changes: &[ProcessStateChange]) {
        if context.stats.interval_cycle == 0 {
            return;
        }

        for change in changes {
            if change.before == change.after {
                continue;
            }
            let client = context.clients.get_mut(&change.cid).unwrap();
            match change.after {
                ProcessState::Running => {
                    client.running_start_at = Some(time::Instant::now());
                }
                ProcessState::Idle => {
                    if change.before == ProcessState::Running {
                        self.record_success(client);
                    }
                }
                ProcessState::Late => {
                    if change.before == ProcessState::Overrun {
                        self.record_overrun_success(client);
                    } else {
                        client.stats.late_count += 1;
                        client.stats.trigger_count += 1;
                    }
                }
                ProcessState::Skip => {
                    client.stats.skip_count += 1;
                    client.stats.trigger_count += 1;
                }
                ProcessState::Overrun => {
                    // overrun count is updated upon completion in record_overrun_success
                }
                ProcessState::Ready => {}
            }
        }
    }

    fn record_success(&self, client: &mut ClientInfo) {
        client.stats.success_count += 1;
        client.stats.trigger_count += 1;
        if let Some(start_at) = client.running_start_at.take() {
            let elapsed = start_at.elapsed().as_millis() as u32;
            client.stats.time_min = client.stats.time_min.min(elapsed);
            client.stats.time_max = client.stats.time_max.max(elapsed);
            client.stats.time_sum += elapsed as u64;
        }
    }

    fn record_overrun_success(&self, client: &mut ClientInfo) {
        client.stats.overrun_count += 1;
        client.stats.trigger_count += 1;
        if let Some(start_at) = client.running_start_at.take() {
            let elapsed = start_at.elapsed().as_millis() as u32;
            client.stats.overrun_time_min = client.stats.overrun_time_min.min(elapsed);
            client.stats.overrun_time_max = client.stats.overrun_time_max.max(elapsed);
            client.stats.overrun_time_sum += elapsed as u64;
        }
    }
}
