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
#[path = "process_starting_test.rs"]
mod process_starting_test;

/* -------------------------------------------------------------------------- */

pub struct ManagerProcStarting;
impl ManagerProc for ManagerProcStarting {
    fn enter_state(&self, context: &mut ManagerContext) {
        trace!("enter_state");
        context.sched.reset_state();
    }

    fn on_cycle_start(&self, _context: &mut ManagerContext, _cycle: u64) -> EventResult {
        trace!("on_cycle_start (nop)");
        Ok(vec![])
    }

    fn on_client_join(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_join@{} CID:{:03}", message.mid, message.cid);
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.cid) {
            match client.state {
                ClientState::None => {
                    debug!("client CID:{:03} is joined", message.cid);
                    client.set_client_state(ClientState::Sync);
                    context.num_active_clients += 1;
                    responses.push(
                        Message::new(MessageType::Ok, message.mid, message.cid, None).unwrap(),
                    );
                }
                ClientState::Sync => {
                    // maybe retransmission, send OK.
                    warn!("client CID:{:03} retransmit join", message.cid);
                    responses.push(
                        Message::new(MessageType::Ok, message.mid, message.cid, None).unwrap(),
                    );
                }
                _ => {
                    warn!("client CID:{:03} is already joined, dropped.", message.cid);
                }
            }
        }
        Ok(responses)
    }

    fn on_client_ready(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_ready@{} CID:{:03}", message.mid, message.cid);
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.cid) {
            match client.state {
                ClientState::Sync => {
                    debug!("client CID:{:03} is ready", message.cid);
                    client.set_client_state(ClientState::Active);
                    // holding response for waiting others and trigger.
                    // update graph.
                    let r = context.sched.on_ready(message.cid);
                    if let Err(e) = r {
                        return Err(e);
                    }
                }
                ClientState::Active => {
                    // maybe retransmission, keep waiting others.
                    debug!("client CID:{:03} retransmit ready", message.cid);
                }
                _ => {
                    warn!("client CID:{:03} is not in Idle, dropped.", message.cid);
                }
            }
        }
        // check if all clients are ready.
        if context.num_active_clients == context.clients.len() {
            let num_ready: u16 = context
                .clients
                .values()
                .fold(0, |sum, x| sum + ((x.state == ClientState::Active) as u16));
            if num_ready == context.clients.len() as u16 {
                info!("all client is ready, go to running");
                context.set_state(ManagerState::Running);
            }
        }
        Ok(vec![])
    }

    fn on_client_done(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        warn!(
            "on_client_done@{} CID:{:03} (nop)",
            message.mid, message.cid
        );
        Ok(vec![])
    }

    fn on_client_exit(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_exit@{} CID:{:03}", message.mid, message.cid);
        return self.handle_client_exit(context, message);
    }

    fn on_shutdown(&self, context: &mut ManagerContext) -> EventResult {
        trace!("on_shutdown");
        return self.handle_shutdown(context);
    }
}
