//!
//! Manager state.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use dps_message::{Message, MessageType};

use super::EventResult;
use super::ManagerState;
use super::client_status::ClientState;
use super::context::ManagerContext;
use super::process::ManagerProc;

#[cfg(test)]
#[path = "process_exitting_test.rs"]
mod process_exitting_test;

/* -------------------------------------------------------------------------- */

const LOG_TAG: &str = "StateExitting";

pub struct ManagerProcExitting;
impl ManagerProc for ManagerProcExitting {
    fn enter_state(&self, _context: &mut ManagerContext) {
        trace!("{}: enter_state", LOG_TAG);
    }

    fn on_cycle_start(&self, _context: &mut ManagerContext, _cycle: u64) -> EventResult {
        trace!("{}: on_cycle_start (nop)", LOG_TAG);
        Ok(vec![])
    }

    fn on_client_join(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("{}: on_client_join id={}", LOG_TAG, message.client_id);
        Err(format!("invalid Join from {}", message.client_id))
    }

    fn on_client_ready(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("{}: on_client_ready id={}", LOG_TAG, message.client_id);
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.client_id) {
            if client.state == ClientState::Idle {
                info!(
                    "{}: send error to client {} ready",
                    LOG_TAG, message.client_id
                );
                client.set_client_state(ClientState::Exitting);
                responses.push(Message::new(MessageType::Error, message.client_id, None).unwrap());
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
        warn!("{}: on_client_done id={} (nop)", LOG_TAG, message.client_id);
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.client_id) {
            match client.state {
                ClientState::Running { .. } => {
                    info!(
                        "{}: send error to client {} done",
                        LOG_TAG, message.client_id
                    );
                    client.set_client_state(ClientState::Exitting);
                    responses
                        .push(Message::new(MessageType::Error, message.client_id, None).unwrap());
                }
                _ => {
                    warn!(
                        "{}: client {} is not in Running, dropped.",
                        LOG_TAG, message.client_id
                    );
                }
            }
        }
        Ok(responses)
    }

    fn on_client_exit(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("{}: on_client_exit id={}", LOG_TAG, message.client_id);
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.client_id) {
            if client.state == ClientState::Exitting {
                info!("{}: client {} is exit", LOG_TAG, message.client_id);
                client.set_client_state(ClientState::None);
                context.num_active_clients -= 1;
                responses.push(Message::new(MessageType::Ok, message.client_id, None).unwrap());
            } else {
                warn!(
                    "{}: client {} is not in Exiting, dropped.",
                    LOG_TAG, message.client_id
                );
            }
        }
        // check if all clients are ready.
        if context.num_active_clients == 0 {
            context.set_state(ManagerState::Exitted);
        }
        Ok(responses)
    }

    fn on_shutdown(&self, context: &mut ManagerContext) -> EventResult {
        trace!("{}: on_shutdown", LOG_TAG);
        // keep going.
        Ok(vec![])
    }
}
