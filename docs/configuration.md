# Process Configuration

The scheduler controls process execution based on periodic cycles and process dependencies.

Each process can be configured with a trigger to determine when it runs.
These triggers can be time-based cycles or the completion of other processes.

## Server (Scheduler) Configuration

### Port

- Specify the port number for the scheduler.

### Cycle Time

- Specify cycles using a time-based interval (e.g., 50ms).
  - The scheduler checks which processes should be run at each cycle.

- **NOTE:** This is currently implemented using an interval-based trigger.
  - The underlying architecture is designed to support various cycle triggers for future extensibility.

## Client (Process) Configuration

Per client configuration below:

### Client ID

- Specifies a unique ID for this process.
  - Each client has a unique ID represented as an integer (`u16`, from `0` to `999`).
  - This ID is used to identify the client in the scheduler.

### Display Name

- Specifies a human-readable name for this process.
  - This value is used by visualization tools (like Ritsu Vis) to make it easier to identify processes.
  - It does not affect the actual execution of the process in the scheduler.
  - **Maximum length is 20 characters** to ensure proper display in the UI.
  - If not specified, it defaults to an empty string.

### Cycle

- Specifies how often to run the process within a cycle (e.g., every cycle, every other cycle).
  - `1` means run every cycle. If the cycle is 50ms, run every 50ms.
  - `2` means run every other cycle. If the cycle is 50ms, run every 100ms.

### Cycle Offset

- Specifies the offset for when this client starts running.
  - Useful when you want to alternate the execution of clients for distribute the load evenly.
    - `0`: Start running at the beginning of the specified cycle.
    - `1`: Start running one cycle later than the beginning of the specified cycle.
    - Note: The offset value must be less than the trigger cycle.

### Depends

- Specifies which processes must be completed before this process can run.
- All specified processes must have the same Cycle.
- This acts as an AND condition, meaning that this process will only run if
  both the cycle condition is met and all specified dependent processes have completed.
  - If the dependent process has the same cycle and cycle offset,
    this process starts immediately after the dependent process completes.
  - If the dependent process has a different cycle offset,
    this process starts when the specified cycle and cycle offset are reached,
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

### Expected Duration MS

- Specifies the expected execution time for this process in milliseconds.
  - This value is used by visualization tools (like Ritsu Vis) to display the "planned" execution time on a chart.
  - It does not affect the actual execution of the process in the scheduler.
  - Default is `0` if not specified.

EOF
