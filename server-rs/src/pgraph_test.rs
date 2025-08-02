#[cfg(test)]
use super::*;

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
    let g = ProcessGraph::new(create_entries());

    // setup condition.
    // do nothing.

    // check result.
    assert_eq!(g.entries.len(), 6);
    assert_eq!(g.graph_start.len(), 3);
    assert_eq!(g.graph_start.contains(&0), true);
    assert_eq!(g.graph_start.contains(&1), true);
    assert_eq!(g.graph_start.contains(&20), true);
    assert_eq!(g.graph_forward.len(), 5);
    assert_eq!(g.graph_forward.get(&0).unwrap().contains(&10), true);
    assert_eq!(g.graph_forward.get(&0).unwrap().contains(&11), true);
    assert_eq!(g.graph_forward.get(&1).unwrap().contains(&11), true);
    assert_eq!(g.graph_forward.get(&10).unwrap().contains(&20), true);
    assert_eq!(g.graph_forward.get(&11).unwrap().contains(&20), true);
    assert_eq!(g.graph_forward.get(&20).unwrap().contains(&30), true);
}

#[test]
fn test_reset_state() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut g = ProcessGraph::new(create_entries());

    // setup condition.
    for (_, entry) in g.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }

    // send event.
    g.reset_state();

    // check result.
    assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Idle);
    assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Idle);
    assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Idle);
    assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Idle);
    assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Idle);
    assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Idle);
}

#[test]
fn test_on_start_normal() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut g = ProcessGraph::new(create_entries());

    // setup condition.
    for (_, entry) in g.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }

    // send event.
    let result = g.on_start(0);
    // check result.
    assert_eq!(result.is_ok(), true);
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 0);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Ready);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Running);
    }

    // send event.
    let result = g.on_start(1);
    // check result.
    assert_eq!(result.is_ok(), true);
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 1);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Ready);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Running);
    }

    // send event.
    let result = g.on_start(10);
    // check result.
    assert_eq!(result.is_ok(), false);

    // send event.
    let result = g.on_start(100);
    // check result.
    assert_eq!(result.is_ok(), false);

    // send event.
    let result = g.on_done(0);
    assert_eq!(result.is_ok(), true);
    let result = g.on_done(1);
    assert_eq!(result.is_ok(), true);
    let result = g.on_done(10);
    assert_eq!(result.is_ok(), true);
    let result = g.on_done(11);
    assert_eq!(result.is_ok(), true);
    let result = g.on_start(20);
    // check result.
    assert_eq!(result.is_ok(), true);
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
    let mut g = ProcessGraph::new(create_entries());

    // setup condition.
    for (_, entry) in g.entries.iter_mut() {
        entry.state = ProcessState::Idle;
    }
    g.entries.get_mut(&1).unwrap().state = ProcessState::Ready;
    g.entries.get_mut(&30).unwrap().state = ProcessState::Running;

    // send event.
    let result = g.on_ready(0);
    // check result.
    assert_eq!(result.is_ok(), true);
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 0);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Idle);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Ready);
    }

    // send event.
    let result = g.on_ready(1);
    // check result.
    assert_eq!(result.is_err(), true);

    // send event.
    let result = g.on_ready(10);
    // check result.
    assert_eq!(result.is_ok(), true);
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 10);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Idle);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Ready);
    }

    // send event.
    let result = g.on_ready(20);
    // check result.
    assert_eq!(result.is_ok(), true);
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 20);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Idle);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Ready);
    }

    // send event.
    let result = g.on_ready(30); // retransmission.
    // check result.
    assert_eq!(result.is_ok(), true);
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 30);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Running);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Running);
    }

    // send event.
    let result = g.on_ready(100);
    // check result.
    assert_eq!(result.is_ok(), false);
}

#[test]
fn test_on_done_normal() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut g = ProcessGraph::new(create_entries());

    // setup condition.
    for (_, entry) in g.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }
    g.entries.get_mut(&0).unwrap().state = ProcessState::Running;
    g.entries.get_mut(&1).unwrap().state = ProcessState::Running;

    // send event.
    let result = g.on_done(0);
    // check result.
    assert_eq!(result.is_ok(), true);
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
    let result = g.on_done(0); // retransmission.
    // check result.
    assert_eq!(result.is_ok(), true);
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 0);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Idle);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Idle);
    }

    // send event.
    let result = g.on_done(1);
    // check result.
    assert_eq!(result.is_ok(), true);
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
    let result = g.on_done(100);
    // check result.
    assert_eq!(result.is_ok(), false);
}

#[test]
fn test_find_forward_all() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let g = ProcessGraph::new(create_entries());
    let es = &g.entries;
    let fd = &g.graph_forward;

    // setup condition.
    // do nothing.

    // check result.
    let forwards = ProcessGraph::find_forward_all(0, true, es, fd);
    assert_eq!(forwards.len(), 5);
    let forwards = ProcessGraph::find_forward_all(1, true, es, fd);
    assert_eq!(forwards.len(), 4);
    let forwards = ProcessGraph::find_forward_all(10, true, es, fd);
    assert_eq!(forwards.len(), 3);
    let forwards = ProcessGraph::find_forward_all(11, true, es, fd);
    assert_eq!(forwards.len(), 3);
    let forwards = ProcessGraph::find_forward_all(20, true, es, fd);
    assert_eq!(forwards.len(), 2);
    let forwards = ProcessGraph::find_forward_all(30, true, es, fd);
    assert_eq!(forwards.len(), 1);
    // check result.
    let forwards = ProcessGraph::find_forward_all(0, false, es, fd);
    assert_eq!(forwards.len(), 4);
    let forwards = ProcessGraph::find_forward_all(1, false, es, fd);
    assert_eq!(forwards.len(), 3);
    let forwards = ProcessGraph::find_forward_all(10, false, es, fd);
    assert_eq!(forwards.len(), 2);
    let forwards = ProcessGraph::find_forward_all(11, false, es, fd);
    assert_eq!(forwards.len(), 2);
    let forwards = ProcessGraph::find_forward_all(20, false, es, fd);
    assert_eq!(forwards.len(), 1);
    let forwards = ProcessGraph::find_forward_all(30, false, es, fd);
    assert_eq!(forwards.len(), 0);
}

#[test]
fn test_find_forward_same_cycle() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let g = ProcessGraph::new(create_entries());
    let es = &g.entries;
    let fd = &g.graph_forward;

    // setup condition.
    // do nothing.

    // check result.
    let forwards = ProcessGraph::find_forward_same_cycle(0, true, es, fd);
    assert_eq!(forwards.len(), 3);
    let forwards = ProcessGraph::find_forward_same_cycle(1, true, es, fd);
    assert_eq!(forwards.len(), 2);
    let forwards = ProcessGraph::find_forward_same_cycle(10, true, es, fd);
    assert_eq!(forwards.len(), 1);
    let forwards = ProcessGraph::find_forward_same_cycle(11, true, es, fd);
    assert_eq!(forwards.len(), 1);
    let forwards = ProcessGraph::find_forward_same_cycle(20, true, es, fd);
    assert_eq!(forwards.len(), 2);
    let forwards = ProcessGraph::find_forward_same_cycle(30, true, es, fd);
    assert_eq!(forwards.len(), 1);
    // check result.
    let forwards = ProcessGraph::find_forward_same_cycle(0, false, es, fd);
    assert_eq!(forwards.len(), 2);
    let forwards = ProcessGraph::find_forward_same_cycle(1, false, es, fd);
    assert_eq!(forwards.len(), 1);
    let forwards = ProcessGraph::find_forward_same_cycle(10, false, es, fd);
    assert_eq!(forwards.len(), 0);
    let forwards = ProcessGraph::find_forward_same_cycle(11, false, es, fd);
    assert_eq!(forwards.len(), 0);
    let forwards = ProcessGraph::find_forward_same_cycle(20, false, es, fd);
    assert_eq!(forwards.len(), 1);
    let forwards = ProcessGraph::find_forward_same_cycle(30, false, es, fd);
    assert_eq!(forwards.len(), 0);
}

// -----------------------------------------------------------------------------

#[test]
fn test_skip1() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut g = ProcessGraph::new(create_entries());

    // setup condition.
    for (_, entry) in g.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }

    // scenario:
    // - offset0/first process is overrun.
    // 0
    {
        let _ = g.on_start(0);
        let _ = g.on_start(1);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Running);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Running);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);
    }
    // 1
    {
        let _ = g.on_start(20); // no effect due to dependency unmet.

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Running);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Running);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);
    }
    // 2
    {
        let _ = g.on_start(0);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Overrun); // mark as Overrun.
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Running);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::SkipPrev); // send SKIP for prev.
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::SkipPrev); // send SKIP for prev.
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::SkipPrev); // send SKIP for prev.
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::SkipPrev); // send SKIP for prev.

        let _ = g.on_start(1);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Overrun);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Overrun); // mark as Overrun.
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::SkipPrev);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::SkipPrev); // already skipped.
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::SkipPrev); // already skipped.
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::SkipPrev); // already skipped.

        let _ = g.on_ready(10);
        let _ = g.on_ready(11);
        let _ = g.on_ready(20);
        let _ = g.on_ready(30);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Overrun);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Overrun);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Skip); // send SKIP for current.

        let _ = g.on_ready(10);
        let _ = g.on_ready(11);
        let _ = g.on_ready(20);
        let _ = g.on_ready(30);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Overrun);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Overrun);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);

        let _ = g.on_done(0);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::SkipPrev); // send OK.
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Overrun);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);

        let _ = g.on_done(1);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::SkipPrev);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::SkipPrev); // send OK.
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);

        let _ = g.on_ready(0);
        let _ = g.on_ready(1);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);

        let _ = g.on_ready(0);
        let _ = g.on_ready(1);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);
    }
    // 3
    {
        let _ = g.on_start(20); // no effect due to dependency unmet.

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);
    }
}

#[test]
fn test_skip2() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut g = ProcessGraph::new(create_entries());

    // setup condition.
    for (_, entry) in g.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }

    // scenario:
    // - offset0/second process is overrun.
    // 0
    {
        let _ = g.on_start(0);
        let _ = g.on_done(0);
        let _ = g.on_ready(0);
        let _ = g.on_start(1);
        let _ = g.on_done(1);
        let _ = g.on_ready(1);

        let _ = g.on_done(11);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Running);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Idle);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);
    }
    // 1
    {
        let _ = g.on_start(20); // no effect due to dependency unmet.

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Running);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Idle);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);
    }
    // 2
    {
        let _ = g.on_start(0);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Overrun); // mark as Overrun.
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::SkipPrev); // mark as Skip.
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::SkipPrev); // send SKIP for prev.
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::SkipPrev); // send SKIP for prev.

        let _ = g.on_start(1);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Skip);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Overrun);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::SkipPrev); // already skipped.
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::SkipPrev); // already skipped.
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::SkipPrev); // already skipped.

        let _ = g.on_ready(11);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Skip);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Skip);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Overrun);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::SkipPrev);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::SkipPrev);

        let _ = g.on_ready(0);
        let _ = g.on_ready(1);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Overrun);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Skip);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::SkipPrev);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::SkipPrev);

        let _ = g.on_ready(20);
        let _ = g.on_ready(30);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Overrun);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Skip);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Skip);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Skip);

        let _ = g.on_ready(11);
        let _ = g.on_ready(20);
        let _ = g.on_ready(30);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Overrun);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);

        let _ = g.on_done(10);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::SkipPrev); // send OK.
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);

        let _ = g.on_ready(10);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);

        let _ = g.on_ready(10);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);
    }
    // 3
    {
        let _ = g.on_start(20); // no effect due to dependency unmet.

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);
    }
}

#[test]
fn test_skip3() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut g = ProcessGraph::new(create_entries());

    // setup condition.
    for (_, entry) in g.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }

    // scenario:
    // - offset1/first process is overrun.
    // 0
    {
        let _ = g.on_start(0);
        let _ = g.on_done(0);
        let _ = g.on_ready(0);
        let _ = g.on_start(1);
        let _ = g.on_done(1);
        let _ = g.on_ready(1);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Running);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Running);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);

        let _ = g.on_done(10);
        let _ = g.on_ready(10);
        let _ = g.on_done(11);
        let _ = g.on_ready(11);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);
    }
    // 1
    {
        let _ = g.on_start(20);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Running);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);
    }
    // 2
    {
        let _ = g.on_start(0);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Overrun); // mark as Overrun.
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::SkipPrev); // send SKIP for prev.

        let _ = g.on_start(1);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Skip);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Skip);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Skip); // already skipped.
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Overrun); // already skipped.
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::SkipPrev); // already skipped.

        let _ = g.on_ready(0);
        let _ = g.on_ready(1);
        let _ = g.on_ready(10);
        let _ = g.on_ready(11);
        let _ = g.on_ready(30);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Overrun);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Skip);

        let _ = g.on_ready(30);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Overrun);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);

        let _ = g.on_done(20);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::SkipPrev);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);

        let _ = g.on_ready(20);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Skip);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);

        let _ = g.on_ready(20);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);
    }
    // 3
    {
        let _ = g.on_start(20); // no effect due to dependency unmet.

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);
    }
}

#[test]
fn test_skip4() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut g = ProcessGraph::new(create_entries());

    // setup condition.
    for (_, entry) in g.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }

    // scenario:
    // - offset1/second process is overrun.
    // 0
    {
        let _ = g.on_start(0);
        let _ = g.on_done(0);
        let _ = g.on_ready(0);
        let _ = g.on_start(1);
        let _ = g.on_done(1);
        let _ = g.on_ready(1);

        let _ = g.on_done(10);
        let _ = g.on_ready(10);
        let _ = g.on_done(11);
        let _ = g.on_ready(11);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);
    }
    // 1
    {
        let _ = g.on_start(20);
        let _ = g.on_done(20);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Idle);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Running);

        let _ = g.on_ready(20);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Running);
    }
    // 2
    {
        let _ = g.on_start(0);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready); // send SKIP for current.
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Overrun); // mark as Overrun.

        let _ = g.on_start(1);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Skip);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Skip); // send SKIP for current.
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Skip);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Skip); // already skipped.
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Skip); // already skipped.
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Overrun); // already skipped.

        let _ = g.on_ready(0);
        let _ = g.on_ready(1);
        let _ = g.on_ready(10);
        let _ = g.on_ready(11);
        let _ = g.on_ready(20);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Overrun);

        let _ = g.on_done(30);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::SkipPrev); // send OK.

        let _ = g.on_ready(30);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Skip); // send SKIP for current.

        let _ = g.on_ready(30);

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);
    }
    // 3
    {
        let _ = g.on_start(20); // no effect due to dependency unmet.

        assert_eq!(g.entries.get(&0).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&1).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&10).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&11).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&20).unwrap().state, ProcessState::Ready);
        assert_eq!(g.entries.get(&30).unwrap().state, ProcessState::Ready);
    }
}
