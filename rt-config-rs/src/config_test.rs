#[cfg(test)]
use super::*;

#[test]
fn test_client_config_validation() {
    // Valid config.
    assert!(ClientConfig::new(1, 2, 0, vec![], 0).is_ok());

    // Invalid CID.
    assert!(ClientConfig::new(CLIENT_ID_MAX + 1, 2, 0, vec![], 0).is_err());

    // Invalid cycle.
    assert!(ClientConfig::new(1, 0, 0, vec![], 0).is_err());

    // Invalid offset.
    assert!(ClientConfig::new(1, 2, 2, vec![], 0).is_err());

    // Invalid depends.
    assert!(ClientConfig::new(1, 2, 0, vec![CLIENT_ID_MAX + 1], 0).is_err());

    // Self-dependency.
    assert!(ClientConfig::new(1, 2, 0, vec![1], 0).is_err());

    // Duplicate dependency.
    assert!(ClientConfig::new(1, 2, 0, vec![10, 10], 0).is_err());
}

fn create_config(client_configs: Vec<ClientConfig>) -> SchedulerConfig {
    SchedulerConfig {
        server_config: ServerConfig {
            port: 8080,
            cycle_time_ms: 100,
            stats_interval_cycle: 0,
        },
        client_configs,
    }
}

#[test]
fn test_get_client_rules_valid() {
    let scheduler_config = create_config(vec![
        ClientConfig::new(0, 2, 0, vec![], 0).unwrap(),
        ClientConfig::new(1, 2, 0, vec![0], 0).unwrap(), // Floating (same offset)
        ClientConfig::new(2, 2, 1, vec![1], 0).unwrap(), // Non-floating (different offset)
    ]);

    let rules = scheduler_config.get_client_rules();
    assert_eq!(rules.len(), 3);
    assert_eq!(rules.get(&0).unwrap().is_floating, false);
    assert_eq!(rules.get(&1).unwrap().is_floating, true);
    assert_eq!(rules.get(&2).unwrap().is_floating, false);
}

#[test]
fn test_get_client_rules_invalid_all_errors() {
    let scheduler_config = create_config(vec![
        ClientConfig::new(0, 2, 0, vec![], 0).unwrap(),
        ClientConfig::new(1, 3, 0, vec![0], 0).unwrap(), // Error: Different cycle
        ClientConfig::new(2, 2, 0, vec![0], 0).unwrap(), // Valid
        ClientConfig::new(3, 2, 0, vec![999], 0).unwrap(), // Error: Missing CID 999
        ClientConfig::new(4, 2, 0, vec![2], 0).unwrap(), // Valid
        ClientConfig::new(5, 2, 0, vec![6], 0).unwrap(), // Error: CID 6 has future offset
        ClientConfig::new(6, 2, 1, vec![], 0).unwrap(),
    ]);

    let result = scheduler_config.validate();
    assert!(result.is_err());
    let errors = result.err().unwrap();

    // Check if all expected errors are collected for specific CIDs.
    assert_eq!(errors.len(), 3);
    assert!(
        errors
            .get(&1)
            .unwrap()
            .iter()
            .any(|e| e.contains("different cycle"))
    );
    assert!(
        errors
            .get(&3)
            .unwrap()
            .iter()
            .any(|e| e.contains("does not exist"))
    );
    assert!(
        errors
            .get(&5)
            .unwrap()
            .iter()
            .any(|e| e.contains("larger cycle_offset"))
    );
}

#[test]
fn test_get_client_rules_duplicate_cid() {
    let scheduler_config = create_config(vec![
        ClientConfig::new(10, 2, 0, vec![], 0).unwrap(),
        ClientConfig::new(10, 2, 0, vec![], 0).unwrap(),
    ]);

    let result = scheduler_config.validate();
    assert!(result.is_err());
    let errors = result.err().unwrap();

    assert_eq!(errors.len(), 1);
    assert!(
        errors
            .get(&10)
            .unwrap()
            .iter()
            .any(|e| e.contains("Duplicate client ID"))
    );
}

#[test]
fn test_circular_mutual_dependency() {
    let scheduler_config = create_config(vec![
        // Healthy Tree (X -> Y)
        ClientConfig::new(100, 1, 0, vec![], 0).unwrap(),
        ClientConfig::new(101, 1, 0, vec![100], 0).unwrap(),
        // Mutual dependency (A <-> B)
        ClientConfig::new(10, 1, 0, vec![11], 0).unwrap(),
        ClientConfig::new(11, 1, 0, vec![10], 0).unwrap(),
    ]);

    let errors = scheduler_config
        .validate()
        .err()
        .expect("Should find errors");

    // Healthy processes must NOT have errors
    assert!(!errors.contains_key(&100));
    assert!(!errors.contains_key(&101));

    // Broken processes must have circular error
    let expected_msg = "Circular dependency detected: 010 -> 011 -> 010";
    assert!(
        errors
            .get(&10)
            .unwrap()
            .iter()
            .any(|e| e.contains(expected_msg))
    );
    assert!(
        errors
            .get(&11)
            .unwrap()
            .iter()
            .any(|e| e.contains(expected_msg))
    );
}

#[test]
fn test_circular_whole_loop() {
    let scheduler_config = create_config(vec![
        // Healthy Tree (X -> Y)
        ClientConfig::new(100, 1, 0, vec![], 0).unwrap(),
        ClientConfig::new(101, 1, 0, vec![100], 0).unwrap(),
        // Whole loop (A -> C -> B -> A)
        ClientConfig::new(20, 1, 0, vec![22], 0).unwrap(),
        ClientConfig::new(21, 1, 0, vec![20], 0).unwrap(),
        ClientConfig::new(22, 1, 0, vec![21], 0).unwrap(),
    ]);

    let errors = scheduler_config
        .validate()
        .err()
        .expect("Should find errors");

    // Healthy processes must NOT have errors
    assert!(!errors.contains_key(&100));
    assert!(!errors.contains_key(&101));

    let expected_msg = "Circular dependency detected: 020 -> 022 -> 021 -> 020";
    assert!(
        errors
            .get(&20)
            .unwrap()
            .iter()
            .any(|e| e.contains(expected_msg))
    );
    assert!(
        errors
            .get(&21)
            .unwrap()
            .iter()
            .any(|e| e.contains(expected_msg))
    );
    assert!(
        errors
            .get(&22)
            .unwrap()
            .iter()
            .any(|e| e.contains(expected_msg))
    );
}

#[test]
fn test_circular_mid_path_loop() {
    let scheduler_config = create_config(vec![
        // Healthy Tree (X -> Y)
        ClientConfig::new(100, 1, 0, vec![], 0).unwrap(),
        ClientConfig::new(101, 1, 0, vec![100], 0).unwrap(),
        // Mid-path loop (A -> B <-> C)
        ClientConfig::new(30, 1, 0, vec![], 0).unwrap(), // healthy start
        ClientConfig::new(31, 1, 0, vec![30, 32], 0).unwrap(), // depends on A AND C
        ClientConfig::new(32, 1, 0, vec![31], 0).unwrap(), // depends on B
    ]);

    let errors = scheduler_config
        .validate()
        .err()
        .expect("Should find errors");

    // Healthy processes must NOT have errors
    assert!(!errors.contains_key(&100));
    assert!(!errors.contains_key(&101));
    assert!(!errors.contains_key(&30));

    // Broken processes must have circular error
    let expected_msg = "Circular dependency detected: 031 -> 032 -> 031";
    assert!(
        errors
            .get(&31)
            .unwrap()
            .iter()
            .any(|e| e.contains(expected_msg))
    );
    assert!(
        errors
            .get(&32)
            .unwrap()
            .iter()
            .any(|e| e.contains(expected_msg))
    );
}
