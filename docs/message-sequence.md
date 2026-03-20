# Message Sequence

This software employs a Client-Server architecture utilizing simple UDP messaging.

The client passively waits for a start trigger from the server.
The server then sends a start trigger based on the periodic cycles and dependencies between the clients.

## Startup and Shutdown

The client starts by sending a "JOIN" and "READY" message to the server.

```mermaid
sequenceDiagram
  participant S as Scheduler
  participant A as ProcessA
  participant B as ProcessB

  Note over A: None
  Note over B: None

  A -) S: JOIN,version=1
  Note over A: Connecting
  S --) A: JOINED,version=1

  A -) S: READY
  Note over A: Ready

  B -) S: JOIN,version=1
  Note over B: Connecting
  S --) B: JOINED,version=1

  B -) S: READY
  Note over B: Ready

  Note over S,B: all process is joined and ready, startup completed

  Note over S,B: ( Processing... )

  A -) S: READY
  alt Scheduler initiated shutdown
    Note over S: response ERROR in shutdown
    S --) A: *ERROR,reason=Shutdown*
      Note over A: Disconnecting
    A -) S: EXIT
    S --) A: OK
      Note over A: None

    B -) S: READY
    S --) B: *ERROR,reason=Shutdown*
      Note over B: Disconnecting
    B -) S: EXIT
    S --) B: OK
      Note over B: None

  else Process initiated shutdown
    Note over B: send EXIT
    B -) S: EXIT
      Note over B: Disconnecting
      Note over S: go to shutdown state
    S --) B: OK
      Note over B: None

    Note over S: response ERROR in shutdown
    S --) A: *ERROR,reason=ClientExit,cid=002*
      Note over A: Disconnecting
    A -) S: EXIT
    S --) A: OK
      Note over A: None
  end

  Note over S,B: all process is exited, shutdown complete
```

## Basic Scheduling

- Process A starts periodically
- Process B and C start when A completes

```mermaid
sequenceDiagram
  participant S as Scheduler
  participant A as ProcessA
  participant B as ProcessB
  participant C as ProcessC

  A -) S: READY
    Note over A: Ready
  B -) S: READY
    Note over B: Ready
  C -) S: READY
    Note over C: Ready

  S ->> S: trigger (time-based)
  Note over S: check dependency -> run Process A

  S --) A: START,cycle=1
  Note over A: Running
  A ->> A: process
  A -) S: DONE
  Note over A: Idle
  S --) A: OK

  Note over S: check dependency -> run Process B and C
  S --) B: START,cycle=1
  Note over B: Running
  S --) C: START,cycle=1
  Note over C: Running

  A -) S: READY
    Note over A: Ready

  B ->> B: process
  C ->> C: process

  B -) S: DONE
  Note over B: Idle
  S --) B: OK
  B -) S: READY
    Note over B: Ready

  C -) S: DONE
  Note over C: Idle
  S --) C: OK
  C -) S: READY
    Note over C: Ready

  S ->> S: trigger (time-based)
  Note over S,C: same pattern as before
```

EOF
