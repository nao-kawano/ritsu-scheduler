//!
//! Manager state.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
const LOG_TAG: &str = "StateExited";

use rt_message::{Message, MessageType};

use super::EventResult;
use super::context::ManagerContext;
use super::process::ManagerProc;

#[cfg(test)]
#[path = "process_exited_test.rs"]
mod process_exited_test;

/* -------------------------------------------------------------------------- */

pub struct ManagerProcExited;
impl ManagerProc for ManagerProcExited {
    fn enter_state(&self, _context: &mut ManagerContext) {
        trace!("{}: enter_state", LOG_TAG);
    }

    fn on_cycle_start(&self, _context: &mut ManagerContext, _cycle: u64) -> EventResult {
        return Err("already exited, drop cycle_start".to_string());
    }

    fn on_client_join(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        return Err(format!("already exited, drop {:?}", message));
    }

    fn on_client_ready(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        return Err(format!("already exited, drop {:?}", message));
    }

    fn on_client_done(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        return Err(format!("already exited, drop {:?}", message));
    }

    fn on_client_exit(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        warn!("{}: client {:03} retransmit exit", LOG_TAG, message.cid);
        return Ok(vec![
            Message::new(MessageType::Ok, message.mid, message.cid, None).unwrap(),
        ]);
    }

    fn on_shutdown(&self, _context: &mut ManagerContext) -> EventResult {
        return Err("already exited, drop shutdown".to_string());
    }
}
