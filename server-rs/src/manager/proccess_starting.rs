//!
//! Manager state.
//!

use dps_message::{Message, MessageType};

use super::EventResult;
use super::ManagerState;
use super::client_status::ClientState;
use super::context::ManagerContext;
use super::process::ManagerProc;

pub struct ManagerProcStarting;
impl ManagerProc for ManagerProcStarting {
    fn enter_state(&self, context: &mut ManagerContext) {
        // TODO: implements.
        print!("{:?} enter_state\n", context.state);
    }

    fn on_cycle_start(&self, context: &mut ManagerContext, _cycle: u64) -> EventResult {
        // TODO: implements.
        print!("{:?} on_cycle_start\n", context.state);
        Ok(vec![])
    }

    fn on_client_join(&self, context: &mut ManagerContext, _message: &Message) -> EventResult {
        // TODO: implements.
        print!("{:?} on_client_join\n", context.state);
        Ok(vec![])
    }

    fn on_client_ready(&self, context: &mut ManagerContext, _message: &Message) -> EventResult {
        // TODO: implements.
        print!("{:?} on_client_ready\n", context.state);
        Ok(vec![])
    }

    fn on_client_done(&self, context: &mut ManagerContext, _message: &Message) -> EventResult {
        // TODO: implements.
        print!("{:?} on_client_done\n", context.state);
        Ok(vec![])
    }

    fn on_client_exit(&self, context: &mut ManagerContext, _message: &Message) -> EventResult {
        // TODO: implements.
        print!("{:?} on_client_exit\n", context.state);
        Ok(vec![])
    }

    fn on_shutdown(&self, context: &mut ManagerContext) -> EventResult {
        // TODO: implements.
        print!("{:?} on_shutdown\n", context.state);
        let mut responses: Vec<Message> = Vec::new();
        // send "ERROR" to ready clients.
        if context.num_active_clients == 0 {
            print!("{:?} no clients connected, go to exitted\n", context.state);
            context.set_state(ManagerState::Exitted);
        } else {
            print!(
                "{:?} {} clients connected, go to exitting\n",
                context.state, context.num_active_clients
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
