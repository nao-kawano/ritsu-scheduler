#[cfg(test)]
use super::*;

fn create_entries() -> HashMap<u16, ProcessEntry> {
    let mut entries: HashMap<u16, ProcessEntry> = HashMap::new();
    entries.insert(0, ProcessEntry::new(0, &vec![], false));
    entries.insert(1, ProcessEntry::new(1, &vec![], false));
    entries.insert(10, ProcessEntry::new(10, &vec![0], true));
    entries.insert(11, ProcessEntry::new(11, &vec![0, 1], true));
    entries.insert(20, ProcessEntry::new(20, &vec![10, 11], false));
    entries
}

#[test]
fn test_new() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let graph = ProcessGraph::new(create_entries());

    // setup condition.
    // do nothing.

    // check result.
    assert_eq!(graph.entries.len(), 5);
    assert_eq!(graph.graph_start.len(), 3);
    assert_eq!(graph.graph_start.contains(&0), true);
    assert_eq!(graph.graph_start.contains(&1), true);
    assert_eq!(graph.graph_forward.len(), 4);
    assert_eq!(graph.graph_forward.get(&0).unwrap().contains(&10), true);
    assert_eq!(graph.graph_forward.get(&0).unwrap().contains(&11), true);
    assert_eq!(graph.graph_forward.get(&1).unwrap().contains(&11), true);
    assert_eq!(graph.graph_forward.get(&10).unwrap().contains(&20), true);
    assert_eq!(graph.graph_forward.get(&11).unwrap().contains(&20), true);
}

#[test]
fn test_reset_state() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut graph = ProcessGraph::new(create_entries());

    // setup condition.
    for (_, entry) in graph.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }

    // send event.
    graph.reset_state();

    // check result.
    assert_eq!(graph.entries.get(&0).unwrap().state, ProcessState::Idle);
    assert_eq!(graph.entries.get(&1).unwrap().state, ProcessState::Idle);
    assert_eq!(graph.entries.get(&10).unwrap().state, ProcessState::Idle);
    assert_eq!(graph.entries.get(&11).unwrap().state, ProcessState::Idle);
    assert_eq!(graph.entries.get(&20).unwrap().state, ProcessState::Idle);
}

#[test]
fn test_on_start_normal() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut graph = ProcessGraph::new(create_entries());

    // setup condition.
    for (_, entry) in graph.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }

    // send event.
    let result = graph.on_start(0);
    // check result.
    assert_eq!(result.is_ok(), true);
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 0);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Ready);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Running);
    }

    // send event.
    let result = graph.on_start(1);
    // check result.
    assert_eq!(result.is_ok(), true);
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 1);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Ready);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Running);
    }

    // send event.
    let result = graph.on_start(10);
    // check result.
    assert_eq!(result.is_ok(), false);

    // send event.
    let result = graph.on_start(100);
    // check result.
    assert_eq!(result.is_ok(), false);
}

#[test]
fn test_on_ready_normal() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut graph = ProcessGraph::new(create_entries());

    // setup condition.
    for (_, entry) in graph.entries.iter_mut() {
        entry.state = ProcessState::Idle;
    }
    graph.entries.get_mut(&1).unwrap().state = ProcessState::Ready;

    // send event.
    let result = graph.on_ready(0);
    // check result.
    assert_eq!(result.is_ok(), true);
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 0);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Idle);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Ready);
    }

    // send event.
    let result = graph.on_ready(1);
    // check result.
    assert_eq!(result.is_err(), true);

    // send event.
    let result = graph.on_ready(10);
    // check result.
    assert_eq!(result.is_ok(), true);
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 10);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Idle);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Ready);
    }

    // send event.
    let result = graph.on_ready(11);
    // check result.
    assert_eq!(result.is_ok(), true);
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 11);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Idle);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Ready);
    }

    // send event.
    let result = graph.on_ready(20);
    // check result.
    assert_eq!(result.is_ok(), true);
    if let Ok(changes) = &result {
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.get(0).unwrap().pid, 20);
        assert_eq!(changes.get(0).unwrap().before, ProcessState::Idle);
        assert_eq!(changes.get(0).unwrap().after, ProcessState::Ready);
    }

    // send event.
    let result = graph.on_ready(100);
    // check result.
    assert_eq!(result.is_ok(), false);
}

#[test]
fn test_on_done_normal() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut graph = ProcessGraph::new(create_entries());

    // setup condition.
    for (_, entry) in graph.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }
    graph.entries.get_mut(&0).unwrap().state = ProcessState::Running;
    graph.entries.get_mut(&1).unwrap().state = ProcessState::Running;

    // send event.
    let result = graph.on_done(0);
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
    let result = graph.on_done(1);
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
    let result = graph.on_done(100);
    // check result.
    assert_eq!(result.is_ok(), false);
}

// -----------------------------------------------------------------------------

#[test]
fn test_skip1() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut graph = ProcessGraph::new(create_entries());

    // setup condition.
    for (_, entry) in graph.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }

    // send event.
    let _ = graph.on_start(0);
    let _ = graph.on_start(1);
    let _ = graph.on_start(0);

    // check result.
    assert_eq!(graph.entries.get(&0).unwrap().state, ProcessState::Overrun);
    assert_eq!(graph.entries.get(&1).unwrap().state, ProcessState::Overrun);
    assert_eq!(graph.entries.get(&10).unwrap().state, ProcessState::Idle);
    assert_eq!(graph.entries.get(&11).unwrap().state, ProcessState::Idle);
    assert_eq!(graph.entries.get(&20).unwrap().state, ProcessState::Idle);
}

#[test]
fn test_skip2() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut graph = ProcessGraph::new(create_entries());

    // setup condition.
    for (_, entry) in graph.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }

    // send event.
    // 1.
    let _ = graph.on_start(0);
    let _ = graph.on_done(0);
    let _ = graph.on_ready(0);
    let _ = graph.on_start(1);
    let _ = graph.on_done(1);
    let _ = graph.on_ready(1);
    let _ = graph.on_done(10);
    let _ = graph.on_ready(10);
    // 2.
    let _ = graph.on_start(0);
    let _ = graph.on_start(1);
    //
    let _ = graph.on_done(0);
    let _ = graph.on_done(1);

    // check result.
    assert_eq!(graph.entries.get(&0).unwrap().state, ProcessState::Idle);
    assert_eq!(graph.entries.get(&1).unwrap().state, ProcessState::Idle);
    assert_eq!(graph.entries.get(&10).unwrap().state, ProcessState::Running);
    assert_eq!(graph.entries.get(&11).unwrap().state, ProcessState::Overrun);
    assert_eq!(graph.entries.get(&20).unwrap().state, ProcessState::Idle);
}

#[test]
fn test_skip3() {
    // create objects.
    let _ = env_logger::builder().is_test(true).try_init();
    let mut graph = ProcessGraph::new(create_entries());

    // setup condition.
    for (_, entry) in graph.entries.iter_mut() {
        entry.state = ProcessState::Ready;
    }

    // send event.
    // 1.
    let _ = graph.on_start(0);
    let _ = graph.on_done(0);
    let _ = graph.on_ready(0);
    let _ = graph.on_start(1);
    let _ = graph.on_done(1);
    let _ = graph.on_ready(1);
    let _ = graph.on_done(10);
    let _ = graph.on_done(11);
    //
    let _ = graph.on_start(0);
    let _ = graph.on_start(1);
    let _ = graph.on_done(0);

    // check result.
    assert_eq!(graph.entries.get(&0).unwrap().state, ProcessState::Idle);
    assert_eq!(graph.entries.get(&1).unwrap().state, ProcessState::Overrun);
    assert_eq!(graph.entries.get(&10).unwrap().state, ProcessState::Skip);
    assert_eq!(graph.entries.get(&11).unwrap().state, ProcessState::Skip);
    assert_eq!(graph.entries.get(&20).unwrap().state, ProcessState::Overrun);
}
