# Process Configuration

The scheduler controls process execution based on periodic cycles and process dependencies.

Each process can be configured with a trigger to determine when it runs.
These triggers can be time-based cycles or the completion of other processes.

## Scheduler Configuration

### Cycles

- Specify cycles using a time-based interval (e.g., 50ms).
  - The scheduler checks which processes should be run at each cycle.

## Process Configuration

### Triggers

Choose one of the following:

- **Periodic:**
  - Specifies how often to run the process within a cycle (e.g., every cycle, every other cycle).
    - `1` means run every cycle. If the cycle is 50ms, run every 50ms.
    - `2` means run every other cycle. If the cycle is 50ms, run every 100ms.
- **Dependencies:**
  - Specifies which processes must complete before this process can run.
  - Processes are identified by a ClientID (3-digit decimal with leading zero)
  - Multiple dependencies can be specified; this means all dependent processes must complete.
    - `"001"`: Run after process "001" completes.
    - `"001,002"`: Run after both processes "001" and "002" complete.

## Example

- _T.B.D._ (To Be Determined)

EOF
