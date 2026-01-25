use crate::entry::{ProcessEntry, ProcessState};

#[test]
fn test_new() {
    let _ = env_logger::builder().is_test(true).try_init();

    // cycle trigger.
    let entry = ProcessEntry::new(1, &vec![], false);
    assert_eq!(entry.pid, 1);
    assert_eq!(entry.state, ProcessState::Idle);
    assert_eq!(entry.depends_on.len(), 0);
    assert_eq!(entry.is_floating, false);

    // depends trigger.
    let entry = ProcessEntry::new(2, &vec![1, 3], true);
    assert_eq!(entry.pid, 2);
    assert_eq!(entry.state, ProcessState::Idle);
    assert_eq!(entry.depends_on.len(), 2);
    assert_eq!(entry.depends_on.get(&0), None);
    assert_eq!(entry.depends_on.get(&1), Some(&false));
    assert_eq!(entry.depends_on.get(&3), Some(&false));
    assert_eq!(entry.is_floating, true);
}

#[test]
fn test_set_state() {
    let _ = env_logger::builder().is_test(true).try_init();

    let mut entry = ProcessEntry::new(1, &vec![], false);
    assert_eq!(entry.state, ProcessState::Idle);

    // normal.
    assert!(entry.set_state(ProcessState::Ready));
    assert!(entry.set_state(ProcessState::Running));
    assert!(entry.set_state(ProcessState::Idle));
    assert!(entry.set_state(ProcessState::Ready));
    // skip in idle.
    assert!(entry.set_state(ProcessState::Running));
    assert!(entry.set_state(ProcessState::Idle));
    assert!(entry.set_state(ProcessState::SkipPrev));
    assert!(entry.set_state(ProcessState::Skip));
    assert!(entry.set_state(ProcessState::Ready));
    // skip in ready for current.
    assert!(entry.set_state(ProcessState::Skip));
    assert!(entry.set_state(ProcessState::Ready));
    // skip in ready for previous.
    assert!(entry.set_state(ProcessState::SkipPrev));
    assert!(entry.set_state(ProcessState::Skip));
    assert!(entry.set_state(ProcessState::Ready));
    // skip in running.
    assert!(entry.set_state(ProcessState::Running));
    assert!(entry.set_state(ProcessState::Overrun));
    assert!(entry.set_state(ProcessState::SkipPrev));
    assert!(entry.set_state(ProcessState::Skip));
    assert!(entry.set_state(ProcessState::Ready));
}

#[test]
fn test_has_depends() {
    let _ = env_logger::builder().is_test(true).try_init();

    // cycle trigger.
    let entry = ProcessEntry::new(1, &vec![], false);
    assert_eq!(entry.has_depends(), false);

    // depends trigger.
    let entry = ProcessEntry::new(2, &vec![1, 3], true);
    assert_eq!(entry.has_depends(), true);
}

#[test]
fn test_is_depends_ok() {
    let _ = env_logger::builder().is_test(true).try_init();

    // cycle trigger.
    let entry = ProcessEntry::new(1, &vec![], false);
    assert_eq!(entry.is_depends_ok(), true);

    // depends trigger.
    let mut entry = ProcessEntry::new(2, &vec![1, 3], true);
    assert_eq!(entry.is_depends_ok(), false);

    entry.depends_on.insert(1, true);
    assert_eq!(entry.is_depends_ok(), false);

    entry.depends_on.insert(3, true);
    assert_eq!(entry.is_depends_ok(), true);
}

#[test]
fn test_update_depend() {
    let _ = env_logger::builder().is_test(true).try_init();

    // cycle trigger.
    let mut entry = ProcessEntry::new(1, &vec![], false);
    entry.update_depend(3); // no effect.
    assert_eq!(entry.is_depends_ok(), true);

    // depends trigger.
    let mut entry = ProcessEntry::new(2, &vec![1, 3], true);
    assert_eq!(entry.is_depends_ok(), false);

    entry.update_depend(1);
    assert_eq!(entry.is_depends_ok(), false);

    entry.update_depend(2);
    assert_eq!(entry.is_depends_ok(), false);

    entry.update_depend(3);
    assert_eq!(entry.is_depends_ok(), true);
}

#[test]
fn test_clear_depends() {
    let _ = env_logger::builder().is_test(true).try_init();

    // cycle trigger.
    let mut entry = ProcessEntry::new(1, &vec![], false);
    entry.clear_depends(); // no effect.
    assert_eq!(entry.is_depends_ok(), true);

    // depends trigger.
    let mut entry = ProcessEntry::new(2, &vec![1, 3], true);
    entry.depends_on.insert(1, true);
    entry.depends_on.insert(3, true);
    assert_eq!(entry.is_depends_ok(), true);

    entry.clear_depends();
    assert_eq!(entry.is_depends_ok(), false);
    assert_eq!(entry.depends_on.len(), 2);
}
