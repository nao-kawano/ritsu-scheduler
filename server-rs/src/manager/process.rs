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

pub trait ManagerProc {
    fn enter_state(&self, context: &mut ManagerContext);
    // from cycle.
    fn on_cycle_start(&self, context: &mut ManagerContext, cycle: u64) -> EventResult;
    // from client.
    fn on_client_join(&self, context: &mut ManagerContext, message: &Message) -> EventResult;
    fn on_client_ready(&self, context: &mut ManagerContext, message: &Message) -> EventResult;
    fn on_client_done(&self, context: &mut ManagerContext, message: &Message) -> EventResult;
    fn on_client_exit(&self, context: &mut ManagerContext, message: &Message) -> EventResult;
    // from main.
    fn on_shutdown(&self, context: &mut ManagerContext) -> EventResult;

    // common process.
    fn handle_client_exit(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        // check manager state.
        match context.state {
            ManagerState::Starting => { /* ok */ }
            ManagerState::Running => { /* ok */ }
            _ => return Ok(vec![]),
        }
        // check client state.
        let client = context.clients.get_mut(&message.client_id).unwrap();
        if client.state == ClientState::None {
            warn!(
                "client {} is already disconnected, dropped.",
                message.client_id
            );
            return Ok(vec![]);
        }
        // send ok to trigger client.
        let mut responses: Vec<Message> = Vec::new();
        client.set_client_state(ClientState::None);
        context.num_active_clients -= 1;
        responses.push(Message::new(MessageType::Ok, message.client_id, None).unwrap());
        // send exit to ready clients.
        self.going_to_exit(context, &mut responses);
        //
        Ok(responses)
    }

    fn handle_shutdown(&self, context: &mut ManagerContext) -> EventResult {
        // check manager state.
        match context.state {
            ManagerState::Starting => { /* ok */ }
            ManagerState::Running => { /* ok */ }
            _ => return Ok(vec![]),
        }
        // send exit to ready clients.
        let mut responses: Vec<Message> = Vec::new();
        self.going_to_exit(context, &mut responses);
        //
        Ok(responses)
    }

    fn going_to_exit(&self, context: &mut ManagerContext, responses: &mut Vec<Message>) {
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
    }
}
