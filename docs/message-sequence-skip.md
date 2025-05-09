# Message Sequence of SKIP scenario

When a trigger occurs and the client is still processing, the server sends a SKIP response to the READY.
This tells the client to skip its current execution and retry to adjust next timing.

```mermaid
sequenceDiagram
  participant M as Scheduler
  participant A as ProcessA

  A -) M: READY
    Note over A: waiting for trigger

  M ->> M: trigger (time-based)
  Note over M: check dependency -> run Process A

  M --) A: OK
  Note over A: received trigger, ok to process
  A ->> A: process
  activate A
  Note over A: processing...

  M ->> M: trigger (time-based)
  Note over M: check dependency -> run Process A <br />but Process A is still running.<br />mark as SKIP this cycle.

  Note over A: processing...
  A -) M: DONE
  Note over M: skip check dependency
  M --) A: OK


  A -) M: READY
  Note over M: skip this cycle
  M --) A: *SKIP*
    Note over A: detected SKIP, send READY again

  A -) M: READY

```

## Examples

- Single client A without dependency

```mermaid
gantt
    dateFormat H-m
    axisFormat %H
    tickInterval 1hour

    section OK
    A-cycle0: a0, 0-0, 0.5h
    A-cycle1: a1, 1-0, 0.5h
    A-cycle2: a2, 2-0, 0.5h

    section SKIP
    A-cycle0: crit, a'0, 0-0, 1.2h
    A-cycle1=SKIP: done, a'1, 1-12, 0.1h
    A-cycle2: a'2, 2-0, 1.2h
```

- Client B depends on client A

```mermaid

gantt
    dateFormat H-m
    axisFormat %H
    tickInterval 1hour

    section OK
    A-cycle0: a0, 0-0, 0.5h
    A-cycle1: a1, 1-0, 0.5h
    A-cycle2: a2, 2-0, 0.5h
    B-cycle0: b0, after a0, 0.8h
    B-cycle1: b1, after a1, 0.8h
    B-cycle2: b2, after a2, 0.8h

    section SKIP
    A-cycle0: aa0, 0-0, 0.8h
    A-cycle1: crit, aa1, 1-0, 0.5h
    A-cycle2: aa2, 2-0, 0.5h
    B-cycle0: crit, bb0, after aa0, 0.8h
    B-cycle1=SKIP: done, bb1, after bb0, 0.1h
    B-cycle2: bb2, after aa2, 0.8h
```

EOF
