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
        info!("[STAT] CYC:{:04} start", cycle);
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
                    info!(
                        "[STAT] CYC:{:04} CID:{:03} START",
                        context.cycle_current, change.cid
                    );
                    responses
                        .push(Message::new(MessageType::Ok, c.last_mid, change.cid, None).unwrap());
                }
                ProcessState::Overrun => {
                    warn!(
                        "[STAT] CYC:{:04} CID:{:03} OVERRUN",
                        context.cycle_current, change.cid
                    );
                }
                ProcessState::Skip => {
                    info!(
                        "[STAT] CYC:{:04} CID:{:03} SKIP",
                        context.cycle_current, change.cid
                    );
                    responses.push(
                        Message::new(MessageType::Skip, c.last_mid, change.cid, None).unwrap(),
                    );
                }
                ProcessState::Late => {
                    info!(
                        "[STAT] CYC:{:04} CID:{:03} SKIP(LATE)",
                        context.cycle_current, change.cid
                    );
                }
                ProcessState::Ready | ProcessState::Idle => {
                    warn!(
                        "CID:{:03} invalid state change by start {:?}",
                        change.cid, change
                    );
                }
            }
        }
        Ok(responses)
    }

    fn on_client_join(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_join@{} CID:{:03}", message.mid, message.cid);
        Err(format!("invalid Join from CID:{:03}", message.cid))
    }

    fn on_client_ready(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_ready@{} CID:{:03}", message.mid, message.cid);
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
                    info!(
                        "[STAT] CYC:{:04} CID:{:03} READY",
                        context.cycle_current, change.cid
                    );
                }
                ProcessState::Running => {
                    // maybe retransmission, send OK to start immediately.
                    responses
                        .push(Message::new(MessageType::Ok, c.last_mid, change.cid, None).unwrap());
                }
                ProcessState::Idle => {
                    if change.before == ProcessState::Late {
                        info!(
                            "[STAT] CYC:{:04} CID:{:03} READY(LATE)",
                            context.cycle_current, change.cid
                        );
                        responses.push(
                            Message::new(MessageType::Late, c.last_mid, change.cid, None).unwrap(),
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
        trace!("on_client_done@{} CID:{:03}", message.mid, message.cid);
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
                    info!(
                        "[STAT] CYC:{:04} CID:{:03} START(NEXT)",
                        context.cycle_current, change.cid
                    );
                    responses
                        .push(Message::new(MessageType::Ok, c.last_mid, change.cid, None).unwrap());
                }
                ProcessState::Idle => {
                    // normal case or retransmission.
                    if change.before == ProcessState::Running {
                        info!(
                            "[STAT] CYC:{:04} CID:{:03} DONE",
                            context.cycle_current, change.cid
                        );
                    }
                    responses
                        .push(Message::new(MessageType::Ok, c.last_mid, change.cid, None).unwrap());
                }
                ProcessState::Late => {
                    info!(
                        "[STAT] CYC:{:04} CID:{:03} DONE(LATE)",
                        context.cycle_current, change.cid
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
        trace!("on_client_exit@{} CID:{:03}", message.mid, message.cid);
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
