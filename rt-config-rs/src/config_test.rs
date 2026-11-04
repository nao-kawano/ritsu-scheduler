// Copyright 2026 Naoyuki Kawano
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// =============================================================================
#[cfg(test)]
use super::*;

#[test]
fn test_client_config_validation() {
    // Valid config.
    assert!(ClientConfig::new(1, 2).is_ok());

    // Invalid CID.
    assert!(ClientConfig::new(CLIENT_ID_MAX + 1, 2).is_err());

    // Invalid interval.
    assert!(ClientConfig::new(1, 0).is_err());

    // Invalid offset.
    assert!(ClientConfig::builder(1, 2).offset(2).build().is_err());

    // Invalid depends.
    assert!(
        ClientConfig::builder(1, 2)
            .depends(vec![CLIENT_ID_MAX + 1])
            .build()
            .is_err()
    );

    // Self-dependency.
    assert!(
        ClientConfig::builder(1, 2)
            .depends(vec![1])
            .build()
            .is_err()
    );

    // Duplicate dependency.
    assert!(
        ClientConfig::builder(1, 2)
            .depends(vec![10, 10])
            .build()
            .is_err()
    );
}

#[test]
fn test_client_config_error() {
    let invalid_cid = CLIENT_ID_MAX + 1;
    let err = ClientConfig::builder(invalid_cid, 1).build().unwrap_err();
    assert_eq!(err.client_id, invalid_cid);
    assert_eq!(
        err.errors,
        vec![format!("Client ID {:03} is too large", invalid_cid)]
    );
    assert_eq!(
        err.to_string(),
        format!(
            "[ClientConfig CID:{:03}] Client ID {:03} is too large",
            invalid_cid, invalid_cid
        )
    );
}

#[test]
fn test_client_config_builder() {
    let name = "Test Process";
    let config = ClientConfig::builder(1, 2)
        .offset(1)
        .depends(vec![0])
        .display_name(name)
        .expected_duration_ms(10)
        .build()
        .unwrap();
    assert_eq!(config.client_id, 1);
    assert_eq!(config.interval, 2);
    assert_eq!(config.offset, 1);
    assert_eq!(config.depends, vec![0]);
    assert_eq!(config.display_name, name);
    assert_eq!(config.expected_duration_ms, 10);
}

fn create_config(client_configs: Vec<ClientConfig>) -> SchedulerConfig {
    SchedulerConfig {
        server_config: ServerConfig {
            port: 8080,
            cycle_time_ms: 100,
            ..Default::default()
        },
        client_configs,
    }
}

#[test]
fn test_get_client_rules_valid() {
    let scheduler_config = create_config(vec![
        ClientConfig::new(0, 2).unwrap(),
        ClientConfig::builder(1, 2)
            .depends(vec![0])
            .build()
            .unwrap(), // Floating (same offset)
        ClientConfig::builder(2, 2)
            .offset(1)
            .depends(vec![1])
            .build()
            .unwrap(), // Non-floating (different offset)
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
        ClientConfig::new(0, 2).unwrap(),
        ClientConfig::builder(1, 3)
            .depends(vec![0])
            .build()
            .unwrap(), // Error: Different interval
        ClientConfig::builder(2, 2)
            .depends(vec![0])
            .build()
            .unwrap(), // Valid
        ClientConfig::builder(3, 2)
            .depends(vec![999])
            .build()
            .unwrap(), // Error: Missing CID 999
        ClientConfig::builder(4, 2)
            .depends(vec![2])
            .build()
            .unwrap(), // Valid
        ClientConfig::builder(5, 2)
            .depends(vec![6])
            .build()
            .unwrap(), // Error: CID 6 has future offset
        ClientConfig::builder(6, 2).offset(1).build().unwrap(),
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
            .any(|e| e.contains("different interval"))
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
            .any(|e| e.contains("larger offset"))
    );
}

#[test]
fn test_get_client_rules_duplicate_cid() {
    let scheduler_config = create_config(vec![
        ClientConfig::new(10, 2).unwrap(),
        ClientConfig::new(10, 2).unwrap(),
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
        ClientConfig::new(100, 1).unwrap(),
        ClientConfig::builder(101, 1)
            .depends(vec![100])
            .build()
            .unwrap(),
        // Mutual dependency (A <-> B)
        ClientConfig::builder(10, 1)
            .depends(vec![11])
            .build()
            .unwrap(),
        ClientConfig::builder(11, 1)
            .depends(vec![10])
            .build()
            .unwrap(),
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
        ClientConfig::new(100, 1).unwrap(),
        ClientConfig::builder(101, 1)
            .depends(vec![100])
            .build()
            .unwrap(),
        // Whole loop (A -> C -> B -> A)
        ClientConfig::builder(20, 1)
            .depends(vec![22])
            .build()
            .unwrap(),
        ClientConfig::builder(21, 1)
            .depends(vec![20])
            .build()
            .unwrap(),
        ClientConfig::builder(22, 1)
            .depends(vec![21])
            .build()
            .unwrap(),
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
        ClientConfig::new(100, 1).unwrap(),
        ClientConfig::builder(101, 1)
            .depends(vec![100])
            .build()
            .unwrap(),
        // Mid-path loop (A -> B <-> C)
        ClientConfig::new(30, 1).unwrap(), // healthy start
        ClientConfig::builder(31, 1)
            .depends(vec![30, 32])
            .build()
            .unwrap(), // depends on A AND C
        ClientConfig::builder(32, 1)
            .depends(vec![31])
            .build()
            .unwrap(), // depends on B
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

#[test]
fn test_server_config_logging_defaults() {
    let toml_str = r#"
        port = 8080
        cycle_time_ms = 50
    "#;
    let config: ServerConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.port, 8080);
    assert_eq!(config.cycle_time_ms, 50);
    assert_eq!(config.stats_interval_cycle, 0);
    assert_eq!(config.log_level, "");
    assert_eq!(config.log_file_dir, "");
    assert_eq!(config.log_file_level, "");
}

#[test]
fn test_server_config_logging_custom_values() {
    let toml_str = r#"
        port = 7878
        cycle_time_ms = 100
        stats_interval_cycle = 200
        log_level = "debug"
        log_file_dir = "./custom_logs"
        log_file_level = "trace"
    "#;
    let config: ServerConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.port, 7878);
    assert_eq!(config.cycle_time_ms, 100);
    assert_eq!(config.stats_interval_cycle, 200);
    assert_eq!(config.log_level, "debug");
    assert_eq!(config.log_file_dir, "./custom_logs");
    assert_eq!(config.log_file_level, "trace");

    // Verify round-trip serialization and deserialization.
    let serialized = toml::to_string(&config).unwrap();
    let deserialized: ServerConfig = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.log_level, "debug");
    assert_eq!(deserialized.log_file_dir, "./custom_logs");
    assert_eq!(deserialized.log_file_level, "trace");
}
