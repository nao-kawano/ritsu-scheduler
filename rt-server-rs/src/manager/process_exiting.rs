// Copyright 2026 Naoyuki Kawano
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// =============================================================================
//!
//! Manager state.
//!

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
    fn enter_state(&self, context: &mut ManagerContext) {
        trace!("enter_state");
        if context.stats.exiting_start_at.is_none() {
            context.stats.exiting_start_at = Some(std::time::Instant::now());
        }
    }

    fn on_cycle_start(&self, _context: &mut ManagerContext, _global_cycle: u64) -> EventResult {
        trace!("on_cycle_start (nop)");
        Ok(vec![])
    }

    fn on_client_join(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_join CID:{:03} MID:{}", message.cid, message.mid);
        let mut responses: Vec<Message> = Vec::new();
        responses.push(
            Message::new(
                MessageType::Error,
                message.mid,
                message.cid,
                context.exit_reason.clone(),
            )
            .unwrap(),
        );
        Ok(responses)
    }

    fn on_client_ready(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_ready CID:{:03} MID:{}", message.cid, message.mid);
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.cid) {
            if client.state != ClientState::None {
                debug!(
                    "<STAT> CYC:{:012} CID:{:03} MID:{} READY (Exiting)",
                    context.running_cycle, message.cid, message.mid
                );
                client.set_client_state(ClientState::Exiting, context.running_cycle);
                responses.push(
                    Message::new(
                        MessageType::Error,
                        client.last_mid,
                        message.cid,
                        context.exit_reason.clone(),
                    )
                    .unwrap(),
                );
            } else {
                warn!(
                    "CYC:{:012} CID:{:03} MID:{} is disconnected, dropped READY.",
                    context.running_cycle, message.cid, message.mid
                );
            }
        }
        Ok(responses)
    }

    fn on_client_done(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!(
            "on_client_done CID:{:03} MID:{} (no-op)",
            message.cid, message.mid
        );
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.cid) {
            if client.state != ClientState::None {
                debug!(
                    "<STAT> CYC:{:012} CID:{:03} MID:{} DONE (Exiting)",
                    context.running_cycle, message.cid, message.mid
                );
                // Always return OK for DONE. Return ERROR on the next READY.
                responses.push(
                    Message::new(MessageType::Ok, client.last_mid, message.cid, None).unwrap(),
                );
            } else {
                warn!(
                    "CYC:{:012} CID:{:03} MID:{} is disconnected, dropped DONE.",
                    context.running_cycle, message.cid, message.mid
                );
            }
        }
        Ok(responses)
    }

    fn on_client_exit(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        trace!("on_client_exit CID:{:03} MID:{}", message.cid, message.mid);
        let mut responses: Vec<Message> = Vec::new();
        // update client state.
        if let Some(client) = context.clients.get_mut(&message.cid) {
            match client.state {
                ClientState::None => {
                    // maybe retransmission.
                    warn!(
                        "<STAT> CYC:{:012} CID:{:03} MID:{} EXIT (Retransmit)",
                        context.running_cycle, message.cid, message.mid
                    );
                    responses.push(
                        Message::new(MessageType::Ok, client.last_mid, message.cid, None).unwrap(),
                    );
                }
                ClientState::Exiting => {
                    debug!(
                        "<STAT> CYC:{:012} CID:{:03} MID:{} EXIT",
                        context.running_cycle, message.cid, message.mid
                    );
                    client.set_client_state(ClientState::None, context.running_cycle);
                    context.num_active_clients -= 1;
                    responses.push(
                        Message::new(MessageType::Ok, client.last_mid, message.cid, None).unwrap(),
                    );
                }
                ClientState::Active | ClientState::Sync => {
                    warn!(
                        "<STAT> CYC:{:012} CID:{:03} MID:{} EXIT (Unexpected)",
                        context.running_cycle, message.cid, message.mid
                    );
                    client.set_client_state(ClientState::None, context.running_cycle);
                    context.num_active_clients -= 1;
                    responses.push(
                        Message::new(MessageType::Ok, client.last_mid, message.cid, None).unwrap(),
                    );
                }
            }
        }
        // check if all clients are ready.
        if context.num_active_clients == 0 {
            debug!("all client is exit, go to exited");
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
