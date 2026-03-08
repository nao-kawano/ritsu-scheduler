//!
//! Manager state.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use rt_message::{Message, MessageType};

use super::EventResult;
use super::context::ManagerContext;
use super::process::ManagerProc;
use crate::config::ClientConfig;
use rt_core::ProcessState;
use rt_core::ProcessStateChange;

#[cfg(test)]
#[path = "process_running_test.rs"]
mod process_running_test;

/* -------------------------------------------------------------------------- */

pub struct ManagerProcRunning;
impl ManagerProc for ManagerProcRunning {
    fn enter_state(&self, context: &mut ManagerContext) {
        trace!("enter_state");
        context.cycle_current = ManagerContext::CYCLE_MAX;
    }

    fn on_cycle_start(&self, context: &mut ManagerContext, _cycle: u64) -> EventResult {
        let cycle = self.update_cycle(context);
        debug!("<STAT> CYC:{:05} START", cycle);
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
        // convert changes to response.
        for change in changes {
            let c = context.clients.get(&change.cid).unwrap();
            match change.after {
                ProcessState::Running => {
                    debug!(
                        "<STAT> CYC:{:05} CID:{:03} MID:{} {:?} -> {:?} (Cycle)",
                        context.cycle_current, change.cid, c.last_mid, change.before, change.after
                    );
                    responses
                        .push(Message::new(MessageType::Ok, c.last_mid, change.cid, None).unwrap());
                }
                ProcessState::Overrun => {
                    warn!(
                        "<STAT> CYC:{:05} CID:{:03} MID:{} {:?} -> {:?}",
                        context.cycle_current, change.cid, c.last_mid, change.before, change.after
                    );
                }
                ProcessState::Skip => {
                    warn!(
                        "<STAT> CYC:{:05} CID:{:03} MID:{} {:?} -> {:?}",
                        context.cycle_current, change.cid, c.last_mid, change.before, change.after
                    );
                    responses.push(
                        Message::new(MessageType::Skip, c.last_mid, change.cid, None).unwrap(),
                    );
                }
                ProcessState::Late => {
                    warn!(
                        "<STAT> CYC:{:05} CID:{:03} MID:{} {:?} -> {:?}",
                        context.cycle_current, change.cid, c.last_mid, change.before, change.after
                    );
                }
                ProcessState::Ready | ProcessState::Idle => {
                    warn!(
                        "CYC:{:05} CID:{:03} invalid state change by start {:?}",
                        context.cycle_current, change.cid, change
                    );
                }
            }
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
        // convert changes to response.
        for change in changes {
            let c = context.clients.get(&change.cid).unwrap();
            match change.after {
                ProcessState::Ready => {
                    if change.before == ProcessState::Ready {
                        warn!(
                            "<STAT> CYC:{:05} CID:{:03} MID:{} {:?} -> {:?} (Retransmit)",
                            context.cycle_current,
                            change.cid,
                            c.last_mid,
                            change.before,
                            change.after
                        );
                    } else {
                        debug!(
                            "<STAT> CYC:{:05} CID:{:03} MID:{} {:?} -> {:?}",
                            context.cycle_current,
                            change.cid,
                            c.last_mid,
                            change.before,
                            change.after
                        );
                    }
                }
                ProcessState::Running => {
                    // maybe retransmission, send OK to start immediately.
                    warn!(
                        "<STAT> CYC:{:05} CID:{:03} MID:{} {:?} -> {:?} (Retransmit)",
                        context.cycle_current, change.cid, c.last_mid, change.before, change.after
                    );
                    responses
                        .push(Message::new(MessageType::Ok, c.last_mid, change.cid, None).unwrap());
                }
                ProcessState::Idle => {
                    if change.before == ProcessState::Late {
                        debug!(
                            "<STAT> CYC:{:05} CID:{:03} MID:{} {:?} -> {:?} (Late)",
                            context.cycle_current,
                            change.cid,
                            c.last_mid,
                            change.before,
                            change.after
                        );
                        responses.push(
                            Message::new(MessageType::Late, c.last_mid, change.cid, None).unwrap(),
                        );
                    } else {
                        // already idle or unexpected.
                        warn!(
                            "CYC:{:05} CID:{:03} MID:{} ignore READY in {:?}",
                            context.cycle_current, change.cid, c.last_mid, change.before
                        );
                    }
                }
                ProcessState::Overrun | ProcessState::Skip | ProcessState::Late => {
                    warn!(
                        "CID:{:03} invalid state change by start {:?}",
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
        // convert changes to response.
        for change in changes {
            let c = context.clients.get(&change.cid).unwrap();
            match change.after {
                ProcessState::Running => {
                    debug!(
                        "<STAT> CYC:{:05} CID:{:03} MID:{} {:?} -> {:?} (Dependency)",
                        context.cycle_current, change.cid, c.last_mid, change.before, change.after
                    );
                    responses
                        .push(Message::new(MessageType::Ok, c.last_mid, change.cid, None).unwrap());
                }
                ProcessState::Idle => {
                    // normal case or retransmission.
                    if change.before == ProcessState::Running {
                        debug!(
                            "<STAT> CYC:{:05} CID:{:03} MID:{} {:?} -> {:?}",
                            context.cycle_current,
                            change.cid,
                            c.last_mid,
                            change.before,
                            change.after
                        );
                    } else {
                        // maybe retransmission.
                        warn!(
                            "<STAT> CYC:{:05} CID:{:03} MID:{} {:?} -> {:?} (Retransmit)",
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
                        "<STAT> CYC:{:05} CID:{:03} MID:{} {:?} -> {:?} (Late)",
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

    fn update_cycle(&self, context: &mut ManagerContext) -> u32 {
        if context.cycle_current >= ManagerContext::CYCLE_MAX {
            context.cycle_current = 0;
        } else {
            context.cycle_current += 1;
        }
        return context.cycle_current;
    }

    fn is_target_cycle(&self, cycle: u32, config: &ClientConfig) -> bool {
        let target_cycle = config.cycle as u32;
        let target_cycle_offset = config.cycle_offset as u32;
        return cycle % target_cycle == target_cycle_offset;
    }
}
