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
        // start target.
        // if target is still running, mark as overrun and propagate skip to after processes.
        let changes = self.try_start(pid);
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
        // if target is in skip, set to idle to re-send ready.
        let mut changes: Vec<ProcessStateChange> = Vec::new();
        if let Some(entry) = self.entries.get_mut(&pid) {
            let change = ProcessGraph::apply_ready(entry);
            if let Some(c) = change {
                changes.push(c);
            }
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
            let change = ProcessGraph::apply_done(entry);
            if let Some(c) = change {
                if c.before == ProcessState::Overrun {
                    skipped = true;
                    info!("{}: {:3} done in overrun", LOG_TAG, pid);
                }
                changes.push(c);
            }
        }
        // start afters.
        if changes.len() > 0 && !skipped {
            if let Some(afters) = self.graph_forward.get(&pid).cloned() {
                // update depends first for guard propagate.
                trace!("{}: update after processes for pid {:3}", LOG_TAG, pid);
                for pid_after in &afters {
                    if let Some(entry) = self.entries.get_mut(pid_after) {
                        let updated = entry.update_depend(pid);
                        if !updated {
                            changes.extend(self.skip_all_depends(*pid_after));
                        }
                    }
                }
                // start or skip.
                trace!("{}: start after processes for pid {:3}", LOG_TAG, pid);
                for pid_after in &afters {
                    if let Some(entry) = self.entries.get_mut(pid_after) {
                        if entry.is_depends_ok() {
                            // cleared all dependency, try to start.
                            // if target is still running, mark as overrun and propagate skip to after processes.
                            trace!("{}: starting pid {:3}", LOG_TAG, *pid_after);
                            let changes2 = self.try_start(*pid_after);
                            if changes2.len() > 0 {
                                changes.extend(changes2);
                            } else {
                                return Err(format!(
                                    "process {:3} likely the second skip has occurred.",
                                    *pid_after
                                ));
                            }
                        } else {
                            // wait for the remaining dependent processes to complete.
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

    pub fn try_start(&mut self, pid_target: u16) -> Vec<ProcessStateChange> {
        let mut changes: Vec<ProcessStateChange> = Vec::new();
        // update target.
        if let Some(entry) = self.entries.get_mut(&pid_target) {
            let mut change: ProcessStateChange = ProcessStateChange::new(entry);
            match entry.state {
                ProcessState::Ready => {
                    entry.clear_depends();
                    entry.set_state(ProcessState::Running);
                    change.after = ProcessState::Running;
                    changes.push(change);
                }
                ProcessState::Running | ProcessState::Idle => {
                    info!("{}: detected pid {:3} skipped", LOG_TAG, pid_target);
                    // not ready or running, propagate Skip.
                    changes.extend(self.skip_all_depends(pid_target));
                }
                _ => {
                    warn!(
                        "{}: ignore start for pid {:3} in {:?}",
                        LOG_TAG, pid_target, entry.state
                    );
                }
            };
        }
        changes
    }

    fn skip_all_depends(&mut self, pid_target: u16) -> Vec<ProcessStateChange> {
        trace!("{}: skip all depends for pid {:3}", LOG_TAG, pid_target);
        let mut changes: Vec<ProcessStateChange> = Vec::new();
        let mut visited: HashSet<u16> = HashSet::new();
        // update target.
        if let Some(entry) = self.entries.get_mut(&pid_target) {
            let change = ProcessGraph::apply_skip(entry);
            if let Some(change) = change {
                changes.push(change);
                visited.insert(pid_target);
            } else {
                return changes; // second skip, empty changes.
            }
        }
        // propagate skip.
        if changes.len() > 0 {
            let mut neighbors: Vec<u16> = Vec::new();
            // push initial targets.
            // subsequent processes must be skipped due to unexecuted state.
            if let Some(entry) = self.graph_forward.get(&pid_target) {
                neighbors.extend(entry);
            }
            // add befores
            if let Some(entry) = self.entries.get(&pid_target) {
                neighbors.extend(
                    entry
                        .depends_on
                        .iter()
                        .filter(|(_, completed)| !*completed)
                        .map(|(pid, _)| *pid),
                );
            }
            trace!("{}: initial propagate {:?}", LOG_TAG, neighbors);
            // propagate.
            while let Some(neighbor) = neighbors.pop() {
                // pre-check.
                if visited.contains(&neighbor) {
                    trace!("{}: already propagated {:?}", LOG_TAG, neighbor);
                    continue;
                } else {
                    trace!("{}: propagating {:?}", LOG_TAG, neighbor);
                    visited.insert(neighbor);
                }
                // propagate.
                // set target to skip and obtains incompleted befores.
                if let Some(entry) = self.entries.get_mut(&neighbor) {
                    let change = ProcessGraph::apply_skip(entry);
                    if let Some(change) = change {
                        changes.push(change);
                    }
                    // add new neighbors.
                    neighbors.extend(
                        entry
                            .depends_on
                            .iter()
                            .filter(|(_, completed)| !*completed)
                            .map(|(pid, _)| *pid),
                    );
                    if let Some(entry) = self.graph_forward.get(&neighbor) {
                        neighbors.extend(entry);
                    }
                }
                trace!("{}: > new propagate {:?}", LOG_TAG, neighbors);
                trace!("{}: > new skipped   {:?}", LOG_TAG, visited);
            }
        }
        changes
    }

    fn apply_ready(entry: &mut ProcessEntry) -> Option<ProcessStateChange> {
        let mut change: Option<ProcessStateChange> = None;
        // update state.
        {
            let mut c: ProcessStateChange = ProcessStateChange::new(entry);
            match entry.state {
                ProcessState::Idle => {
                    entry.set_state(ProcessState::Ready);
                    c.after = ProcessState::Ready;
                    change = Some(c);
                }
                ProcessState::Skip => {
                    entry.set_state(ProcessState::Idle);
                    c.after = ProcessState::Idle;
                    change = Some(c);
                }
                _ => {
                    warn!(
                        "{}: ignore ready for process {:3} in {:?}",
                        LOG_TAG, entry.pid, entry.state
                    );
                }
            };
        }
        change
    }

    fn apply_done(entry: &mut ProcessEntry) -> Option<ProcessStateChange> {
        let mut change: Option<ProcessStateChange> = None;
        // update state.
        {
            let mut c: ProcessStateChange = ProcessStateChange::new(entry);
            match entry.state {
                ProcessState::Running => {
                    entry.set_state(ProcessState::Idle);
                    c.after = ProcessState::Idle;
                    change = Some(c);
                }
                ProcessState::Overrun => {
                    entry.set_state(ProcessState::Idle);
                    c.after = ProcessState::Idle;
                    change = Some(c);
                }
                _ => {
                    warn!(
                        "{}: ignore done for process {:3} in {:?}",
                        LOG_TAG, entry.pid, entry.state
                    );
                }
            };
        }
        change
    }

    fn apply_skip(entry: &mut ProcessEntry) -> Option<ProcessStateChange> {
        let mut change: Option<ProcessStateChange> = None;
        // update state.
        {
            let mut c: ProcessStateChange = ProcessStateChange::new(entry);
            match entry.state {
                ProcessState::Ready => {
                    entry.set_state(ProcessState::Idle);
                    c.after = ProcessState::Idle;
                    change = Some(c);
                }
                ProcessState::Running => {
                    entry.set_state(ProcessState::Overrun);
                    c.after = ProcessState::Overrun;
                    change = Some(c);
                }
                ProcessState::Idle => {
                    entry.set_state(ProcessState::Skip);
                    c.after = ProcessState::Skip;
                    change = Some(c);
                }
                _ => {
                    warn!(
                        "{}: ignore skip for process {:3} in {:?}",
                        LOG_TAG, entry.pid, entry.state
                    );
                }
            }
        }
        change
    }

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
}
