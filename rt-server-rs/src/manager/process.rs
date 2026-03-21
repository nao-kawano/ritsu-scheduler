//!
//! Manager state.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use rt_message::{Message, MessageType};

use super::EventResult;
use super::ManagerState;
use super::context::{ClientState, ManagerContext};

pub trait ManagerProc {
    fn enter_state(&self, context: &mut ManagerContext);
    // from cycle.
    fn on_cycle_start(&self, context: &mut ManagerContext, global_cycle: u64) -> EventResult;
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
                "<STAT> CYC:{:05} CID:{:03} MID:{} EXIT (Retransmit)",
                context.cycle_current, message.cid, message.mid
            );
            return Ok(vec![
                Message::new(MessageType::Ok, message.mid, message.cid, None).unwrap(),
            ]);
        }
        // send ok to trigger client.
        debug!(
            "<STAT> CYC:{:05} CID:{:03} MID:{} EXIT",
            context.cycle_current, message.cid, message.mid
        );
        let mut responses: Vec<Message> = Vec::new();
        client.set_client_state(ClientState::None, context.cycle_current);
        context.num_active_clients -= 1;
        context.exit_reason = Some(vec![
            ("reason".to_string(), "ClientExit".to_string()),
            ("cid".to_string(), format!("{:03}", message.cid)),
        ]);
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
        context.exit_reason = Some(vec![("reason".to_string(), "Shutdown".to_string())]);
        self.going_to_exit(context, &mut responses);
        //
        Ok(responses)
    }

    fn going_to_exit(&self, context: &mut ManagerContext, responses: &mut Vec<Message>) {
        if context.num_active_clients == 0 {
            debug!(
                "CYC:{:05} no clients connected, go to Exited",
                context.cycle_current
            );
            context.set_state(ManagerState::Exited);
        } else {
            debug!(
                "CYC:{:05} {} clients connected, go to Exiting",
                context.cycle_current, context.num_active_clients
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
                                context.exit_reason.clone(),
                            )
                            .unwrap(),
                        );
                        client.set_client_state(ClientState::Exiting, context.cycle_current);
                    }
                }
            }
            context.set_state(ManagerState::Exiting);
        }
    }
}
