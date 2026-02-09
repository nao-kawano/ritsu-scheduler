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
        assert_eq!(changes.get(0).unwrap().pid, 0);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Ready);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Running);
    }

    // send event.
    let result = sched.on_start(1);
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 1);
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
        assert_eq!(changes.get(0).unwrap().pid, 20);
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
        assert_eq!(changes.get(0).unwrap().pid, 0);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Idle);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Ready);
    }

    // send event. maybe retransmission.
    let result = sched.on_ready(1);
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 1);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Ready);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Ready);
    }

    // send event.
    let result = sched.on_ready(10);
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 10);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Idle);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Ready);
    }

    // send event.
    let result = sched.on_ready(20);
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 20);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Idle);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Ready);
    }

    // send event. maybe retransmission.
    let result = sched.on_ready(30);
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 30);
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
        assert_eq!(changes.get(0).unwrap().pid, 0);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Running);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Idle);
        assert_eq!(changes.get(1).unwrap().pid, 10);
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
        assert_eq!(changes.get(0).unwrap().pid, 0);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Idle);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Idle);
    }

    // send event.
    let result = sched.on_done(1);
    // check result.
    assert!(result.is_ok());
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 2);
        assert_eq!(changes.get(0).unwrap().pid, 1);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Running);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Idle);
        assert_eq!(changes.get(1).unwrap().pid, 11);
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
    let changes = sched.on_start(20).unwrap();
    assert_eq!(changes.len(), 0);

    // == cycle:2
    // 0 marked as overrun. All dependent processes that ware Ready will become skip.
    let changes = sched.on_start(0).unwrap();
    assert_eq!(changes.len(), 5);
    assert_eq!(sched.entries.get(&0).unwrap().state, ProcessState::Overrun);
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Ready); // not affected
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Skip); // Skip
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Skip); // Skip
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Skip); // Skip
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Skip); // Skip

    let changes = sched.on_start(1).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(sched.entries.get(&0).unwrap().state, ProcessState::Overrun);
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Skip); // Skip
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Skip);

    let _ = sched.on_ready(1).unwrap();
    let _ = sched.on_ready(10).unwrap();
    let _ = sched.on_ready(11).unwrap();
    let _ = sched.on_ready(20).unwrap();
    let _ = sched.on_ready(30).unwrap();
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
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Ready); // keep Ready
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Ready); // keep Ready
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

    // == cycle:2
    let changes = sched.on_start(0).unwrap();
    assert_eq!(changes.len(), 5);
    assert_eq!(sched.entries.get(&0).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Ready); // not affected
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Late); // mark as Late
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Late); // mark as Late
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Overrun); // mark as Overrun
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Skip);

    let changes = sched.on_start(1).unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(sched.entries.get(&0).unwrap().state, ProcessState::Skip);
    assert_eq!(sched.entries.get(&1).unwrap().state, ProcessState::Skip); // Skip
    assert_eq!(sched.entries.get(&10).unwrap().state, ProcessState::Late);
    assert_eq!(sched.entries.get(&11).unwrap().state, ProcessState::Late);
    assert_eq!(sched.entries.get(&20).unwrap().state, ProcessState::Overrun);
    assert_eq!(sched.entries.get(&30).unwrap().state, ProcessState::Skip);
}
