//!
//! Manager state.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use dps_message::{Message, MessageType};

use super::EventResult;
use super::client_status::ClientState;
use super::context::ManagerContext;
use super::process::ManagerProc;
use crate::config::{ClientConfig, TriggerType};

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
        trace!("{}: on_cycle_start {:4}", LOG_TAG, cycle);
        let mut responses: Vec<Message> = Vec::new();
        // check and start trigger=cycle clients.
        for client_id in &context.graph_start {
            let client = context.clients.get_mut(client_id).unwrap();
            // check cycle.
            if self.is_target_cycle(context.cycle_current, &client.config) {
                // check state.
                if client.state == ClientState::Ready {
                    // start.
                    client.set_client_state(ClientState::Running { cycle });
                    responses.push(Message::new(MessageType::Ok, *client_id, None).unwrap());
                } else {
                    // TODO: still running, skip this cycle and notify all dependent clients.
                }
            }
        }
        Ok(responses)
    }

    fn on_client_join(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("{}: on_client_join id={}", LOG_TAG, message.client_id);
        Err(format!("invalid Join from {}", message.client_id))
    }

    fn on_client_ready(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("{}: on_client_ready id={}", LOG_TAG, message.client_id);
        let mut responses: Vec<Message> = Vec::new();
        if let Some(client) = context.clients.get_mut(&message.client_id) {
            if client.state == ClientState::Idle {
                // TODO: check skip state.
                client.set_client_state(ClientState::Ready);
                responses.push(Message::new(MessageType::Ok, message.client_id, None).unwrap());
            } else {
                warn!(
                    "{}: client {} is not in Idle, dropped.",
                    LOG_TAG, message.client_id
                );
            }
        }
        Ok(responses)
    }

    fn on_client_done(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("{}: on_client_done id={}", LOG_TAG, message.client_id);
        let client = context.clients.get_mut(&message.client_id).unwrap();
        let mut responses: Vec<Message> = Vec::new();
        if matches!(client.state, ClientState::Running { .. }) {
            client.set_client_state(ClientState::Idle);
            responses.push(Message::new(MessageType::Ok, message.client_id, None).unwrap());
            // TODO: check skip state. if no in skip state,
            self.notify_start(context, message.client_id, &mut responses);
        } else {
            warn!(
                "{}: client {} is not in Running, dropped.",
                LOG_TAG, message.client_id
            );
        }
        Ok(responses)
    }

    fn on_client_exit(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("{}: on_client_exit id={}", LOG_TAG, message.client_id);
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
        if let TriggerType::Cycle(target_cycle) = config.trigger_type {
            let target_cycle = target_cycle as u32;
            let target_cycle_offset = config.cycle_offset as u32;
            return cycle % target_cycle == target_cycle_offset;
        } else {
            return false;
        }
    }

    fn notify_start(
        &self,
        context: &mut ManagerContext,
        trigger: u16,
        responses: &mut Vec<Message>,
    ) {
        for next_client_id in context.graph_forward.get(&trigger).unwrap() {
            trace!("checking {} can start", next_client_id);
            // update dependency.
            let next_client = context.clients.get_mut(next_client_id).unwrap();
            next_client.update_depend(trigger);
            // check dependency.
            if next_client.is_depends_ok() {
                next_client.clear_depends();
                // check state.
                if next_client.state == ClientState::Ready {
                    // start.
                    debug!("start {}", next_client_id);
                    next_client.set_client_state(ClientState::Running {
                        cycle: context.cycle_current,
                    });
                    responses.push(Message::new(MessageType::Ok, *next_client_id, None).unwrap());
                } else {
                    // TODO: still running, skip this cycle and notify all dependent clients.
                }
            }
        }
    }
}
