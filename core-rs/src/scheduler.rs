//!
//! Process scheduler.
//!

extern crate log;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
const LOG_TAG: &str = "Scheduler";

use std::collections::{HashMap, HashSet};

use crate::entry::ProcessEntry;
use crate::entry::ProcessState;

#[cfg(test)]
#[path = "scheduler_test.rs"]
mod scheduler_test;

/* -------------------------------------------------------------------------- */

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProcessStateChange {
    pub cid: u16,
    pub before: ProcessState,
    pub after: ProcessState,
}

impl ProcessStateChange {
    pub(crate) fn new(entry: &ProcessEntry) -> Self {
        ProcessStateChange {
            cid: entry.cid,
            before: entry.state,
            after: entry.state,
        }
    }
}

/* -------------------------------------------------------------------------- */

pub struct Scheduler {
    entries: HashMap<u16, ProcessEntry>,
    graph_start: HashSet<u16>,
    graph_forward: HashMap<u16, HashSet<u16>>,
}

impl Scheduler {
    pub fn new(entries: HashMap<u16, ProcessEntry>) -> Self {
        let (graph_start, graph_forward) = Scheduler::create_graph(&entries);
        Scheduler {
            entries,
            graph_start,
            graph_forward,
        }
    }

    pub fn get_ready_processes(&self) -> Vec<u16> {
        let mut ready_processes: Vec<u16> = Vec::new();
        for entry in self.entries.values() {
            if entry.state == ProcessState::Ready {
                ready_processes.push(entry.cid);
            }
        }
        return ready_processes;
    }

    pub fn reset_state(&mut self) {
        trace!("{}: reset process state", LOG_TAG);
        for entry in self.entries.values_mut() {
            entry.reset(); // all process set to Idle.
        }
    }

    pub fn on_start(&mut self, cid: u16) -> Result<Vec<ProcessStateChange>, String> {
        trace!("{}: update CID:{:03} by start", LOG_TAG, cid);
        if !self.graph_start.contains(&cid) {
            return Err(format!("process CID:{:03} does not exist", cid));
        }

        // check if dependency is met.
        // if target cid has dependency, check at next root cycle.
        if let Some(entry) = self.entries.get(&cid) {
            if !entry.is_depends_ok() {
                debug!(
                    "{}: CID:{:03} has dependency unmet, skip start",
                    LOG_TAG, cid
                );
                return Ok(vec![]);
            }
        }

        // check for not-ready processes.
        let forwards = Scheduler::find_forward_all(cid, true, &self.entries, &self.graph_forward);
        let mut overrun_cids: Vec<u16> = Vec::new();
        let mut late_cids: Vec<u16> = Vec::new();
        let mut is_not_ready = false;
        for forward_cid in &forwards {
            if let Some(entry) = self.entries.get(forward_cid) {
                match entry.state {
                    ProcessState::Ready => {
                        // OK.
                    }
                    ProcessState::Running => {
                        overrun_cids.push(*forward_cid); // holds for state change.
                        is_not_ready = true;
                    }
                    ProcessState::Overrun => {
                        is_not_ready = true;
                    }
                    ProcessState::Idle => {
                        late_cids.push(*forward_cid); // holds for state change.
                        is_not_ready = true;
                    }
                    ProcessState::Skip => {
                        is_not_ready = true;
                    }
                    ProcessState::Late => {
                        is_not_ready = true;
                    }
                }
            }
        }

        let mut changes = Vec::new();
        if is_not_ready {
            let mut forwards_set: HashSet<u16> = forwards.iter().cloned().collect();
            // Mark Running processes as Overrun and their dependents as Skip
            for running_cid in overrun_cids {
                // Mark as Overrun
                if let Some(entry) = self.entries.get_mut(&running_cid) {
                    let mut change = ProcessStateChange::new(&entry);
                    if entry.set_state(ProcessState::Overrun) {
                        // Process itself becomes Overrun
                        change.after = ProcessState::Overrun;
                        changes.push(change);
                    }
                    forwards_set.remove(&running_cid);
                }
                // Dependents of overrun processes become Skip
                let skip_forwards = Scheduler::find_forward_all(
                    running_cid,
                    false, // Do not include itself, as it's already Overrun
                    &self.entries,
                    &self.graph_forward,
                );
                for skip_cid in skip_forwards {
                    if forwards_set.contains(&skip_cid) {
                        // Only change if not already handled
                        if let Some(entry) = self.entries.get_mut(&skip_cid) {
                            let mut change = ProcessStateChange::new(&entry);
                            if entry.set_state(ProcessState::Skip) {
                                entry.clear_depends(); // Clear dependencies as it's skipped
                                change.after = ProcessState::Skip;
                                changes.push(change);
                            }
                            forwards_set.remove(&skip_cid);
                        }
                    }
                }
            }
            // Mark Idle processes as Late
            for late_cid in late_cids {
                if forwards_set.contains(&late_cid) {
                    // Only change if not already handled
                    if let Some(entry) = self.entries.get_mut(&late_cid) {
                        let mut change = ProcessStateChange::new(&entry);
                        if entry.set_state(ProcessState::Late) {
                            change.after = ProcessState::Late;
                            changes.push(change);
                        }
                        forwards_set.remove(&late_cid);
                    }
                }
            }
            // Mark remaining Ready processes as Skip
            for skip_cid in forwards_set {
                if let Some(entry) = self.entries.get_mut(&skip_cid) {
                    if entry.state == ProcessState::Ready {
                        let mut change = ProcessStateChange::new(&entry);
                        if entry.set_state(ProcessState::Skip) {
                            change.after = ProcessState::Skip;
                            changes.push(change);
                        }
                    }
                }
            }
        } else {
            // All dependent process is in ready, normal start
            if let Some(entry) = self.entries.get_mut(&cid) {
                let mut change = ProcessStateChange::new(&entry);
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
            return Err(format!("process CID:{:03} start caused no changes", cid));
        }
    }

    pub fn on_ready(&mut self, cid: u16) -> Result<Vec<ProcessStateChange>, String> {
        trace!("{}: update CID:{:03} by ready", LOG_TAG, cid);
        if !self.entries.contains_key(&cid) {
            return Err(format!("process CID:{:03} does not exist", cid));
        }
        // set target to ready.
        let mut changes: Vec<ProcessStateChange> = Vec::new();
        if let Some(entry) = self.entries.get_mut(&cid) {
            let mut change = ProcessStateChange::new(entry);
            match entry.state {
                ProcessState::Idle | ProcessState::Skip => {
                    entry.set_state(ProcessState::Ready);
                    change.after = ProcessState::Ready;
                    changes.push(change);
                }
                ProcessState::Ready => {
                    // maybe retransmission, keep Ready.
                    entry.set_state(ProcessState::Ready);
                    change.after = ProcessState::Ready;
                    changes.push(change);
                }
                ProcessState::Running => {
                    // maybe retransmission, keep Running and send OK to start immediately.
                    change.after = ProcessState::Running;
                    changes.push(change);
                }
                ProcessState::Overrun => {
                    // cannot transition to Ready directly.
                    warn!(
                        "{}: ignore ready for process CID:{:03} in {:?}",
                        LOG_TAG, entry.cid, entry.state
                    );
                }
                ProcessState::Late => {
                    entry.set_state(ProcessState::Idle);
                    change.after = ProcessState::Idle;
                    changes.push(change);
                }
            };
        }
        //
        if changes.len() > 0 {
            Ok(changes)
        } else {
            return Err(format!("process CID:{:03} cannot be ready", cid));
        }
    }

    pub fn on_done(&mut self, cid: u16) -> Result<Vec<ProcessStateChange>, String> {
        trace!("{}: update CID:{:03} by done", LOG_TAG, cid);
        if !self.entries.contains_key(&cid) {
            return Err(format!("process CID:{:03} does not exist", cid));
        }
        // set target to done.
        // if target is in overrun, set skipped flag to stop starting afters.
        let mut changes: Vec<ProcessStateChange> = Vec::new();
        let mut skipped = false;
        if let Some(entry) = self.entries.get_mut(&cid) {
            let mut change = ProcessStateChange::new(entry);
            match entry.state {
                ProcessState::Idle => {
                    // maybe retransmission, keep Idle and send OK.
                    change.after = ProcessState::Idle;
                    changes.push(change);
                    skipped = true;
                }
                ProcessState::Running => {
                    entry.set_state(ProcessState::Idle);
                    change.after = ProcessState::Idle;
                    changes.push(change);
                }
                ProcessState::Overrun => {
                    entry.set_state(ProcessState::Late);
                    change.after = ProcessState::Late;
                    changes.push(change);
                    skipped = true;
                }
                ProcessState::Late => {
                    // maybe retransmission, keep Late and send OK.
                    change.after = ProcessState::Late;
                    changes.push(change);
                    skipped = true;
                }
                ProcessState::Ready | ProcessState::Skip => {
                    warn!(
                        "{}: ignore done for process CID:{:03} in {:?}",
                        LOG_TAG, entry.cid, entry.state
                    );
                }
            };
        }
        // start afters.
        // if not skipped, and there are changes (meaning the process state was valid for done)
        if !skipped && changes.len() > 0 {
            if let Some(afters) = self.graph_forward.get(&cid).cloned() {
                // update depends first.
                trace!("{}: update after processes for CID:{:03}", LOG_TAG, cid);
                for cid_after in &afters {
                    if let Some(entry) = self.entries.get_mut(cid_after) {
                        let _ = entry.update_depend(cid);
                    }
                }
                // start.
                trace!("{}: start after processes for CID:{:03}", LOG_TAG, cid);
                for cid_after in &afters {
                    if let Some(entry) = self.entries.get_mut(cid_after) {
                        if !entry.is_depends_ok() {
                            // wait for the remaining dependent processes to complete.
                        } else {
                            if !entry.is_floating {
                                trace!(
                                    "{}: dependency met, waiting for next cycle CID:{:03}",
                                    LOG_TAG, cid_after
                                );
                            } else {
                                // dependency met, start.
                                trace!("{}: starting CID:{:03}", LOG_TAG, *cid_after);
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
            return Err(format!("process CID:{:03} cannot be done", cid));
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
            start_points.insert(entry.cid);
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
                    .insert(entry.cid);
            }
        }
        // ok.
        return (start_points, forward_dependencies);
    }

    fn find_forward(
        cid: u16,
        include_self: bool,
        entries: &HashMap<u16, ProcessEntry>,
        forward_dependencies: &HashMap<u16, HashSet<u16>>,
        same_cycle_only: bool,
    ) -> Vec<u16> {
        let mut forwards: HashSet<u16> = HashSet::with_capacity(entries.len());
        let mut targets: Vec<u16> = Vec::new();

        // setup initial.
        targets.push(cid);
        if include_self {
            forwards.insert(cid);
        }

        // find forwards.
        while let Some(target) = targets.pop() {
            if let Some(target_forwards) = forward_dependencies.get(&target) {
                for &forward in target_forwards {
                    if same_cycle_only && !entries.get(&forward).map_or(false, |e| e.is_floating) {
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
        cid: u16,
        include_self: bool,
        entries: &HashMap<u16, ProcessEntry>,
        forward_dependencies: &HashMap<u16, HashSet<u16>>,
    ) -> Vec<u16> {
        return Scheduler::find_forward(cid, include_self, entries, forward_dependencies, false);
    }

    #[allow(dead_code)]
    fn find_forward_same_cycle(
        cid: u16,
        include_self: bool,
        entries: &HashMap<u16, ProcessEntry>,
        forward_dependencies: &HashMap<u16, HashSet<u16>>,
    ) -> Vec<u16> {
        return Scheduler::find_forward(cid, include_self, entries, forward_dependencies, true);
    }
}
