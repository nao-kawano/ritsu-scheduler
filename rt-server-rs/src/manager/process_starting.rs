//!
//! Manager state.
//!

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use rt_message::{Message, MessageType, PROTOCOL_VERSION};

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

    fn on_cycle_start(&self, _context: &mut ManagerContext, _global_cycle: u64) -> EventResult {
        trace!("on_cycle_start (nop)");
        Ok(vec![])
    }

    fn on_client_join(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_join CID:{:03} MID:{}", message.cid, message.mid);
        let mut responses: Vec<Message> = Vec::new();

        // check protocol version
        if let Some(version) = message.get_extra("version") {
            if version != PROTOCOL_VERSION {
                warn!(
                    "CYC:{:012} CID:{:03} MID:{} Incompatible version: {}",
                    context.cycle_current, message.cid, message.mid, version
                );
                responses.push(
                    Message::new(
                        MessageType::Error,
                        message.mid,
                        message.cid,
                        Some(vec![(
                            "reason".to_string(),
                            "IncompatibleVersion".to_string(),
                        )]),
                    )
                    .unwrap(),
                );
                return Ok(responses);
            }
        } else {
            warn!(
                "CYC:{:012} CID:{:03} MID:{} Missing version extra",
                context.cycle_current, message.cid, message.mid
            );
            responses.push(
                Message::new(
                    MessageType::Error,
                    message.mid,
                    message.cid,
                    Some(vec![(
                        "reason".to_string(),
                        "IncompatibleVersion".to_string(),
                    )]),
                )
                .unwrap(),
            );
            return Ok(responses);
        }

        // update client state.
        if let Some(client) = context.clients.get_mut(&message.cid) {
            match client.state {
                ClientState::None => {
                    debug!(
                        "<STAT> CYC:{:012} CID:{:03} MID:{} JOIN",
                        context.cycle_current, message.cid, message.mid
                    );
                    client.set_client_state(ClientState::Sync, context.cycle_current);
                    context.num_active_clients += 1;
                    responses.push(
                        Message::new(
                            MessageType::Joined,
                            message.mid,
                            message.cid,
                            Some(vec![("version".to_string(), PROTOCOL_VERSION.to_string())]),
                        )
                        .unwrap(),
                    );
                }
                ClientState::Sync => {
                    // maybe retransmission, send JOINED.
                    warn!(
                        "<STAT> CYC:{:012} CID:{:03} MID:{} JOIN (Retransmit)",
                        context.cycle_current, message.cid, message.mid
                    );
                    responses.push(
                        Message::new(
                            MessageType::Joined,
                            message.mid,
                            message.cid,
                            Some(vec![("version".to_string(), PROTOCOL_VERSION.to_string())]),
                        )
                        .unwrap(),
                    );
                }
                _ => {
                    warn!(
                        "CYC:{:012} CID:{:03} MID:{} is already joined, dropped.",
                        context.cycle_current, message.cid, message.mid
                    );
                }
            }
        }
        Ok(responses)
    }

    fn on_client_ready(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_ready CID:{:03} MID:{}", message.cid, message.mid);
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.cid) {
            match client.state {
                ClientState::Sync => {
                    client.set_client_state(ClientState::Active, context.cycle_current);
                    // holding response for waiting others and trigger.
                    // update graph.
                    let r = context.sched.on_ready(message.cid);
                    match r {
                        Ok(changes) => {
                            for change in changes {
                                debug!(
                                    "<STAT> CYC:{:012} CID:{:03} MID:{} {:?} -> {:?}",
                                    context.cycle_current,
                                    change.cid,
                                    message.mid,
                                    change.before,
                                    change.after
                                );
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
                ClientState::Active => {
                    // maybe retransmission, keep waiting others.
                    debug!(
                        "<STAT> CYC:{:012} CID:{:03} MID:{} READY (Retransmit)",
                        context.cycle_current, message.cid, message.mid
                    );
                }
                _ => {
                    warn!(
                        "CYC:{:012} CID:{:03} MID:{} is not in Sync, dropped.",
                        context.cycle_current, message.cid, message.mid
                    );
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
                debug!("all client is ready, go to running");
                context.set_state(ManagerState::Running);
            }
        }
        Ok(vec![])
    }

    fn on_client_done(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!(
            "on_client_done CID:{:03} MID:{} (no-op)",
            message.cid, message.mid
        );
        Ok(vec![])
    }

    fn on_client_exit(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_exit CID:{:03} MID:{}", message.cid, message.mid);
        return self.handle_client_exit(context, message);
    }

    fn on_shutdown(&self, context: &mut ManagerContext) -> EventResult {
        trace!("on_shutdown");
        return self.handle_shutdown(context);
    }
}
