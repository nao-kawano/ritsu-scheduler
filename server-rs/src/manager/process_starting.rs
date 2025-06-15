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
#[path = "process_starting_test.rs"]
mod process_starting_test;

/* -------------------------------------------------------------------------- */

const LOG_TAG: &str = "StateStarting";

pub struct ManagerProcStarting;
impl ManagerProc for ManagerProcStarting {
    fn enter_state(&self, _context: &mut ManagerContext) {
        trace!("{}: enter_state", LOG_TAG);
    }

    fn on_cycle_start(&self, _context: &mut ManagerContext, _cycle: u64) -> EventResult {
        trace!("{}: on_cycle_start (nop)", LOG_TAG);
        Ok(vec![])
    }

    fn on_client_join(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("{}: on_client_join id={}", LOG_TAG, message.client_id);
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.client_id) {
            if client.state == ClientState::None {
                debug!("{}: client {} is joined", LOG_TAG, message.client_id);
                client.set_client_state(ClientState::Idle);
                context.num_active_clients += 1;
                responses.push(Message::new(MessageType::Ok, message.client_id, None).unwrap());
            } else {
                warn!(
                    "{}: client {} is already joined, dropped.",
                    LOG_TAG, message.client_id
                );
            }
        }
        Ok(responses)
    }

    fn on_client_ready(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("{}: on_client_ready id={}", LOG_TAG, message.client_id);
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.client_id) {
            if client.state == ClientState::Idle {
                debug!("{}: client {} is ready", LOG_TAG, message.client_id);
                client.set_client_state(ClientState::Ready);
                responses.push(Message::new(MessageType::Ok, message.client_id, None).unwrap());
            } else {
                warn!(
                    "{}: client {} is not in Idle, dropped.",
                    LOG_TAG, message.client_id
                );
            }
        }
        // check if all clients are ready.
        if context.num_active_clients == context.clients.len() {
            let num_ready: u16 = context
                .clients
                .values()
                .fold(0, |sum, x| sum + ((x.state == ClientState::Ready) as u16));
            if num_ready == context.clients.len() as u16 {
                context.set_state(ManagerState::Running);
            }
        }
        Ok(responses)
    }

    fn on_client_done(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        warn!("{}: on_client_done id={} (nop)", LOG_TAG, message.client_id);
        Ok(vec![])
    }

    fn on_client_exit(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("{}: on_client_exit id={}", LOG_TAG, message.client_id);
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.client_id) {
            if client.state != ClientState::None {
                debug!("{}: client {} is exitted", LOG_TAG, message.client_id);
                client.set_client_state(ClientState::None);
                context.num_active_clients -= 1;
                responses.push(Message::new(MessageType::Ok, message.client_id, None).unwrap());
            } else {
                warn!(
                    "{}: client {} is already disconnected, dropped.",
                    LOG_TAG, message.client_id
                );
            }
        }
        // send "ERROR" to ready clients for exit.
        let responses2 = self.going_to_exit(context);
        responses.extend(responses2);
        //
        Ok(responses)
    }

    fn on_shutdown(&self, context: &mut ManagerContext) -> EventResult {
        trace!("{}: on_shutdown", LOG_TAG);
        let responses = self.going_to_exit(context);
        return Ok(responses);
    }
}

impl ManagerProcStarting {
    // -----
    // private methods.

    fn going_to_exit(&self, context: &mut ManagerContext) -> Vec<Message> {
        let mut responses: Vec<Message> = Vec::new();
        // send "ERROR" to ready clients.
        if context.num_active_clients == 0 {
            debug!("no clients connected, go to exitted");
            context.set_state(ManagerState::Exitted);
        } else {
            debug!(
                "{} clients connected, go to exitting",
                context.num_active_clients
            );
            for client in context.clients.values_mut() {
                if client.state == ClientState::Ready {
                    responses.push(
                        Message::new(MessageType::Error, client.config.client_id, None).unwrap(),
                    );
                    client.set_client_state(ClientState::Exitting);
                }
            }
            context.set_state(ManagerState::Exitting);
        }
        return responses;
    }
}
