use crate::entry::{ProcessEntry, ProcessState};
use crate::scheduler::Scheduler;
use std::collections::HashMap;

fn create_entries() -> HashMap<u16, ProcessEntry> {
    let mut entries: HashMap<u16, ProcessEntry> = HashMap::new();
    entries.insert(0, ProcessEntry::new(0, &vec![], false));
    entries.insert(1, ProcessEntry::new(1, &vec![], false));
    entries.insert(10, ProcessEntry::new(10, &vec![0], true));
    entries.insert(11, ProcessEntry::new(11, &vec![0, 1], true));
    entries.insert(20, ProcessEntry::new(20, &vec![10, 11], false));
    entries.insert(30, ProcessEntry::new(30, &vec![20], true));
    entries
}

#[test]
fn test_new() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let sched = Scheduler::new(create_entries());

    // setup condition.
    // do nothing.

    // check result.
    assert_eq!(sched.entries.len(), 6);
    assert_eq!(sched.graph_start.len(), 3);
    assert_eq!(sched.graph_start.contains(&0), true);
    assert_eq!(sched.graph_start.contains(&1), true);
    assert_eq!(sched.graph_start.contains(&20), true);
    assert_eq!(sched.graph_forward.len(), 5);
    assert_eq!(sched.graph_forward.get(&0).unwrap().contains(&10), true);
    assert_eq!(sched.graph_forward.get(&0).unwrap().contains(&11), true);
    assert_eq!(sched.graph_forward.get(&1).unwrap().contains(&11), true);
    assert_eq!(sched.graph_forward.get(&10).unwrap().contains(&20), true);
    assert_eq!(sched.graph_forward.get(&11).unwrap().contains(&20), true);
    assert_eq!(sched.graph_forward.get(&20).unwrap().contains(&30), true);
    assert_eq!(sched.graph_forward_all.len(), 6);
    assert_eq!(sched.graph_forward_same_cycle.len(), 6);
}

#[test]
fn test_reset_state() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut sched = Scheduler::new(create_entries());

    // setup condition.
    for (_, entry) in sched.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }

    // send event.
    sched.reset_state();

    // check result.
    assert_eq!(sched.entries.get(&0).unwrap().state, ProcessState::Idle);
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Idle);
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Idle);
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Idle);
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Idle);
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Idle);
}

#[test]
fn test_on_start_normal() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut sched = Scheduler::new(create_entries());

    // setup condition.
    for (_, entry) in sched.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }

    // send event.
    let result = sched.on_start(0);
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().cid, 0);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Ready);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Running);
    }

    // send event.
    let result = sched.on_start(1);
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().cid, 1);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Ready);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Running);
    }

    // send event. floating process is ignored.
    let result = sched.on_start(10);
    // check result.
    assert!(result.is_err());

    // send event. invalid process is ignored.
    let result = sched.on_start(100);
    // check result.
    assert!(result.is_err());

    // --- setup for process 20 start ---
    let _ = sched.on_done(0); // 0 finishes, updates 10's and 11's dependency on 0 -> start 10.
    let _ = sched.on_done(1); // 1 finishes, updates 11's dependency on 1 -> start 11.
    // Now call on_done for 10 and 11 to satisfy 20's dependencies.
    let _ = sched.on_done(10); // 10 finishes, updates 20's dependency on 10.
    let _ = sched.on_done(11); // 11 finishes, updates 20's dependency on 11.

    let result = sched.on_start(20);
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().cid, 20);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Ready);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Running);
    }
}

#[test]
fn test_on_ready_normal() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut sched = Scheduler::new(create_entries());

    // setup condition.
    for (_, entry) in sched.entries.iter_mut() {
        entry.state = ProcessState::Idle;
    }
    sched.entries.get_mut(&1).unwrap().state = ProcessState::Ready;
    sched.entries.get_mut(&30).unwrap().state = ProcessState::Running;

    // send event.
    let result = sched.on_ready(0);
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().cid, 0);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Idle);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Ready);
    }

    // send event. maybe retransmission.
    let result = sched.on_ready(1);
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().cid, 1);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Ready);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Ready);
    }

    // send event.
    let result = sched.on_ready(10);
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().cid, 10);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Idle);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Ready);
    }

    // send event.
    let result = sched.on_ready(20);
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().cid, 20);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Idle);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Ready);
    }

    // send event. maybe retransmission.
    let result = sched.on_ready(30);
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().cid, 30);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Running);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Running);
    }

    // send event.
    let result = sched.on_ready(100);
    // check result.
    assert!(result.is_err());
}

#[test]
fn test_on_done_normal() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut sched = Scheduler::new(create_entries());

    // setup condition.
    for (_, entry) in sched.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }
    sched.entries.get_mut(&0).unwrap().state = ProcessState::Running;
    sched.entries.get_mut(&1).unwrap().state = ProcessState::Running;

    // send event.
    let result = sched.on_done(0);
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 2);
        assert_eq!(changes.get(0).unwrap().cid, 0);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Running);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Idle);
        assert_eq!(changes.get(1).unwrap().cid, 10);
        assert_eq!(changes.get(1).unwrap().before, ProcessState::Ready);
        assert_eq!(changes.get(1).unwrap().after, ProcessState::Running);
    }

    // send event.
    let result = sched.on_done(0); // retransmission.
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        // for response OK to retransmission.
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().cid, 0);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Idle);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Idle);
    }

    // send event.
    let result = sched.on_done(1);
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 2);
        assert_eq!(changes.get(0).unwrap().cid, 1);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Running);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Idle);
        assert_eq!(changes.get(1).unwrap().cid, 11);
        assert_eq!(changes.get(1).unwrap().before, ProcessState::Ready);
        assert_eq!(changes.get(1).unwrap().after, ProcessState::Running);
    }

    // send event.
    let result = sched.on_done(100);
    // check result.
    assert!(result.is_err());
}

#[test]
fn test_find_forward_all() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let sched = Scheduler::new(create_entries());
    let es = &sched.entries;
    let fd = &sched.graph_forward;

    // setup condition.
    // do nothing.

    // check result.
    let forwards = Scheduler::find_forward_all(0, true, es, fd);
    assert_eq!(forwards.len(), 5);
    let forwards = Scheduler::find_forward_all(1, true, es, fd);
    assert_eq!(forwards.len(), 4);
    let forwards = Scheduler::find_forward_all(10, true, es, fd);
    assert_eq!(forwards.len(), 3);
    let forwards = Scheduler::find_forward_all(11, true, es, fd);
    assert_eq!(forwards.len(), 3);
    let forwards = Scheduler::find_forward_all(20, true, es, fd);
    assert_eq!(forwards.len(), 2);
    let forwards = Scheduler::find_forward_all(30, true, es, fd);
    assert_eq!(forwards.len(), 1);
    // check result.
    let forwards = Scheduler::find_forward_all(0, false, es, fd);
    assert_eq!(forwards.len(), 4);
    let forwards = Scheduler::find_forward_all(1, false, es, fd);
    assert_eq!(forwards.len(), 3);
    let forwards = Scheduler::find_forward_all(10, false, es, fd);
    assert_eq!(forwards.len(), 2);
    let forwards = Scheduler::find_forward_all(11, false, es, fd);
    assert_eq!(forwards.len(), 2);
    let forwards = Scheduler::find_forward_all(20, false, es, fd);
    assert_eq!(forwards.len(), 1);
    let forwards = Scheduler::find_forward_all(30, false, es, fd);
    assert_eq!(forwards.len(), 0);
}

#[test]
fn test_find_forward_same_cycle() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let sched = Scheduler::new(create_entries());
    let es = &sched.entries;
    let fd = &sched.graph_forward;

    // setup condition.
    // do nothing.

    // check result.
    let forwards = Scheduler::find_forward_same_cycle(0, true, es, fd);
    assert_eq!(forwards.len(), 3);
    let forwards = Scheduler::find_forward_same_cycle(1, true, es, fd);
    assert_eq!(forwards.len(), 2);
    let forwards = Scheduler::find_forward_same_cycle(10, true, es, fd);
    assert_eq!(forwards.len(), 1);
    let forwards = Scheduler::find_forward_same_cycle(11, true, es, fd);
    assert_eq!(forwards.len(), 1);
    let forwards = Scheduler::find_forward_same_cycle(20, true, es, fd);
    assert_eq!(forwards.len(), 2);
    let forwards = Scheduler::find_forward_same_cycle(30, true, es, fd);
    assert_eq!(forwards.len(), 1);
    // check result.
    let forwards = Scheduler::find_forward_same_cycle(0, false, es, fd);
    assert_eq!(forwards.len(), 2);
    let forwards = Scheduler::find_forward_same_cycle(1, false, es, fd);
    assert_eq!(forwards.len(), 1);
    let forwards = Scheduler::find_forward_same_cycle(10, false, es, fd);
    assert_eq!(forwards.len(), 0);
    let forwards = Scheduler::find_forward_same_cycle(11, false, es, fd);
    assert_eq!(forwards.len(), 0);
    let forwards = Scheduler::find_forward_same_cycle(20, false, es, fd);
    assert_eq!(forwards.len(), 1);
    let forwards = Scheduler::find_forward_same_cycle(30, false, es, fd);
    assert_eq!(forwards.len(), 0);
}

// -----------------------------------------------------------------------------

#[test]
fn test_scenario_overrun_and_late() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut sched = Scheduler::new(create_entries());

    // scenario:
    // - process 0 is overrun.

    // setup condition.
    for (_, entry) in sched.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }

    // == cycle:0
    let changes = sched.on_start(0).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(sched.entries.get(&0).unwrap().state, ProcessState::Running);

    // NOTE: Processes sharing the same activation cycle are handled consecutively, ensuring that no interleaved events occur.
    let changes = sched.on_start(1).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Running);
    let changes = sched.on_done(1).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Idle);
    let changes = sched.on_ready(1).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Ready);

    // == cycle:1
    // 20 and 30 autonomously skip due to unmet dependencies (10 and 11 are not started).
    let changes = sched.on_start(20).unwrap();
    assert_eq!(changes.len(), 2); // 20 (Skip), 30 (Skip)
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Skip);
    let _ = sched.on_ready(20).unwrap();
    let _ = sched.on_ready(30).unwrap();

    // == cycle:2
    // 0 marked as overrun. Dependents in the same cycle (10, 11) will become skip.
    let changes = sched.on_start(0).unwrap();
    assert_eq!(changes.len(), 3); // 0 (Overrun), 10 (Skip), 11 (Skip)
    assert_eq!(sched.entries.get(&0).unwrap().state, ProcessState::Overrun);
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Ready); // Not affected yet
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Ready); // Not affected yet

    let changes = sched.on_start(1).unwrap();
    assert_eq!(changes.len(), 1); // 1 (Skip)
    assert_eq!(sched.entries.get(&0).unwrap().state, ProcessState::Overrun);
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Ready);

    let _ = sched.on_ready(1).unwrap();
    let _ = sched.on_ready(10).unwrap();
    let _ = sched.on_ready(11).unwrap();
    assert_eq!(sched.entries.get(&0).unwrap().state, ProcessState::Overrun);
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Ready);

    // 0 is done
    let changes = sched.on_done(0).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(sched.entries.get(&0).unwrap().state, ProcessState::Late);
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Ready);

    // 0 sends ready
    let changes = sched.on_ready(0).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(sched.entries.get(&0).unwrap().state, ProcessState::Idle);

    // 0 sends ready again
    let changes = sched.on_ready(0).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(sched.entries.get(&0).unwrap().state, ProcessState::Ready);

    // == cycle:3
    // Call on_start(20) at its execution window.
    // 20 and 30 autonomously skip due to unmet dependencies (10 and 11 are skipped, not done).
    let changes = sched.on_start(20).unwrap();
    assert_eq!(changes.len(), 2); // 20 (Skip), 30 (Skip)
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Skip);
}

#[test]
fn test_scenario_idle_and_late() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut sched = Scheduler::new(create_entries());

    // scenario:
    // - process 0, 1 are completed
    // - process 10, 11 are done but not ready

    // setup condition.
    for (_, entry) in sched.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }

    // == cycle:0
    let _ = sched.on_start(0).unwrap();
    let _ = sched.on_start(1).unwrap();
    let _ = sched.on_done(0).unwrap();
    let _ = sched.on_done(1).unwrap();
    let _ = sched.on_ready(0).unwrap();
    let _ = sched.on_ready(1).unwrap();

    let _ = sched.on_done(10).unwrap();
    let _ = sched.on_done(11).unwrap();
    assert_eq!(sched.entries.get(&0).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Idle);
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Idle);
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Ready);

    // == cycle:1
    let _ = sched.on_start(20).unwrap();
    assert_eq!(sched.entries.get(&0).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Idle);
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Idle);
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Running);
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Ready);
    let _ = sched.on_done(20).unwrap();
    let _ = sched.on_ready(20).unwrap();
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Running);
    let _ = sched.on_done(30).unwrap();
    let _ = sched.on_ready(30).unwrap();

    // == cycle:2
    let changes = sched.on_start(0).unwrap();
    assert_eq!(changes.len(), 3); // 0 (Skip), 10 (Late), 11 (Late)
    assert_eq!(sched.entries.get(&0).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Late);
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Late);
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Ready); // Not affected yet
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Ready); // Not affected yet
    let _ = sched.on_ready(0).unwrap();

    let changes = sched.on_start(1).unwrap();
    assert_eq!(changes.len(), 1); // 1 (Skip)
    assert_eq!(sched.entries.get(&0).unwrap().state, ProcessState::Ready); // recovered
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Late);
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Late);
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Ready);
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Ready);
    let _ = sched.on_ready(1).unwrap();

    // Recover Late processes (Late -> Idle -> Ready)
    // For 10
    let changes = sched.on_ready(10).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Idle);
    let changes = sched.on_ready(10).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Ready);
    // For 11
    let changes = sched.on_ready(11).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Idle);
    let changes = sched.on_ready(11).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Ready);

    // == cycle:3
    // Call on_start(20) at its execution window.
    // 20 and 30 autonomously skip due to unmet dependencies (10 and 11 were Late).
    let changes = sched.on_start(20).unwrap();
    assert_eq!(changes.len(), 2); // 20 (Skip), 30 (Skip)
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Skip);
    let _ = sched.on_ready(20).unwrap();
    let _ = sched.on_ready(30).unwrap();
}

#[test]
fn test_dependency_past_ghost_clear() {
    let _ = env_logger::builder().is_test(true).try_init();
    let mut sched = Scheduler::new(create_entries());

    // Setup all processes to Ready
    for (_, entry) in sched.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }

    // == cycle:0
    let _ = sched.on_start(0).unwrap();
    let _ = sched.on_start(1).unwrap();

    // 0 completes, starting 10. 11 waits because 1 is not done.
    let _ = sched.on_done(0).unwrap();
    let _ = sched.on_ready(0).unwrap();
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Running);
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Ready);

    // 10 completes.
    let _ = sched.on_done(10).unwrap();
    let _ = sched.on_ready(10).unwrap();
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Ready);

    // 1 remains Running (delayed).

    // == cycle:1
    // 1 is still Running.
    // 20 should autonomously skip due to unmet dependencies (11 is not done).
    let changes = sched.on_start(20).unwrap();
    assert_eq!(changes.len(), 2); // 20 (Skip), 30 (Skip)
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Skip);
    let _ = sched.on_ready(20).unwrap();
    let _ = sched.on_ready(30).unwrap();

    // Later in cycle:1, 1 finally finishes.
    // This starts 11.
    let _ = sched.on_done(1).unwrap();
    let _ = sched.on_ready(1).unwrap();
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Running);

    // 11 finishes, setting dependency flag on 20.
    // 20's dependency on 11 is now satisfied (which is a "ghost" from cycle 1 delay).
    let _ = sched.on_done(11).unwrap();
    let _ = sched.on_ready(11).unwrap();

    // == cycle:2
    // Start of cycle 2 (offset=0's turn).
    // This must trigger the ghost clearing logic for descendants of 0 (which includes 20).
    let _ = sched.on_start(0).unwrap();
    let _ = sched.on_start(1).unwrap();

    // Check that 20's dependency flag from 11 has been reset to false,
    // and unmet_dependencies is back to 2.
    let entry_20 = sched.entries.get(&20).unwrap();
    let dep_11 = entry_20
        .dependency_statuses
        .iter()
        .find(|x| x.0 == 11)
        .unwrap();
    assert_eq!(dep_11.1, false); // Cleared
    assert_eq!(entry_20.unmet_dependencies, 2);

    // Both 0 and 1 complete immediately.
    // 10 starts. 11 starts because both 0 and 1 are complete.
    let _ = sched.on_done(0).unwrap();
    let _ = sched.on_ready(0).unwrap();
    let _ = sched.on_done(1).unwrap();
    let _ = sched.on_ready(1).unwrap();
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Running);
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Running);

    // 10 completes, but 11 is delayed again.
    let _ = sched.on_done(10).unwrap();
    let _ = sched.on_ready(10).unwrap();

    // == cycle:3
    // 20's execution window (offset=1's turn).
    // Since 11 is still Running, 20's dependency on 11 is unmet.
    // If the ghost flag was correctly cleared, 20 must skip.
    let changes = sched.on_start(20).unwrap();
    assert_eq!(changes.len(), 2); // 20 (Skip), 30 (Skip)
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Skip);
    let _ = sched.on_ready(20).unwrap();
    let _ = sched.on_ready(30).unwrap();
}
