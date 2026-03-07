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
use super::process::ManagerProc;

#[cfg(test)]
#[path = "process_exiting_test.rs"]
mod process_exiting_test;

/* -------------------------------------------------------------------------- */

pub struct ManagerProcExiting;
impl ManagerProc for ManagerProcExiting {
    fn enter_state(&self, _context: &mut ManagerContext) {
        trace!("enter_state");
    }

    fn on_cycle_start(&self, _context: &mut ManagerContext, _cycle: u64) -> EventResult {
        trace!("on_cycle_start (nop)");
        Ok(vec![])
    }

    fn on_client_join(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_join@{} CID:{:03}", message.mid, message.cid);
        Err(format!("invalid Join from CID:{:03}", message.cid))
    }

    fn on_client_ready(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_ready@{} CID:{:03}", message.mid, message.cid);
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.cid) {
            if client.state != ClientState::None {
                info!("send error to client CID:{:03} for ready", message.cid);
                client.set_client_state(ClientState::Exiting);
                responses.push(
                    Message::new(MessageType::Error, client.last_mid, message.cid, None).unwrap(),
                );
            } else {
                warn!("client CID:{:03} is disconnected, dropped.", message.cid);
            }
        }
        Ok(responses)
    }

    fn on_client_done(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        warn!(
            "on_client_done@{} CID:{:03} (nop)",
            message.mid, message.cid
        );
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.cid) {
            if client.state != ClientState::None {
                info!("send error to client CID:{:03} for done", message.cid);
                client.set_client_state(ClientState::Exiting);
                responses.push(
                    Message::new(MessageType::Error, client.last_mid, message.cid, None).unwrap(),
                );
            } else {
                warn!("client CID:{:03} is disconnected, dropped.", message.cid);
            }
        }
        Ok(responses)
    }

    fn on_client_exit(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_exit@{} CID:{:03}", message.mid, message.cid);
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.cid) {
            match client.state {
                ClientState::None => {
                    // maybe retransmission.
                    warn!("client CID:{:03} retransmit exit", message.cid);
                    responses.push(
                        Message::new(MessageType::Ok, client.last_mid, message.cid, None).unwrap(),
                    );
                }
                ClientState::Exiting => {
                    info!("client CID:{:03} is exit", message.cid);
                    client.set_client_state(ClientState::None);
                    context.num_active_clients -= 1;
                    responses.push(
                        Message::new(MessageType::Ok, client.last_mid, message.cid, None).unwrap(),
                    );
                }
                _ => {
                    warn!("client CID:{:03} is not in Exiting, dropped.", message.cid);
                }
            }
        }
        // check if all clients are ready.
        if context.num_active_clients == 0 {
            info!("all client is exit, go to exited");
            context.set_state(ManagerState::Exited);
        }
        Ok(responses)
    }

    fn on_shutdown(&self, _context: &mut ManagerContext) -> EventResult {
        trace!("on_shutdown");
        // keep going.
        Ok(vec![])
    }
}
