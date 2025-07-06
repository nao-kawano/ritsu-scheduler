# Process Configuration

The scheduler controls process execution based on periodic cycles and process dependencies.

Each process can be configured with a trigger to determine when it runs.
These triggers can be time-based cycles or the completion of other processes.

## Server Configuration

### Port

- Specify the port number for the scheduler.

### Cycle Time

- Specify cycles using a time-based interval (e.g., 50ms).
  - The scheduler checks which processes should be run at each cycle.

## Client Configuration

Per client configuration below:

### Client ID

- Specifies a unique ID for this process.
  - Each client has a unique ID (string of 3-digit decimal with leading zero).
  - This ID is used to identify the client in the scheduler.

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
- For example: `ID=0`, `Cycle=2`, `CycleOffset=0`, `Depends=""` and
  - `Cycle=2`, `CycleOffset=0`, `Depends="000"`
    - Run every cycle, starting immediately after process "000" completes.
    - This is because the cycle and cycle offset are the same as process "000".
  - `Cycle=2`, `CycleOffset=1`, `Depends="000"`
    - Run every other cycle, with an offset of 1 cycle, but only if process "000" has already completed.
    - This process will wait until Cycle 2, CycleOffset 1, to check if process "000" is complete before starting.
- Processes are identified by a ClientID.
- Multiple dependencies can be specified; this means all dependent processes must complete.
  - `""`: No dependencies. Run only based on `Cycle` and `Cycle Offset`.
  - `"001"`: Run after process "001" completes.
  - `"001,002"`: Run after both processes "001" and "002" complete.

EOF
