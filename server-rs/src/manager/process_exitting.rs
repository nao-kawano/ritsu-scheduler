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

const LOG_TAG: &str = "StateExitting";

pub struct ManagerProcExitting;
impl ManagerProc for ManagerProcExitting {
    fn enter_state(&self, context: &mut ManagerContext) {
        // TODO: implements.
        trace!("{}: enter_state\n", LOG_TAG);
    }

    fn on_cycle_start(&self, context: &mut ManagerContext, _cycle: u64) -> EventResult {
        // TODO: implements.
        trace!("{}: on_cycle_start\n", LOG_TAG);
        Ok(vec![])
    }

    fn on_client_join(&self, context: &mut ManagerContext, _message: &Message) -> EventResult {
        // TODO: implements.
        trace!("{}: on_client_join\n", LOG_TAG);
        Ok(vec![])
    }

    fn on_client_ready(&self, context: &mut ManagerContext, _message: &Message) -> EventResult {
        // TODO: implements.
        trace!("{}: on_client_ready\n", LOG_TAG);
        Ok(vec![])
    }

    fn on_client_done(&self, context: &mut ManagerContext, _message: &Message) -> EventResult {
        // TODO: implements.
        trace!("{}: on_client_done\n", LOG_TAG);
        Ok(vec![])
    }

    fn on_client_exit(&self, context: &mut ManagerContext, _message: &Message) -> EventResult {
        // TODO: implements.
        trace!("{}: on_client_exit\n", LOG_TAG);
        Ok(vec![])
    }

    fn on_shutdown(&self, context: &mut ManagerContext) -> EventResult {
        // TODO: implements.
        trace!("{}: on_shutdown\n", LOG_TAG);
        Ok(vec![])
    }
}
