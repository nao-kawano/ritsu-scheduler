# DPS

DPS: Deterministic Process Scheduler - schedules user processes considering periodic cycles and process dependencies.

## Features

- Periodic process execution with pre-defined dependencies (single, sequential, parallel)
  - example: Image processing using camera frame
- Simple messaging for various implementation languages
  - this repository includes sample client for Rust, Python
- Simulation and Visualization tools (currently under planning)

## Process scheduling

This section describes how DPS schedules your processes using a sequence diagram.

This is a simplified diagram, see also detailed documents for a complete understanding.

### example scenario: A -> B, C

- Process A starts periodically
- Process B and C start when A completes

```mermaid
sequenceDiagram
  participant S as Scheduler
  participant A as ProcessA
  participant B as ProcessB
  participant C as ProcessC

  A -) S: READY
    Note over A: waiting for trigger
  B -) S: READY
    Note over B: waiting for trigger
  C -) S: READY
    Note over C: waiting for trigger

  S ->> S: trigger (time-based)
  Note over S: check dependency -> run Process A

  S --) A: OK
  Note over A: received trigger, ok to process
  A ->> A: process
  A -) S: DONE

  Note over S: check dependency -> run Process B and C
  S --) B: OK
  Note over B: received trigger, ok to process
  S --) C: OK
  Note over C: received trigger, ok to process

  A -) S: READY
    Note over A: waiting for trigger

  B ->> B: process
  C ->> C: process

  B -) S: DONE
  B -) S: READY
    Note over B: waiting for trigger

  C -) S: DONE
  C -) S: READY
    Note over C: waiting for trigger

  S ->> S: trigger (time-based)
  Note over S,C: same pattern as before
```

## Components of this repository

- this repository
  - messages-rs
    - Common message types and structures for Rust server/scheduler and client/process.
  - server-rs
    - Server/Scheduler program implemented in Rust.
  - clientlib-rs
    - Client library for Rust client/process. Handles communication with the server/scheduler.

EOF
