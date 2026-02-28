use crate::entry::{ProcessEntry, ProcessState};

#[test]
fn test_new() {
    let _ = env_logger::builder().is_test(true).try_init();

    // cycle trigger.
    let entry = ProcessEntry::new(1, &vec![], false);
    assert_eq!(entry.cid, 1);
    assert_eq!(entry.state, ProcessState::Idle);
    assert_eq!(entry.dependency_statuses.len(), 0);
    assert_eq!(entry.is_floating, false);

    // depends trigger.
    let entry = ProcessEntry::new(2, &vec![1, 3], true);
    assert_eq!(entry.cid, 2);
    assert_eq!(entry.state, ProcessState::Idle);
    assert_eq!(entry.dependency_statuses.len(), 2);
    assert_eq!(entry.dependency_statuses.get(&0), None);
    assert_eq!(entry.dependency_statuses.get(&1), Some(&false));
    assert_eq!(entry.dependency_statuses.get(&3), Some(&false));
    assert_eq!(entry.is_floating, true);
}

#[test]
fn test_set_state() {
    let _ = env_logger::builder().is_test(true).try_init();

    let mut entry = ProcessEntry::new(1, &vec![], false);
    assert_eq!(entry.state, ProcessState::Idle);

    // normal Idle -> Ready -> Running -> Idle flow.
    entry.state = ProcessState::Idle; // Ensure starting from Idle
    assert!(entry.set_state(ProcessState::Ready));
    assert!(entry.set_state(ProcessState::Running));
    assert!(entry.set_state(ProcessState::Idle));

    // Running -> Overrun -> Late -> Idle flow.
    entry.state = ProcessState::Ready; // Ensure starting from Ready
    assert!(entry.set_state(ProcessState::Running));
    assert!(entry.set_state(ProcessState::Overrun));
    assert!(entry.set_state(ProcessState::Late));
    assert!(entry.set_state(ProcessState::Idle));
    assert!(entry.set_state(ProcessState::Ready));

    // Idle -> Late -> Idle flow.
    entry.state = ProcessState::Idle; // Ensure starting from Idle
    assert!(entry.set_state(ProcessState::Late));
    assert!(entry.set_state(ProcessState::Idle));
    assert!(entry.set_state(ProcessState::Ready));

    // Ready -> Skip -> Ready
    entry.state = ProcessState::Ready;
    assert!(entry.set_state(ProcessState::Skip));
    assert!(entry.set_state(ProcessState::Ready));

    // Invalid transitions
    entry.state = ProcessState::Ready;
    assert!(!entry.set_state(ProcessState::Overrun)); // From Ready to Overrun is invalid
    assert!(!entry.set_state(ProcessState::Late)); // From Ready to Late is invalid

    entry.state = ProcessState::Running;
    assert!(!entry.set_state(ProcessState::Ready)); // Running to Ready is invalid
    assert!(!entry.set_state(ProcessState::Late)); // Running to Late is invalid
}

#[test]
fn test_is_dependent() {
    let _ = env_logger::builder().is_test(true).try_init();

    // cycle trigger.
    let entry = ProcessEntry::new(1, &vec![], false);
    assert_eq!(entry.is_dependent(), false);

    // depends trigger.
    let entry = ProcessEntry::new(2, &vec![1, 3], true);
    assert_eq!(entry.is_dependent(), true);
}

#[test]
fn test_is_dependency_met() {
    let _ = env_logger::builder().is_test(true).try_init();

    // cycle trigger.
    let entry = ProcessEntry::new(1, &vec![], false);
    assert_eq!(entry.is_dependency_met(), true);

    // depends trigger.
    let mut entry = ProcessEntry::new(2, &vec![1, 3], true);
    assert_eq!(entry.is_dependency_met(), false);

    entry.dependency_statuses.insert(1, true);
    assert_eq!(entry.is_dependency_met(), false);

    entry.dependency_statuses.insert(3, true);
    assert_eq!(entry.is_dependency_met(), true);
}

#[test]
fn test_mark_dependency_complete() {
    let _ = env_logger::builder().is_test(true).try_init();

    // cycle trigger.
    let mut entry = ProcessEntry::new(1, &vec![], false);
    entry.mark_dependency_complete(3); // no effect.
    assert_eq!(entry.is_dependency_met(), true);

    // depends trigger.
    let mut entry = ProcessEntry::new(2, &vec![1, 3], true);
    assert_eq!(entry.is_dependency_met(), false);

    entry.mark_dependency_complete(1);
    assert_eq!(entry.is_dependency_met(), false);

    entry.mark_dependency_complete(2);
    assert_eq!(entry.is_dependency_met(), false);

    entry.mark_dependency_complete(3);
    assert_eq!(entry.is_dependency_met(), true);
}

#[test]
fn test_reset_dependency_statuses() {
    let _ = env_logger::builder().is_test(true).try_init();

    // cycle trigger.
    let mut entry = ProcessEntry::new(1, &vec![], false);
    entry.reset_dependency_statuses(); // no effect.
    assert_eq!(entry.is_dependency_met(), true);

    // depends trigger.
    let mut entry = ProcessEntry::new(2, &vec![1, 3], true);
    entry.dependency_statuses.insert(1, true);
    entry.dependency_statuses.insert(3, true);
    assert_eq!(entry.is_dependency_met(), true);

    entry.reset_dependency_statuses();
    assert_eq!(entry.is_dependency_met(), false);
    assert_eq!(entry.dependency_statuses.len(), 2);
}
