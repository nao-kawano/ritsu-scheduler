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
use super::context::ManagerContext;
use super::process::ManagerProc;

#[cfg(test)]
#[path = "process_exited_test.rs"]
mod process_exited_test;

/* -------------------------------------------------------------------------- */

pub struct ManagerProcExited;
impl ManagerProc for ManagerProcExited {
    fn enter_state(&self, context: &mut ManagerContext) {
        trace!("enter_state");
        context.dump_stats(context.stats.last_global_cycle);
    }

    fn on_cycle_start(&self, _context: &mut ManagerContext, _global_cycle: u64) -> EventResult {
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

    fn on_client_exit(&self, context: &mut ManagerContext, message: &Message) -> EventResult {
        warn!(
            "<STAT> CYC:{:012} CID:{:03} MID:{} None -> None (Retransmit)",
            context.running_cycle, message.cid, message.mid
        );
        return Ok(vec![
            Message::new(MessageType::Ok, message.mid, message.cid, None).unwrap(),
        ]);
    }

    fn on_shutdown(&self, _context: &mut ManagerContext) -> EventResult {
        return Err("already exited, drop shutdown".to_string());
    }
}
