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

### Trigger Type

Choose one of the following:

- **Cycle:**
  - Specifies how often to run the process within a cycle (e.g., every cycle, every other cycle).
    - `1` means run every cycle. If the cycle is 50ms, run every 50ms.
    - `2` means run every other cycle. If the cycle is 50ms, run every 100ms.
- **Depends:**
  - Specifies which processes must complete before this process can run.
  - Processes are identified by a ClientID.
  - Multiple dependencies can be specified; this means all dependent processes must complete.
    - `"001"`: Run after process "001" completes.
    - `"001,002"`: Run after both processes "001" and "002" complete.

### Cycle Offset

- Specifies the offset for when this client starts running.
  - Useful when you want to alternate the execution of clients for distribute the load evenly.
    - `0`: Start running at the beginning of the specified cycle.
    - `1`: Start running one cycle later than the beginning of the specified cycle.
    - Note: The offset value is used when trigger type is cycle, and must be less than the trigger cycle.

EOF
