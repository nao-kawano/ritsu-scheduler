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
}

#[test]
fn test_get_client_rules_valid() {
    let server_config = ServerConfig {
        port: 8080,
        cycle_time_ms: 100,
        stats_interval_cycle: 0,
    };
    let client_configs = vec![
        ClientConfig::new(0, 2, 0, vec![], 0).unwrap(),
        ClientConfig::new(1, 2, 0, vec![0], 0).unwrap(), // Floating (same offset)
        ClientConfig::new(2, 2, 1, vec![1], 0).unwrap(), // Non-floating (different offset)
    ];
    let scheduler_config = SchedulerConfig {
        server_config,
        client_configs,
    };

    let rules = scheduler_config.get_client_rules().unwrap();
    assert_eq!(rules.len(), 3);
    assert_eq!(rules.get(&0).unwrap().is_floating, false);
    assert_eq!(rules.get(&1).unwrap().is_floating, true);
    assert_eq!(rules.get(&2).unwrap().is_floating, false);
}

#[test]
fn test_get_client_rules_invalid_all_errors() {
    let server_config = ServerConfig {
        port: 8080,
        cycle_time_ms: 100,
        stats_interval_cycle: 0,
    };
    let client_configs = vec![
        ClientConfig::new(0, 2, 0, vec![], 0).unwrap(),
        ClientConfig::new(1, 3, 0, vec![0], 0).unwrap(), // Error: Different cycle
        ClientConfig::new(2, 2, 0, vec![0], 0).unwrap(), // Valid
        ClientConfig::new(3, 2, 0, vec![999], 0).unwrap(), // Error: Missing CID 999
        ClientConfig::new(4, 2, 0, vec![2], 0).unwrap(), // Valid
        ClientConfig::new(5, 2, 0, vec![6], 0).unwrap(), // Error: CID 6 has future offset
        ClientConfig::new(6, 2, 1, vec![], 0).unwrap(),
    ];

    let scheduler_config = SchedulerConfig {
        server_config,
        client_configs,
    };

    let result = scheduler_config.get_client_rules();
    assert!(result.is_err());
    let errors = result.err().unwrap();

    // Check if all expected errors are collected.
    assert_eq!(errors.len(), 3);
    assert!(errors.iter().any(|e| e.contains("different cycle")));
    assert!(errors.iter().any(|e| e.contains("does not exist")));
    assert!(errors.iter().any(|e| e.contains("larger cycle_offset")));
}
