//!
//! Process status in graph.
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
}

/// Represents the process.
#[derive(Debug, Clone)]
pub struct ProcessEntry {
    pub(super) pid: u16,
    pub(super) state: ProcessState,
    pub(super) depends_on: HashMap<u16, bool>,
    pub(super) is_floating: bool,
}

impl ProcessEntry {
    pub fn new(pid: u16, dependency: &Vec<u16>, is_floating: bool) -> Self {
        let dependency: HashMap<u16, bool> = dependency.iter().map(|x| (*x, false)).collect();
        ProcessEntry {
            pid,
            is_floating,
            state: ProcessState::Idle,
            depends_on: dependency,
        }
    }

    /// Reset state and dependency.
    pub fn reset(&mut self) {
        self.state = ProcessState::Idle;
        self.clear_depends();
    }

    /// Get the id of the process.
    pub fn get_pid(&self) -> u16 {
        self.pid
    }

    /// Get the floating or not.
    pub fn is_floating(&self) -> bool {
        self.is_floating
    }

    /// Get the state of the process.
    pub fn get_state(&self) -> ProcessState {
        self.state
    }

    /// Set the state of the process.
    pub fn set_state(&mut self, new_state: ProcessState) -> bool {
        let ok_to_change = match self.state {
            ProcessState::Ready => match new_state {
                ProcessState::Idle => true,
                ProcessState::Running => true,
                _ => false,
            },
            ProcessState::Running => match new_state {
                ProcessState::Idle => true,
                ProcessState::Overrun => true,
                _ => false,
            },
            ProcessState::Overrun => match new_state {
                ProcessState::Idle => true,
                _ => false,
            },
            ProcessState::Idle => match new_state {
                ProcessState::Ready => true,
                ProcessState::Skip => true,
                _ => false,
            },
            ProcessState::Skip => match new_state {
                ProcessState::Idle => true,
                _ => false,
            },
        };
        if ok_to_change {
            info!(
                "{}: pid {:3}, state {:?} -> {:?}",
                LOG_TAG, self.pid, self.state, new_state
            );
            self.state = new_state;
        } else {
            warn!(
                "{}: pid {:3}, state change failed {:?} -> {:?}",
                LOG_TAG, self.pid, self.state, new_state
            );
        }
        return ok_to_change;
    }

    /// Check if process has dependency.
    pub fn has_depends(&self) -> bool {
        self.depends_on.len() > 0
    }

    /// Check if the process can start.
    pub fn is_depends_ok(&self) -> bool {
        self.depends_on.iter().all(|x| *(x.1))
    }

    /// Update the dependency status.
    pub fn update_depend(&mut self, pid: u16) -> bool {
        if let Some(depend_value) = self.depends_on.get_mut(&pid) {
            if *depend_value {
                warn!("{}: pid {:3}, already updated {:3}", LOG_TAG, self.pid, pid);
                return false;
            } else {
                trace!("{}: pid {:3}, update depend {:3}", LOG_TAG, self.pid, pid);
                *depend_value = true;
                return true;
            }
        } else {
            return false;
        }
    }

    /// Clear the dependency status.
    pub fn clear_depends(&mut self) {
        trace!("{}: pid {:3}, clear depend", LOG_TAG, self.pid);
        for depend_value in self.depends_on.values_mut() {
            *depend_value = false;
        }
    }
}
