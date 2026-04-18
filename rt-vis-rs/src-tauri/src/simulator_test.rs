use super::*;
use rt_config::{ClientConfig, SchedulerConfig, ServerConfig};

fn create_config(clients: Vec<ClientConfig>) -> SchedulerConfig {
    SchedulerConfig {
        server_config: ServerConfig {
            port: 8080,
            cycle_time_ms: 100,
            stats_interval_cycle: 1,
        },
        client_configs: clients,
    }
}

#[test]
fn test_simulate_plan_single_process() {
    let _ = env_logger::builder().is_test(true).try_init();

    let config = create_config(vec![ClientConfig::new(1, 1, 0, vec![], 40).unwrap()]);

    let result = simulate_plan(config).unwrap();

    // max_cycle is 1. max_manager_cycle is 2.
    // It should simulate manager_cycle 0 and 1.
    assert_eq!(result.executions.len(), 2);

    let ex0 = &result.executions[0];
    assert_eq!(ex0.cid, 1);
    assert_eq!(ex0.cycle, 0);
    assert_eq!(ex0.cycle_offset_ms, 0);
    assert_eq!(ex0.start_ms, 0);
    assert_eq!(ex0.duration_ms, 40);

    let ex1 = &result.executions[1];
    assert_eq!(ex1.cid, 1);
    assert_eq!(ex1.cycle, 1);
    assert_eq!(ex1.cycle_offset_ms, 0);
    assert_eq!(ex1.start_ms, 100);
    assert_eq!(ex1.duration_ms, 40);

    // Metrics should capture the starts and ends.
    // At 0ms: start 1 process. -> count = 1
    // At 40ms: end 1 process. -> count = 0
    // At 100ms: start 1 process. -> count = 1
    // At 140ms: end 1 process. -> count = 0
    assert_eq!(result.metrics.len(), 4);
    assert_eq!(result.metrics[0].time_ms, 0);
    assert_eq!(result.metrics[0].running_count, 1);
    assert_eq!(result.metrics[1].time_ms, 40);
    assert_eq!(result.metrics[1].running_count, 0);
    assert_eq!(result.metrics[2].time_ms, 100);
    assert_eq!(result.metrics[2].running_count, 1);
    assert_eq!(result.metrics[3].time_ms, 140);
    assert_eq!(result.metrics[3].running_count, 0);
}

#[test]
fn test_simulate_plan_dependencies() {
    let _ = env_logger::builder().is_test(true).try_init();

    // Process 1: takes 120ms.
    // Process 2: depends on 1, takes 50ms.
    let config = create_config(vec![
        ClientConfig::new(1, 2, 0, vec![], 120).unwrap(),
        ClientConfig::new(2, 2, 0, vec![1], 50).unwrap(),
    ]);

    let result = simulate_plan(config).unwrap();

    // Max cycle is 2, so it will simulate cycle 0, 1, 2. (max_cycle + 1)
    // Process 1 runs at cycle 0 and cycle 2. (2 times)
    // Process 2 runs at cycle 1. (1 time, after Process 1)
    // Total executions: 3
    assert_eq!(result.executions.len(), 3);

    // Filter executions by CID
    let mut execs_1: Vec<_> = result.executions.iter().filter(|e| e.cid == 1).collect();
    let mut execs_2: Vec<_> = result.executions.iter().filter(|e| e.cid == 2).collect();
    execs_1.sort_by_key(|e| e.start_ms);
    execs_2.sort_by_key(|e| e.start_ms);

    assert_eq!(execs_1.len(), 2);
    assert_eq!(execs_2.len(), 1);

    // Process 1 Cycle 0
    assert_eq!(execs_1[0].start_ms, 0);
    assert_eq!(execs_1[0].duration_ms, 120);

    // Process 2 Cycle 1 (Floating start after Process 1)
    assert_eq!(execs_2[0].start_ms, 120);
    assert_eq!(execs_2[0].duration_ms, 50);
    assert_eq!(execs_2[0].cycle_offset_ms, 20); // 20ms offset within cycle 1

    // Process 1 Cycle 2
    assert_eq!(execs_1[1].start_ms, 200);
    assert_eq!(execs_1[1].duration_ms, 120);

    // Metrics should capture the floating start.
    // 0ms: P1 starts (1).
    // 120ms: P1 ends, P2 starts (1).
    // 170ms: P2 ends (0).
    // 200ms: P1 starts (1).
    // (omit 320ms: P1 ends (0))
    assert_eq!(result.metrics.len(), 4);
    assert_eq!(result.metrics[0].time_ms, 0);
    assert_eq!(result.metrics[0].running_count, 1);
    assert_eq!(result.metrics[1].time_ms, 120);
    assert_eq!(result.metrics[1].running_count, 1);
    assert_eq!(result.metrics[2].time_ms, 170);
    assert_eq!(result.metrics[2].running_count, 0);
    assert_eq!(result.metrics[3].time_ms, 200);
    assert_eq!(result.metrics[3].running_count, 1);
}

#[test]
fn test_simulate_plan_offset() {
    let _ = env_logger::builder().is_test(true).try_init();

    // Process 1: cycle 2, offset 0
    // Process 2: cycle 2, offset 1
    let config = create_config(vec![
        ClientConfig::new(1, 2, 0, vec![], 10).unwrap(),
        ClientConfig::new(2, 2, 1, vec![], 10).unwrap(),
    ]);

    let result = simulate_plan(config).unwrap();

    // Max cycle is 2, so it will simulate cycle 0, 1, 2. (max_cycle + 1)
    // Process 1 runs at cycle 0 and cycle 2. (2 times)
    // Process 2 runs at cycle 1. (1 time)
    // Total executions: 3
    assert_eq!(result.executions.len(), 3);

    // Filter executions by CID
    let mut execs_1: Vec<_> = result.executions.iter().filter(|e| e.cid == 1).collect();
    let mut execs_2: Vec<_> = result.executions.iter().filter(|e| e.cid == 2).collect();
    execs_1.sort_by_key(|e| e.start_ms);
    execs_2.sort_by_key(|e| e.start_ms);

    assert_eq!(execs_1.len(), 2);
    assert_eq!(execs_2.len(), 1);

    // Process 1 Cycle 0
    assert_eq!(execs_1[0].start_ms, 0);
    assert_eq!(execs_1[0].duration_ms, 10);

    // Process 2 Cycle 1
    assert_eq!(execs_2[0].start_ms, 100);
    assert_eq!(execs_2[0].duration_ms, 10);

    // Process 1 Cycle 2
    assert_eq!(execs_1[1].start_ms, 200);
    assert_eq!(execs_1[1].duration_ms, 10);

    // Metrics should capture the offset start.
    // 0ms: P1 starts (1).
    // 10ms: P1 ends (0).
    // 100ms: P2 starts (1).
    // 110ms: P2 ends (0).
    // 200ms: P1 starts (1).
    // 210ms: P1 ends (0).
    assert_eq!(result.metrics.len(), 6);
    assert_eq!(result.metrics[0].time_ms, 0);
    assert_eq!(result.metrics[0].running_count, 1);
    assert_eq!(result.metrics[1].time_ms, 10);
    assert_eq!(result.metrics[1].running_count, 0);
    assert_eq!(result.metrics[2].time_ms, 100);
    assert_eq!(result.metrics[2].running_count, 1);
    assert_eq!(result.metrics[3].time_ms, 110);
    assert_eq!(result.metrics[3].running_count, 0);
    assert_eq!(result.metrics[4].time_ms, 200);
    assert_eq!(result.metrics[4].running_count, 1);
    assert_eq!(result.metrics[5].time_ms, 210);
    assert_eq!(result.metrics[5].running_count, 0);
}

#[test]
fn test_simulate_plan_concurrent_metrics() {
    let _ = env_logger::builder().is_test(true).try_init();

    // Process 1: cycle 2, offset 0, duration 150ms. (Runs at 0ms, 200ms)
    // Process 2: cycle 2, offset 1, duration 30ms. (Runs at 100ms, 300ms)
    let config = create_config(vec![
        ClientConfig::new(1, 2, 0, vec![], 150).unwrap(),
        ClientConfig::new(2, 2, 1, vec![], 30).unwrap(),
    ]);

    let result = simulate_plan(config).unwrap();

    // Max cycle is 2, so it will simulate cycle 0, 1, 2. (max_cycle + 1)
    // Process 1 runs at cycle 0 and cycle 2. (2 times)
    // Process 2 runs at cycle 1. (1 time)
    // Total executions: 3
    assert_eq!(result.executions.len(), 3);

    // Filter executions by CID
    let mut execs_1: Vec<_> = result.executions.iter().filter(|e| e.cid == 1).collect();
    let mut execs_2: Vec<_> = result.executions.iter().filter(|e| e.cid == 2).collect();
    execs_1.sort_by_key(|e| e.start_ms);
    execs_2.sort_by_key(|e| e.start_ms);

    assert_eq!(execs_1.len(), 2);
    assert_eq!(execs_2.len(), 1);

    // Process 1 Cycle 0
    assert_eq!(execs_1[0].start_ms, 0);
    assert_eq!(execs_1[0].duration_ms, 150);

    // Process 2 Cycle 1
    assert_eq!(execs_2[0].start_ms, 100);
    assert_eq!(execs_2[0].duration_ms, 30);

    // Process 1 Cycle 2
    assert_eq!(execs_1[1].start_ms, 200);
    assert_eq!(execs_1[1].duration_ms, 150);

    // Verify 0 -> 1 -> 2 -> 1 -> 0 pattern
    // 0ms: P1 starts (1)
    // 100ms: P2 starts (2)
    // 130ms: P2 ends (1)
    // 150ms: P1 ends (0)
    // 200ms: P1 starts (1)
    // (omit 300ms: P2 starts (2))
    // (omit 350ms: P1 ends (0))
    assert_eq!(result.metrics.len(), 5);
    assert_eq!(result.metrics[0].time_ms, 0);
    assert_eq!(result.metrics[0].running_count, 1);
    assert_eq!(result.metrics[1].time_ms, 100);
    assert_eq!(result.metrics[1].running_count, 2);
    assert_eq!(result.metrics[2].time_ms, 130);
    assert_eq!(result.metrics[2].running_count, 1);
    assert_eq!(result.metrics[3].time_ms, 150);
    assert_eq!(result.metrics[3].running_count, 0);
    assert_eq!(result.metrics[4].time_ms, 200);
    assert_eq!(result.metrics[4].running_count, 1);
}

#[test]
fn test_simulate_plan_default_duration() {
    let _ = env_logger::builder().is_test(true).try_init();

    let config = create_config(vec![
        ClientConfig::new(1, 1, 0, vec![], 0).unwrap(), // 0 duration
    ]);

    let result = simulate_plan(config).unwrap();

    // max_cycle is 1. max_manager_cycle is 2.
    // It should simulate manager_cycle 0 and 1.
    assert_eq!(result.executions.len(), 2);

    // check if duration_ms set to MIN_DURATION_MS from 0.
    assert_eq!(result.executions[0].duration_ms, MIN_DURATION_MS);

    // 0ms: P1 starts (1).
    // 5ms: P1 ends (0).
    // 100ms: P1 starts (1).
    // 105ms: P1 ends (0).
    assert_eq!(result.metrics.len(), 4);
    assert_eq!(result.metrics[0].time_ms, 0);
    assert_eq!(result.metrics[0].running_count, 1);
    assert_eq!(result.metrics[1].time_ms, 5);
    assert_eq!(result.metrics[1].running_count, 0);
    assert_eq!(result.metrics[2].time_ms, 100);
    assert_eq!(result.metrics[2].running_count, 1);
    assert_eq!(result.metrics[3].time_ms, 105);
    assert_eq!(result.metrics[3].running_count, 0);
}
