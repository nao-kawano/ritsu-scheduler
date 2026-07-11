# Process Configuration

The scheduler controls process execution based on periodic cycles and process dependencies.

Each process can be configured with a trigger to determine when it runs.
These triggers can be time-based cycles or the completion of other processes.

## Cycle Terminology

To understand how the scheduler manages execution, we distinguish between the following concepts:

### 1. Time / Tick (Scheduler Clock)
- **`global_cycle`**: The absolute cycle count tracked from the start of the scheduler's event loop (e.g., time-based tick interval).
- **`running_cycle`**: The execution cycle count tracked from when the scheduler transitions to the `Running` state and actually schedules process executions. It starts at `-1` (representing "not started") and increments to `0` at the first running cycle start.

### 2. Scheduling Interval and Offset
- **`cycle` (process's cycle)**: The configuration parameter that defines the scheduling interval of the process (e.g., run every `1` cycle, run every `2` cycles).
- **`cycle_offset`**: The offset in running cycles applied to the start timing of the process.

<p align="center">
  <img src="assets/cycle-terminology.drawio.svg" alt="Overview of Cycles" width="800">
</p>

## Configuration File

The scheduler execution rules are defined in a TOML configuration file. A sample template is available at `rt-server-rs/config.toml`.

By default, the server reads `config.toml` from the current working directory, but a custom file path can also be specified via command-line options. Please refer to the server command help for details.

## Server Configuration (Section: `[server_config]`)

### Port (Key: `port`)

- Specify the port number for the scheduler.

### Base Cycle Time (Key: `cycle_time_ms`)

- Specify cycles using a time-based interval in milliseconds (e.g., 50ms).
  - The scheduler checks which processes should be run at each cycle.

- **NOTE:** This is currently implemented using an interval-based trigger.
  - The underlying architecture is designed to support various cycle triggers for future extensibility.

### Statistics Log Interval (in cycles) (Key: `stats_interval_cycle`)

- Specifies the interval in running cycles at which the scheduler logs performance statistics for each client.
  - A value of `0` or omitting the parameter disables all statistics logging (including the final shutdown log).
  - If enabled,
    - Logs include metrics like execution time (min, max, average), skip count, late count, and overrun count.
    - Final statistics are also logged when the scheduler shuts down, regardless of the cycle count.

## Client Configuration (Section: `[[client_configs]]`)

Per client configuration below:

### Client ID (Key: `client_id`)

- Specifies a unique ID for this process.
  - Each client has a unique ID represented as an integer (`u16`, from `0` to `999`).
  - This ID is used to identify the client in the scheduler.

### Display Name (Key: `display_name`)

- Specifies a human-readable name for this process.
  - This value is used by visualization tools (like Ritsu Vis) to make it easier to identify processes.
  - It does not affect the actual execution of the process in the scheduler.
  - **Maximum length is 20 characters** to ensure proper display in the UI.
  - If not specified, it defaults to an empty string.

### Scheduling Interval (in cycles) (Key: `cycle`)

- Specifies how often to run the process in terms of running cycles (e.g., every cycle, every other cycle).
  - `1` means run every running cycle. If the cycle time is 50ms, run every 50ms.
  - `2` means run every other running cycle. If the cycle time is 50ms, run every 100ms.

### Scheduling Offset (in cycles) (Key: `cycle_offset`)

- Specifies the offset in running cycles for when this client starts running.
  - Useful when you want to alternate the execution of clients to distribute the load evenly.
    - `0`: Start running at the beginning of the specified running cycle.
    - `1`: Start running one running cycle later than the beginning of the specified running cycle.
    - Note: The offset value must be less than `cycle`.

### Process Dependencies (Key: `depends`)

- Specifies which processes must be completed before this process can run.
- All specified processes must have the same `cycle`.
- This acts as an AND condition, meaning that this process will only run if
  both the cycle condition is met and all specified dependent processes have completed.
  - If the dependent process has the same `cycle` and `cycle_offset`,
    this process starts immediately after the dependent process completes.
  - If the dependent process has a different `cycle_offset`,
    this process starts when the specified `cycle` and `cycle_offset` are reached,
    and only if the dependent process has already completed.
- For example: `client_id=10`, `cycle=2`, `cycle_offset=0`, `depends=[]` and
  - `cycle=2`, `cycle_offset=0`, `depends=[10]`
    - Run every 2nd cycle, starting immediately after process `10` completes.
    - This is because the cycle and cycle offset are the same as process `10`.
  - `cycle=2`, `cycle_offset=1`, `depends=[10]`
    - Run every 2nd cycle, with an offset of 1 cycle, but only if process `10` has already completed.
    - This process will wait until cycle_offset 1 to check if process `10` is complete before starting.
- Processes are identified by their `client_id`.
- Multiple dependencies can be specified in an array; this means all dependent processes must complete.
  - `[]`: No dependencies. Run only based on `cycle` and `cycle_offset`.
  - `[10]`: Run after process `10` completes.
  - `[10, 11]`: Run after both processes `10` and `11` complete.
- **Lifecycle and Reset**:
  - Dependency completion flags are cleared at the start of each process's execution cycle.
  - This ensures that a process always waits for the execution result of the _latest_ execution cycle of its dependencies,
    preventing premature startup due to leftover completion flags from previous execution cycles.

### Expected Execution Duration (Key: `expected_duration_ms`)

- Specifies the expected execution time for this process in milliseconds.
  - This value is used by visualization tools (like Ritsu Vis) to display the "planned" execution time on a chart.
  - It does not affect the actual execution of the process in the scheduler.
  - Default is `0` if not specified.

EOF
