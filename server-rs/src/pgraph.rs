//!
//! Process graph.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
const LOG_TAG: &str = "ProcessGraph";

mod entry;

use std::collections::{HashMap, HashSet};

use entry::ProcessEntry;
use entry::ProcessState;

#[cfg(test)]
#[path = "pgraph_test.rs"]
mod pgraph_test;

/* -------------------------------------------------------------------------- */

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProcessStateChange {
    pub pid: u16,
    pub before: ProcessState,
    pub after: ProcessState,
}

impl ProcessStateChange {
    pub fn new(entry: &ProcessEntry) -> Self {
        ProcessStateChange {
            pid: entry.pid,
            before: entry.state,
            after: entry.state,
        }
    }
}

/* -------------------------------------------------------------------------- */

pub struct ProcessGraph {
    entries: HashMap<u16, ProcessEntry>,
    graph_start: HashSet<u16>,
    graph_forward: HashMap<u16, HashSet<u16>>,
}

impl ProcessGraph {
    pub fn new(entries: HashMap<u16, ProcessEntry>) -> Self {
        let (graph_start, graph_forward) = ProcessGraph::create_graph(&entries);
        ProcessGraph {
            entries,
            graph_start,
            graph_forward,
        }
    }

    pub fn reset_state(&mut self) {
        trace!("{}: reset process state", LOG_TAG);
        for entry in self.entries.values_mut() {
            entry.reset(); // all process set to Idle.
        }
    }

    pub fn on_start(&mut self, pid: u16) -> Result<Vec<ProcessStateChange>, String> {
        trace!("{}: update pid {:3} by start", LOG_TAG, pid);
        if !self.graph_start.contains(&pid) {
            return Err(format!("process {:3} does not exist", pid));
        }
        // check if dependency is met.
        if let Some(entry) = self.entries.get_mut(&pid) {
            if !entry.is_depends_ok() {
                info!(
                    "{}: pid {:3} has dependency unmet, skip start",
                    LOG_TAG, pid
                );
                return Ok(vec![]);
            }
        }
        // check if all process is in Ready.
        let forwards =
            ProcessGraph::find_forward_all(pid, true, &self.entries, &self.graph_forward);
        let is_not_ready = forwards.iter().any(|pid| {
            self.entries
                .get(pid)
                .map_or(true, |entry| entry.state != ProcessState::Ready)
        });
        // if found not-ready, update to Skip.
        let mut changes = Vec::new();
        if is_not_ready {
            let mut forwards_set: HashSet<u16> = forwards.iter().cloned().collect();
            let running_forwards: Vec<u16> = forwards
                .iter()
                .filter(|pid| self.entries[pid].state == ProcessState::Running)
                .copied()
                .collect();
            // mark as Overrun and afters set to SkipPrev.
            for running_pid in running_forwards {
                // set target to Overrun.
                if let Some(entry) = self.entries.get_mut(&running_pid) {
                    let mut change: ProcessStateChange = ProcessStateChange::new(&entry);
                    if entry.set_state(ProcessState::Overrun) {
                        change.after = ProcessState::Overrun;
                        changes.push(change);
                    }
                    // remove updated.
                    forwards_set.remove(&running_pid);
                }
                // set afters to SkipPrev.
                let skip_forwards = ProcessGraph::find_forward_all(
                    running_pid,
                    false,
                    &self.entries,
                    &self.graph_forward,
                );
                for skip_pid in skip_forwards {
                    if let Some(entry) = self.entries.get_mut(&skip_pid) {
                        let mut change: ProcessStateChange = ProcessStateChange::new(&entry);
                        if entry.set_state(ProcessState::SkipPrev) {
                            entry.clear_depends();
                            change.after = ProcessState::SkipPrev;
                            changes.push(change);
                        }
                        // remove updated.
                        forwards_set.remove(&skip_pid);
                    }
                }
            }
            // All other (done) processes set to Skip.
            for skip_pid in forwards_set {
                // set target to Skip.
                if let Some(entry) = self.entries.get_mut(&skip_pid) {
                    let mut change: ProcessStateChange = ProcessStateChange::new(&entry);
                    match entry.state {
                        ProcessState::Idle => {
                            entry.set_state(ProcessState::SkipPrev);
                            change.after = ProcessState::SkipPrev;
                            changes.push(change);
                        }
                        ProcessState::Ready => {
                            entry.set_state(ProcessState::Skip);
                            change.after = ProcessState::Skip;
                            changes.push(change);
                        }
                        _ => {
                            warn!(
                                "{}: ignore skip for process {:3} in {:?}",
                                LOG_TAG, entry.pid, entry.state
                            );
                        }
                    };
                }
            }
        } else {
            if let Some(entry) = self.entries.get_mut(&pid) {
                // dependency cleared, start target.
                let mut change: ProcessStateChange = ProcessStateChange::new(&entry);
                if entry.set_state(ProcessState::Running) {
                    entry.clear_depends();
                    change.after = ProcessState::Running;
                    changes.push(change);
                }
            }
        }
        //
        if changes.len() > 0 {
            Ok(changes)
        } else {
            return Err(format!(
                "process {:3} likely the second skip has occurred.",
                pid
            ));
        }
    }

    pub fn on_ready(&mut self, pid: u16) -> Result<Vec<ProcessStateChange>, String> {
        trace!("{}: update pid {:3} by ready", LOG_TAG, pid);
        if !self.entries.contains_key(&pid) {
            return Err(format!("process {:3} does not exist", pid));
        }
        // set target to ready.
        // if target is in skip, send Skip to retry.
        let mut changes: Vec<ProcessStateChange> = Vec::new();
        if let Some(entry) = self.entries.get_mut(&pid) {
            let mut change = ProcessStateChange::new(entry);
            match entry.state {
                ProcessState::Idle => {
                    entry.set_state(ProcessState::Ready);
                    change.after = ProcessState::Ready;
                    changes.push(change);
                }
                ProcessState::Skip => {
                    entry.set_state(ProcessState::Ready);
                    change.after = ProcessState::Ready;
                    changes.push(change);
                }
                ProcessState::SkipPrev => {
                    entry.set_state(ProcessState::Skip);
                    change.after = ProcessState::Skip;
                    changes.push(change);
                }
                _ => {
                    warn!(
                        "{}: ignore ready for process {:3} in {:?}",
                        LOG_TAG, entry.pid, entry.state
                    );
                }
            };
        }
        //
        if changes.len() > 0 {
            Ok(changes)
        } else {
            return Err(format!("process {:3} cannot be ready", pid));
        }
    }

    pub fn on_done(&mut self, pid: u16) -> Result<Vec<ProcessStateChange>, String> {
        trace!("{}: update pid {:3} by done", LOG_TAG, pid);
        if !self.entries.contains_key(&pid) {
            return Err(format!("process {:3} does not exist", pid));
        }
        // set target to done.
        // if target is in overrun, set skipped flag for stop starting afters.
        let mut changes: Vec<ProcessStateChange> = Vec::new();
        let mut skipped = false;
        if let Some(entry) = self.entries.get_mut(&pid) {
            let mut change = ProcessStateChange::new(entry);
            match entry.state {
                ProcessState::Running => {
                    entry.set_state(ProcessState::Idle);
                    change.after = ProcessState::Idle;
                    changes.push(change);
                }
                ProcessState::Overrun => {
                    entry.set_state(ProcessState::SkipPrev);
                    change.after = ProcessState::SkipPrev;
                    changes.push(change);
                    skipped = true;
                }
                _ => {
                    warn!(
                        "{}: ignore done for process {:3} in {:?}",
                        LOG_TAG, entry.pid, entry.state
                    );
                }
            };
        }
        // start afters.
        if changes.len() > 0 && !skipped {
            if let Some(afters) = self.graph_forward.get(&pid).cloned() {
                // update depends first.
                trace!("{}: update after processes for pid {:3}", LOG_TAG, pid);
                for pid_after in &afters {
                    if let Some(entry) = self.entries.get_mut(pid_after) {
                        let _ = entry.update_depend(pid);
                    }
                }
                // start.
                trace!("{}: start after processes for pid {:3}", LOG_TAG, pid);
                for pid_after in &afters {
                    if let Some(entry) = self.entries.get_mut(pid_after) {
                        if !entry.is_depends_ok() {
                            // wait for the remaining dependent processes to complete.
                        } else {
                            if !entry.is_floating {
                                trace!(
                                    "{}: dependency met, waiting for next cycle {:3}",
                                    LOG_TAG, pid_after
                                );
                            } else {
                                // dependency cleared, start.
                                trace!("{}: starting pid {:3}", LOG_TAG, *pid_after);
                                let mut change: ProcessStateChange =
                                    ProcessStateChange::new(&entry);
                                if entry.set_state(ProcessState::Running) {
                                    entry.clear_depends();
                                    change.after = ProcessState::Running;
                                    changes.push(change);
                                }
                            }
                        }
                    }
                }
            }
        }
        //
        if changes.len() > 0 {
            Ok(changes)
        } else {
            return Err(format!("process {:3} cannot be done", pid));
        }
    }

    // -----
    // private methods.

    fn create_graph(
        entries: &HashMap<u16, ProcessEntry>,
    ) -> (HashSet<u16>, HashMap<u16, HashSet<u16>>) {
        // at least one client must be provided.
        if entries.len() < 1 {
            panic!("no process provided");
        }
        // find start points.
        let mut start_points: HashSet<u16> = HashSet::new();
        for entry in entries.values().filter(|e| !e.is_floating) {
            start_points.insert(entry.pid);
        }
        // - verify that at least one start point is exist.
        if start_points.len() < 1 {
            panic!("no start-point process found");
        }
        // create forward dependency by reverse.
        let mut forward_dependencies: HashMap<u16, HashSet<u16>> = HashMap::new();
        for entry in entries.values() {
            for depend in entry.depends_on.keys() {
                // - verify that dependent process exists.
                if !entries.contains_key(depend) {
                    panic!("dependent process {} does not exist", depend);
                }
                // add forward dependency.
                forward_dependencies
                    .entry(*depend)
                    .or_insert(HashSet::new())
                    .insert(entry.pid);
            }
        }
        // ok.
        return (start_points, forward_dependencies);
    }

    fn find_forward(
        pid: u16,
        include_self: bool,
        entries: &HashMap<u16, ProcessEntry>,
        forward_dependencies: &HashMap<u16, HashSet<u16>>,
        same_cycle_only: bool,
    ) -> Vec<u16> {
        let mut forwards: HashSet<u16> = HashSet::with_capacity(entries.len());
        let mut targets: Vec<u16> = Vec::new();

        // setup initial.
        targets.push(pid);
        if include_self {
            forwards.insert(pid);
        }

        // find forwards.
        while let Some(target) = targets.pop() {
            if let Some(target_forwards) = forward_dependencies.get(&target) {
                for &forward in target_forwards {
                    if same_cycle_only && !entries.get(&forward).unwrap().is_floating {
                        // stop searching when found the non-floating processes.
                        continue;
                    }
                    if !forwards.contains(&forward) {
                        forwards.insert(forward);
                        targets.push(forward);
                    }
                }
            }
        }

        return forwards.into_iter().collect();
    }

    fn find_forward_all(
        pid: u16,
        include_self: bool,
        entries: &HashMap<u16, ProcessEntry>,
        forward_dependencies: &HashMap<u16, HashSet<u16>>,
    ) -> Vec<u16> {
        return ProcessGraph::find_forward(pid, include_self, entries, forward_dependencies, false);
    }

    #[allow(dead_code)]
    fn find_forward_same_cycle(
        pid: u16,
        include_self: bool,
        entries: &HashMap<u16, ProcessEntry>,
        forward_dependencies: &HashMap<u16, HashSet<u16>>,
    ) -> Vec<u16> {
        return ProcessGraph::find_forward(pid, include_self, entries, forward_dependencies, true);
    }
}
