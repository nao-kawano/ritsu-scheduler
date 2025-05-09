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

  A -) M: JOIN
  M --) A: OK
  A -) M: READY

  B -) M: JOIN
  M --) B: OK
  B -) M: READY

  Note over M,B: all process is joined and ready, startup completed

  Note over M,B: ( Processing... )


  A -) M: READY
  alt Scheduler initiated shutdown
    Note over M: send ERROR for current or next request
    M --) A: *ERROR*
      Note over A: go to exit
    A -) M: EXIT
    M --) A: OK
    B -) M: READY
    M --) B: *ERROR*
      Note over B: go to exit
    B -) M: EXIT
    M --) B: OK
  else Process initiated shutdown
    Note over B: send EXIT
    B -) M: EXIT
    M --) B: OK

    Note over M: send ERROR for current or next request
    M --) A: *ERROR*
      Note over A: go to exit
    A -) M: EXIT
    M --) A: OK
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
    Note over A: waiting for trigger
  B -) M: READY
    Note over B: waiting for trigger
  C -) M: READY
    Note over C: waiting for trigger

  M ->> M: trigger (time-based)
  Note over M: check dependency -> run Process A

  M --) A: OK
  Note over A: received trigger, ok to process
  A ->> A: process
  A -) M: DONE
  M --) A: OK

  Note over M: check dependency -> run Process B and C
  M --) B: OK
  Note over B: received trigger, ok to process
  M --) C: OK
  Note over C: received trigger, ok to process

  A -) M: READY
    Note over A: waiting for trigger

  B ->> B: process
  C ->> C: process

  B -) M: DONE
  M --) B: OK
  B -) M: READY
    Note over B: waiting for trigger

  C -) M: DONE
  M --) C: OK
  C -) M: READY
    Note over C: waiting for trigger

  M ->> M: trigger (time-based)
  Note over M,C: same pattern as before
```


EOF
