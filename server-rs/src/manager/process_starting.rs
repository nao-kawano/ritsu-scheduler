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

const LOG_TAG: &str = "StateStarting";

pub struct ManagerProcStarting;
impl ManagerProc for ManagerProcStarting {
    fn enter_state(&self, context: &mut ManagerContext) {
        // TODO: implements.
        trace!("{}: enter_state", LOG_TAG);
    }

    fn on_cycle_start(&self, context: &mut ManagerContext, _cycle: u64) -> EventResult {
        // TODO: implements.
        trace!("{}: on_cycle_start", LOG_TAG);
        Ok(vec![])
    }

    fn on_client_join(&self, context: &mut ManagerContext, _message: &Message) -> EventResult {
        // TODO: implements.
        trace!("{}: on_client_join", LOG_TAG);
        Ok(vec![])
    }

    fn on_client_ready(&self, context: &mut ManagerContext, _message: &Message) -> EventResult {
        // TODO: implements.
        trace!("{}: on_client_ready", LOG_TAG);
        Ok(vec![])
    }

    fn on_client_done(&self, context: &mut ManagerContext, _message: &Message) -> EventResult {
        // TODO: implements.
        trace!("{}: on_client_done", LOG_TAG);
        Ok(vec![])
    }

    fn on_client_exit(&self, context: &mut ManagerContext, _message: &Message) -> EventResult {
        // TODO: implements.
        trace!("{}: on_client_exit", LOG_TAG);
        Ok(vec![])
    }

    fn on_shutdown(&self, context: &mut ManagerContext) -> EventResult {
        // TODO: implements.
        trace!("{}: on_shutdown", LOG_TAG);
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
        Ok(responses)
    }
}
