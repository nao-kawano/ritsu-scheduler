# Message Sequence

This software employs a Client-Server architecture utilizing simple UDP messaging.

The client passively waits for a start trigger from the server.
The server then sends a start trigger based on the periodic cycles and dependencies between the clients.

## Startup and Shutdown

The client starts by sending a "JOIN" and "READY" message to the server.

```mermaid
sequenceDiagram
  participant M as Scheduler
  participant A as ProcessA
  participant B as ProcessB

  Note over A: None
  Note over B: None

  A -) M: JOIN
  Note over A: Connecting
  M --) A: OK

  A -) M: READY
  Note over A: Ready

  B -) M: JOIN
  Note over B: Connecting
  M --) B: OK

  B -) M: READY
  Note over B: Ready

  Note over M,B: all process is joined and ready, startup completed

  Note over M,B: ( Processing... )

  A -) M: READY
  alt Scheduler initiated shutdown
    Note over M: response ERROR in shutdown
    M --) A: *ERROR*
      Note over A: Disconnecting
    A -) M: EXIT
    M --) A: OK
      Note over A: None
    
    B -) M: READY
    M --) B: *ERROR*
      Note over B: Disconnecting
    B -) M: EXIT
    M --) B: OK
      Note over B: None

  else Process initiated shutdown
    Note over B: send EXIT
    B -) M: EXIT
      Note over B: Disconnecting
      Note over M: go to shutdown state
    M --) B: OK
      Note over B: None

    Note over M: response ERROR in shutdown
    M --) A: *ERROR*
      Note over A: Disconnecting
    A -) M: EXIT
    M --) A: OK
      Note over A: None
  end

  Note over M,B: all process is exitted, shutdown complete
```

## Basic Scheduling

- Process A starts periodically
- Process B and C start when A completes

```mermaid
sequenceDiagram
  participant M as Scheduler
  participant A as ProcessA
  participant B as ProcessB
  participant C as ProcessC

  A -) M: READY
    Note over A: Ready
  B -) M: READY
    Note over B: Ready
  C -) M: READY
    Note over C: Ready

  M ->> M: trigger (time-based)
  Note over M: check dependency -> run Process A

  M --) A: OK
  Note over A: Running
  A ->> A: process
  A -) M: DONE
  Note over A: Idle
  M --) A: OK

  Note over M: check dependency -> run Process B and C
  M --) B: OK
  Note over B: Running
  M --) C: OK
  Note over C: Running

  A -) M: READY
    Note over A: Ready

  B ->> B: process
  C ->> C: process

  B -) M: DONE
  Note over B: Idle
  M --) B: OK
  B -) M: READY
    Note over B: Ready

  C -) M: DONE
  Note over C: Idle
  M --) C: OK
  C -) M: READY
    Note over C: Ready

  M ->> M: trigger (time-based)
  Note over M,C: same pattern as before
```

EOF
