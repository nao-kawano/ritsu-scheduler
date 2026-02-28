//!
//! Manager state.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
const LOG_TAG: &str = "StateCommon";

use rt_message::{Message, MessageType};

use super::EventResult;
use super::ManagerState;
use super::context::{ClientState, ManagerContext};

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
        let Some(client) = context.clients.get_mut(&message.cid) else {
            return Err(format!("client {:03} does not exist", message.cid));
        };
        if client.state == ClientState::None {
            warn!(
                "{}: client {:03} is already disconnected, maybe retransmission.",
                LOG_TAG, message.cid
            );
            return Ok(vec![
                Message::new(MessageType::Ok, message.mid, message.cid, None).unwrap(),
            ]);
        }
        // send ok to trigger client.
        let mut responses: Vec<Message> = Vec::new();
        client.set_client_state(ClientState::None);
        context.num_active_clients -= 1;
        responses.push(Message::new(MessageType::Ok, message.mid, message.cid, None).unwrap());
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
            info!("{}: no clients connected, go to exited", LOG_TAG);
            context.set_state(ManagerState::Exited);
        } else {
            info!(
                "{}: {} clients connected, go to exiting",
                LOG_TAG, context.num_active_clients
            );
            let ready_clients: Vec<u16> = context.sched.get_ready_processes();
            for ready_client in ready_clients {
                if let Some(client) = context.clients.get_mut(&ready_client) {
                    // exclude already exited clients.
                    if client.state != ClientState::None {
                        responses.push(
                            Message::new(
                                MessageType::Error,
                                client.last_mid,
                                client.config.client_id,
                                None,
                            )
                            .unwrap(),
                        );
                        client.set_client_state(ClientState::Exiting);
                    }
                }
            }
            context.set_state(ManagerState::Exiting);
        }
    }
}
