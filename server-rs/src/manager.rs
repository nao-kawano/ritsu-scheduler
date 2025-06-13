//!
//! Handles event and control clients.
//!

mod client_status;
mod context;
mod proccess_starting;
mod process;
mod process_exitted;
mod process_exitting;
mod process_running;

use std::collections::HashMap;

use dps_message::{Message, MessageType};

use crate::Event;
use crate::config::ClientConfig;
use context::ManagerContext;
use proccess_starting::ManagerProcStarting;
use process::ManagerProc;
use process_exitted::ManagerProcExitted;
use process_exitting::ManagerProcExitting;
use process_running::ManagerProcRunning;

/* -------------------------------------------------------------------------- */

pub type EventResult = Result<Vec<Message>, String>;

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub enum ManagerState {
    Starting, // waiting for all client is Ready
    Running,  // running
    Exitting, // waiting for all client is Exit
    Exitted,  // inactive
}

/* -------------------------------------------------------------------------- */

pub struct EventManager {
    states: HashMap<ManagerState, Box<dyn ManagerProc>>,
    context: ManagerContext,
}

impl EventManager {
    pub fn new(client_configs: Vec<ClientConfig>) -> Self {
        let mut manager = EventManager {
            states: HashMap::new(),
            context: ManagerContext::new(client_configs),
        };
        // create manager state.
        manager
            .states
            .insert(ManagerState::Starting, Box::new(ManagerProcStarting));
        manager
            .states
            .insert(ManagerState::Running, Box::new(ManagerProcRunning));
        manager
            .states
            .insert(ManagerState::Exitting, Box::new(ManagerProcExitting));
        manager
            .states
            .insert(ManagerState::Exitted, Box::new(ManagerProcExitted));
        // enter initial state.
        if let Some(initial_state) = manager.states.get(&manager.context.state) {
            initial_state.enter_state(&mut manager.context);
        }
        return manager;
    }

    pub fn get_state(&self) -> ManagerState {
        return self.context.state;
    }

    pub fn process(&mut self, event: Event) -> EventResult {
        // get current processor.
        let Some(current_state) = self.states.get(&self.context.state) else {
            return Err(format!("state not found for {:?}", &self.context.state));
        };
        // process event.
        let result = match event {
            Event::Abort => current_state.on_shutdown(&mut self.context),
            Event::CycleStart(cycle) => current_state.on_cycle_start(&mut self.context, cycle),
            Event::ClientMsg(msg) => match msg.message_type {
                MessageType::Join => current_state.on_client_join(&mut self.context, &msg),
                MessageType::Ready => current_state.on_client_ready(&mut self.context, &msg),
                MessageType::Done => current_state.on_client_done(&mut self.context, &msg),
                MessageType::Exit => current_state.on_client_exit(&mut self.context, &msg),
                _ => Err("invalid message type".to_string()),
            },
        };
        // state change.
        if let Err(e) = self.change_state() {
            return Err(e);
        };
        //
        return result;
    }

    // -----
    // private methods.

    fn change_state(&mut self) -> Result<(), String> {
        if self.context.state_changed {
            self.context.state_changed = false;
            if let Some(next_state) = self.states.get(&self.context.state) {
                next_state.enter_state(&mut self.context);
            } else {
                return Err(format!("state not found for {:?}", &self.context.state));
            }
        }
        return Ok(());
    }
}
