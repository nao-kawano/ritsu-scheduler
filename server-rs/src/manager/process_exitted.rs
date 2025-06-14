//!
//! Manager state.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use dps_message::Message;

use super::EventResult;
use super::context::ManagerContext;
use super::process::ManagerProc;

const LOG_TAG: &str = "StateExitted";

pub struct ManagerProcExitted;
impl ManagerProc for ManagerProcExitted {

    fn enter_state(&self, context: &mut ManagerContext) {
        trace!("{}: enter_state", LOG_TAG);
    }

    fn on_cycle_start(&self, _context: &mut ManagerContext, _cycle: u64) -> EventResult {
        return Err("already exited, drop cycle_start".to_string());
    }

    fn on_client_join(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        return Err(format!("already exitted, drop {:?}", message));
    }

    fn on_client_ready(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        return Err(format!("already exited, drop {:?}", message));
    }

    fn on_client_done(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        return Err(format!("already exited, drop {:?}", message));
    }

    fn on_client_exit(&self, _context: &mut ManagerContext, message: &Message) -> EventResult {
        return Err(format!("already exited, drop {:?}", message));
    }

    fn on_shutdown(&self, _context: &mut ManagerContext) -> EventResult {
        return Err("already exited, drop shutdown".to_string());
    }
}
