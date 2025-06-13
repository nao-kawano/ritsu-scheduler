//!
//! Manager state.
//!

use dps_message::Message;

use super::EventResult;
use super::context::ManagerContext;
use super::process::ManagerProc;

pub struct ManagerProcExitted;
impl ManagerProc for ManagerProcExitted {
    fn enter_state(&self, context: &mut ManagerContext) {
        print!("{:?} enter_state\n", context.state);
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
