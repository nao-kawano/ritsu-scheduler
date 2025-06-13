//!
//! Manager state.
//!

use dps_message::Message;

use super::EventResult;
use super::context::ManagerContext;

pub trait ManagerProc {
    fn enter_state(&self, context: &mut ManagerContext);
    // from cycle.
    fn on_cycle_start(&self, context: &mut ManagerContext, cycle: u64) -> EventResult;
    // from client.
    fn on_client_join(&self, context: &mut ManagerContext, message: &Message) -> EventResult;
    fn on_client_ready(&self, context: &mut ManagerContext, message: &Message) -> EventResult;
    fn on_client_done(&self, context: &mut ManagerContext, message: &Message) -> EventResult;
    fn on_client_exit(&self, context: &mut ManagerContext, message: &Message) -> EventResult;
    // from main.
    fn on_shutdown(&self, context: &mut ManagerContext) -> EventResult;
}
