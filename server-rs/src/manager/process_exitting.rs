//!
//! Manager state.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
const LOG_TAG: &str = "StateExitting";

use dps_message::{Message, MessageType};

use super::EventResult;
use super::ManagerState;
use super::context::{ClientState, ManagerContext};
use super::process::ManagerProc;

#[cfg(test)]
#[path = "process_exitting_test.rs"]
mod process_exitting_test;

/* -------------------------------------------------------------------------- */

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
        trace!("{}: on_client_join id={:03}", LOG_TAG, message.cid);
        Err(format!("invalid Join from {:03}", message.cid))
    }

    fn on_client_ready(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("{}: on_client_ready id={:03}", LOG_TAG, message.cid);
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.cid) {
            if client.state != ClientState::None {
                info!(
                    "{}: send error to client {:03} for ready",
                    LOG_TAG, message.cid
                );
                client.set_client_state(ClientState::Exitting);
                responses.push(Message::new(MessageType::Error, message.cid, None).unwrap());
            } else {
                warn!(
                    "{}: client {:03} is disconnected, dropped.",
                    LOG_TAG, message.cid
                );
            }
        }
        Ok(responses)
    }

    fn on_client_done(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        warn!("{}: on_client_done id={:03} (nop)", LOG_TAG, message.cid);
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.cid) {
            if client.state != ClientState::None {
                info!(
                    "{}: send error to client {:03} for done",
                    LOG_TAG, message.cid
                );
                client.set_client_state(ClientState::Exitting);
                responses.push(Message::new(MessageType::Error, message.cid, None).unwrap());
            } else {
                warn!(
                    "{}: client {:03} is disconnected, dropped.",
                    LOG_TAG, message.cid
                );
            }
        }
        Ok(responses)
    }

    fn on_client_exit(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("{}: on_client_exit id={:03}", LOG_TAG, message.cid);
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.cid) {
            if client.state == ClientState::Exitting {
                info!("{}: client {:03} is exit", LOG_TAG, message.cid);
                client.set_client_state(ClientState::None);
                context.num_active_clients -= 1;
                responses.push(Message::new(MessageType::Ok, message.cid, None).unwrap());
            } else {
                warn!(
                    "{}: client {:03} is not in Exiting, dropped.",
                    LOG_TAG, message.cid
                );
            }
        }
        // check if all clients are ready.
        if context.num_active_clients == 0 {
            info!("{}: all client is exit, go to exitted", LOG_TAG);
            context.set_state(ManagerState::Exitted);
        }
        Ok(responses)
    }

    fn on_shutdown(&self, _context: &mut ManagerContext) -> EventResult {
        trace!("{}: on_shutdown", LOG_TAG);
        // keep going.
        Ok(vec![])
    }
}
