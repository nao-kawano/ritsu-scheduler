//!
//! Manager state.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
const LOG_TAG: &str = "StateStarting";

use dps_message::{Message, MessageType};

use super::EventResult;
use super::ManagerState;
use super::context::{ClientState, ManagerContext};
use super::process::ManagerProc;

#[cfg(test)]
#[path = "process_starting_test.rs"]
mod process_starting_test;

/* -------------------------------------------------------------------------- */

pub struct ManagerProcStarting;
impl ManagerProc for ManagerProcStarting {
    fn enter_state(&self, context: &mut ManagerContext) {
        trace!("{}: enter_state", LOG_TAG);
        context.graph.reset_state();
    }

    fn on_cycle_start(&self, _context: &mut ManagerContext, _cycle: u64) -> EventResult {
        trace!("{}: on_cycle_start (nop)", LOG_TAG);
        Ok(vec![])
    }

    fn on_client_join(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("{}: on_client_join id={}", LOG_TAG, message.cid);
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.cid) {
            if client.state == ClientState::None {
                debug!("{}: client {} is joined", LOG_TAG, message.cid);
                client.set_client_state(ClientState::Sync);
                context.num_active_clients += 1;
                responses.push(Message::new(MessageType::Ok, message.cid, None).unwrap());
            } else {
                warn!(
                    "{}: client {} is already joined, dropped.",
                    LOG_TAG, message.cid
                );
            }
        }
        Ok(responses)
    }

    fn on_client_ready(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("{}: on_client_ready id={}", LOG_TAG, message.cid);
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.cid) {
            if client.state == ClientState::Sync {
                debug!("{}: client {} is ready", LOG_TAG, message.cid);
                client.set_client_state(ClientState::Active);
                responses.push(Message::new(MessageType::Ok, message.cid, None).unwrap());
                // also update graph.
                let r = context.graph.on_ready(message.cid);
                if let Err(e) = r {
                    return Err(e);
                }
            } else {
                warn!(
                    "{}: client {} is not in Idle, dropped.",
                    LOG_TAG, message.cid
                );
            }
        }
        // check if all clients are ready.
        if context.num_active_clients == context.clients.len() {
            let num_ready: u16 = context
                .clients
                .values()
                .fold(0, |sum, x| sum + ((x.state == ClientState::Active) as u16));
            if num_ready == context.clients.len() as u16 {
                context.set_state(ManagerState::Running);
            }
        }
        Ok(responses)
    }

    fn on_client_done(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        warn!("{}: on_client_done id={} (nop)", LOG_TAG, message.cid);
        Ok(vec![])
    }

    fn on_client_exit(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("{}: on_client_exit id={}", LOG_TAG, message.cid);
        return self.handle_client_exit(context, message);
    }

    fn on_shutdown(&self, context: &mut ManagerContext) -> EventResult {
        trace!("{}: on_shutdown", LOG_TAG);
        return self.handle_shutdown(context);
    }
}
