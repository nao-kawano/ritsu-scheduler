//!
//! Manager state.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use dps_message::{Message, MessageType};

use super::EventResult;
use super::context::ManagerContext;
use super::process::ManagerProc;
use crate::config::ClientConfig;
use dps_core::entry::ProcessState;
use dps_core::scheduler::ProcessStateChange;

#[cfg(test)]
#[path = "process_running_test.rs"]
mod process_running_test;

/* -------------------------------------------------------------------------- */

const LOG_TAG: &str = "StateRunning";

pub struct ManagerProcRunning;
impl ManagerProc for ManagerProcRunning {
    fn enter_state(&self, context: &mut ManagerContext) {
        trace!("{}: enter_state", LOG_TAG);
        context.cycle_current = ManagerContext::CYCLE_MAX;
    }

    fn on_cycle_start(&self, context: &mut ManagerContext, _cycle: u64) -> EventResult {
        let cycle = self.update_cycle(context);
        trace!("{}: on_cycle_start {:04}", LOG_TAG, cycle);
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
            let c = context.clients.get(&change.pid).unwrap();
            match change.after {
                ProcessState::Running => {
                    responses
                        .push(Message::new(MessageType::Ok, c.last_mid, change.pid, None).unwrap());
                }
                ProcessState::Overrun => { /* keep going */ }
                ProcessState::Skip => {
                    responses.push(
                        Message::new(MessageType::Skip, c.last_mid, change.pid, None).unwrap(),
                    );
                }
                ProcessState::SkipPrev => {
                    if change.before == ProcessState::Ready {
                        responses.push(
                            Message::new(MessageType::Skip, c.last_mid, change.pid, None).unwrap(),
                        );
                    }
                }
                _ => {
                    warn!("{}: invalid state change by start {:?}", LOG_TAG, change);
                }
            }
        }
        Ok(responses)
    }

    fn on_client_join(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!(
            "{}: on_client_join@{} id={:03}",
            LOG_TAG, message.mid, message.cid
        );
        Err(format!("invalid Join from {:03}", message.cid))
    }

    fn on_client_ready(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!(
            "{}: on_client_ready@{} id={:03}",
            LOG_TAG, message.mid, message.cid
        );
        let mut responses: Vec<Message> = Vec::new();
        // update state.
        let r = context.sched.on_ready(message.cid);
        let Ok(changes) = r else {
            return Err(r.unwrap_err());
        };
        // convert changes to response.
        for change in changes {
            let c = context.clients.get(&change.pid).unwrap();
            match change.after {
                ProcessState::Ready => { /* keep waiting */ }
                ProcessState::Skip => {
                    responses.push(
                        Message::new(MessageType::Skip, c.last_mid, change.pid, None).unwrap(),
                    );
                }
                ProcessState::Running => {
                    // maybe retransmission, send OK to start immideately.
                    responses
                        .push(Message::new(MessageType::Ok, c.last_mid, change.pid, None).unwrap());
                }
                _ => {
                    warn!("{}: invalid state change by start {:?}", LOG_TAG, change);
                }
            }
        }
        Ok(responses)
    }

    fn on_client_done(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!(
            "{}: on_client_done@{} id={:03}",
            LOG_TAG, message.mid, message.cid
        );
        let mut responses: Vec<Message> = Vec::new();
        // update state.
        let r = context.sched.on_done(message.cid);
        let Ok(changes) = r else {
            return Err(r.unwrap_err());
        };
        // convert changes to response.
        for change in changes {
            let c = context.clients.get(&change.pid).unwrap();
            match change.after {
                ProcessState::Idle => {
                    // normal case or retransmission.
                    responses
                        .push(Message::new(MessageType::Ok, c.last_mid, change.pid, None).unwrap());
                }
                ProcessState::SkipPrev => {
                    responses
                        .push(Message::new(MessageType::Ok, c.last_mid, change.pid, None).unwrap());
                }
                ProcessState::Running => {
                    responses
                        .push(Message::new(MessageType::Ok, c.last_mid, change.pid, None).unwrap());
                }
                _ => {
                    warn!("{}: invalid state change by start {:?}", LOG_TAG, change);
                }
            }
        }
        Ok(responses)
    }

    fn on_client_exit(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!(
            "{}: on_client_exit@{} id={:03}",
            LOG_TAG, message.mid, message.cid
        );
        return self.handle_client_exit(context, message);
    }

    fn on_shutdown(&self, context: &mut ManagerContext) -> EventResult {
        trace!("{}: on_shutdown", LOG_TAG);
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
