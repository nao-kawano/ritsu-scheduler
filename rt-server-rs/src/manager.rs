//!
//! Handles event and control clients.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
const LOG_TAG: &str = "EventManager";

mod context;
mod process;
mod process_exited;
mod process_exiting;
mod process_running;
mod process_starting;

use std::collections::HashMap;
use std::hash::Hash;

use rt_message::{Message, MessageType};

use crate::Event;
use crate::config::ClientConfig;
use context::ManagerContext;
use process::ManagerProc;
use process_exited::ManagerProcExited;
use process_exiting::ManagerProcExiting;
use process_running::ManagerProcRunning;
use process_starting::ManagerProcStarting;

/* -------------------------------------------------------------------------- */

pub type EventResult = Result<Vec<Message>, String>;

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub enum ManagerState {
    Starting, // waiting for all client is Ready
    Running,  // running
    Exiting,  // waiting for all client is Exit
    Exited,   // inactive
}

/* -------------------------------------------------------------------------- */

pub struct EventManager {
    context: ManagerContext,
    procs: HashMap<ManagerState, Box<dyn ManagerProc>>,
}

impl EventManager {
    pub fn new(client_configs: Vec<ClientConfig>) -> Self {
        // create context.
        let mut context: ManagerContext = ManagerContext::new(client_configs);
        // create procs.
        let mut procs: HashMap<ManagerState, Box<dyn ManagerProc>> = HashMap::new();
        procs.insert(ManagerState::Starting, Box::new(ManagerProcStarting));
        procs.insert(ManagerState::Running, Box::new(ManagerProcRunning));
        procs.insert(ManagerState::Exiting, Box::new(ManagerProcExiting));
        procs.insert(ManagerState::Exited, Box::new(ManagerProcExited));
        // enter initial state.
        if let Some(initial_state) = procs.get(&context.state) {
            initial_state.enter_state(&mut context);
        }
        // return object.
        EventManager { context, procs }
    }

    pub fn get_state(&self) -> ManagerState {
        return self.context.state;
    }

    pub fn process(&mut self, event: Event) -> EventResult {
        trace!("{}: process {:?}", LOG_TAG, event);
        // get current processor.
        let Some(proc) = self.procs.get(&self.context.state) else {
            return Err(format!("state not found for {:?}", &self.context.state));
        };
        // process event.
        let result = match event {
            Event::Abort => proc.on_shutdown(&mut self.context),
            Event::CycleStart(cycle) => proc.on_cycle_start(&mut self.context, cycle),
            Event::ClientMsg(msg) => {
                if !self.context.clients.contains_key(&msg.cid) {
                    Err(format!(
                        "unknown client message type={:?}, CID:{:03}",
                        msg.mtype, msg.cid
                    ))
                } else {
                    self.context
                        .clients
                        .get_mut(&msg.cid)
                        .expect("client must be exist")
                        .last_mid = msg.mid;
                    match msg.mtype {
                        MessageType::Join => proc.on_client_join(&mut self.context, &msg),
                        MessageType::Ready => proc.on_client_ready(&mut self.context, &msg),
                        MessageType::Done => proc.on_client_done(&mut self.context, &msg),
                        MessageType::Exit => proc.on_client_exit(&mut self.context, &msg),
                        _ => Err(format!(
                            "not a client message type={:?}, CID:{:03}",
                            msg.mtype, msg.cid
                        )),
                    }
                }
            }
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
            if let Some(next_proc) = self.procs.get(&self.context.state) {
                next_proc.enter_state(&mut self.context);
            } else {
                return Err(format!("state not found for {:?}", &self.context.state));
            }
        }
        return Ok(());
    }
}
