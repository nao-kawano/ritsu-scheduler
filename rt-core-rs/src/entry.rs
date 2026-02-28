//!
//! Process entry in graph.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
const LOG_TAG: &str = "ProcessEntry";

use std::collections::HashMap;

#[cfg(test)]
#[path = "entry_test.rs"]
mod entry_test;

/* -------------------------------------------------------------------------- */

/// Represents the state of a process.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ProcessState {
    Ready,
    Running,
    Overrun,
    Idle,
    Skip,
    Late,
}

/// Represents the process.
#[derive(Debug, Clone)]
pub struct ProcessEntry {
    pub(crate) cid: u16,
    pub(crate) state: ProcessState,
    pub(crate) dependency_statuses: HashMap<u16, bool>,
    pub(crate) is_floating: bool,
}

impl ProcessEntry {
    pub fn new(cid: u16, depends_on: &Vec<u16>, is_floating: bool) -> Self {
        let statuses: HashMap<u16, bool> = depends_on.iter().map(|x| (*x, false)).collect();
        ProcessEntry {
            cid,
            is_floating,
            state: ProcessState::Idle,
            dependency_statuses: statuses,
        }
    }

    /// Reset state and dependency statuses.
    pub(crate) fn reset(&mut self) {
        self.state = ProcessState::Idle;
        self.reset_dependency_statuses();
    }

    /// Get the id of the process.
    #[allow(dead_code)]
    pub fn get_cid(&self) -> u16 {
        self.cid
    }

    /// Get the floating or not.
    #[allow(dead_code)]
    pub fn is_floating(&self) -> bool {
        self.is_floating
    }

    /// Get the state of the process.
    #[allow(dead_code)]
    pub fn get_state(&self) -> ProcessState {
        self.state
    }

    /// Set the state of the process.
    pub(crate) fn set_state(&mut self, new_state: ProcessState) -> bool {
        let ok_to_change = match self.state {
            ProcessState::Ready => match new_state {
                ProcessState::Running => true,
                ProcessState::Skip => true,
                _ => false,
            },
            ProcessState::Running => match new_state {
                ProcessState::Idle => true,
                ProcessState::Overrun => true,
                _ => false,
            },
            ProcessState::Overrun => match new_state {
                ProcessState::Late => true,
                _ => false,
            },
            ProcessState::Idle => match new_state {
                ProcessState::Ready => true,
                ProcessState::Late => true,
                _ => false,
            },
            ProcessState::Skip => match new_state {
                ProcessState::Ready => true,
                _ => false,
            },
            ProcessState::Late => match new_state {
                ProcessState::Idle => true,
                _ => false,
            },
        };
        if ok_to_change {
            debug!(
                "{}: [STAT] CID:{:03} {:?} -> {:?}",
                LOG_TAG, self.cid, self.state, new_state
            );
            self.state = new_state;
        } else {
            warn!(
                "{}: CID:{:03} state change failed {:?} -> {:?}",
                LOG_TAG, self.cid, self.state, new_state
            );
        }
        return ok_to_change;
    }

    /// Check if process has dependency.
    #[allow(dead_code)]
    pub fn is_dependent(&self) -> bool {
        self.dependency_statuses.len() > 0
    }

    /// Check if the process can start.
    pub fn is_dependency_met(&self) -> bool {
        self.dependency_statuses.iter().all(|x| *(x.1))
    }

    /// Update the dependency status.
    pub(crate) fn mark_dependency_complete(&mut self, cid: u16) -> bool {
        if let Some(depend_value) = self.dependency_statuses.get_mut(&cid) {
            if *depend_value {
                warn!(
                    "{}: CID:{:03} deps already completed CID:{:03}",
                    LOG_TAG, self.cid, cid
                );
                return false;
            } else {
                trace!(
                    "{}: CID:{:03} deps complete CID:{:03}",
                    LOG_TAG, self.cid, cid
                );
                *depend_value = true;
                return true;
            }
        } else {
            return false;
        }
    }

    /// Clear the dependency status.
    pub(crate) fn reset_dependency_statuses(&mut self) {
        trace!("{}: CID:{:03} deps reset", LOG_TAG, self.cid);
        for depend_value in self.dependency_statuses.values_mut() {
            *depend_value = false;
        }
    }
}
