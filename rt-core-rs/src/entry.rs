//!
//! Process entry in graph.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

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
    pub(crate) dependency_statuses: Vec<(u16, bool)>,
    pub(crate) unmet_dependencies: usize,
    pub(crate) is_floating: bool,
}

impl ProcessEntry {
    pub fn new(cid: u16, depends_on: &Vec<u16>, is_floating: bool) -> Self {
        let statuses: Vec<(u16, bool)> = depends_on.iter().map(|x| (*x, false)).collect();
        let unmet_dependencies = statuses.len();
        ProcessEntry {
            cid,
            is_floating,
            state: ProcessState::Idle,
            dependency_statuses: statuses,
            unmet_dependencies,
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
            trace!("CID:{:03} {:?} -> {:?}", self.cid, self.state, new_state);
            self.state = new_state;
        } else {
            warn!(
                "CID:{:03} state change failed {:?} -> {:?}",
                self.cid, self.state, new_state
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
        self.unmet_dependencies == 0
    }

    /// Update the dependency status.
    pub(crate) fn mark_dependency_complete(&mut self, cid: u16) -> bool {
        for depend_value in self.dependency_statuses.iter_mut() {
            if depend_value.0 == cid {
                if depend_value.1 {
                    warn!("CID:{:03} deps already completed CID:{:03}", self.cid, cid);
                    return false;
                } else {
                    trace!("CID:{:03} deps complete CID:{:03}", self.cid, cid);
                    depend_value.1 = true;
                    if self.unmet_dependencies > 0 {
                        self.unmet_dependencies -= 1;
                    }
                    return true;
                }
            }
        }
        return false;
    }

    /// Clear the dependency status.
    pub(crate) fn reset_dependency_statuses(&mut self) {
        trace!("CID:{:03} deps reset", self.cid);
        self.unmet_dependencies = self.dependency_statuses.len();
        for depend_value in self.dependency_statuses.iter_mut() {
            depend_value.1 = false;
        }
    }
}
