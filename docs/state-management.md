# State Management

This document describes state management of DPS, which manages multiple clients and
controls the execution of each client.

Understanding both client-side and server-side state management is
essential for developing and maintaining DPS effectively.

## Client Side

This section details the state management on the client-side,
illustrating the process flow that each client should implement.

```mermaid
stateDiagram-v2
    [*] --> Connecting
    note left of Connecting : (entry) Send JOIN

    Connecting --> [*] : Recv ERROR
    Connecting --> Ready : Recv OK
    note left of Ready : (entry) Send READY

    State Active {
        Ready --> Running : Recv OK
        Ready --> Ready : Recv Skip

        Running --> Idle : Process Complete
        note right of Idle : (entry) Send DONE

        Idle --> Ready : Recv always OK
    }

    Ready --> Disconnecting : Recv ERROR
    Active --> Disconnecting : error in client
    note left of Disconnecting : (entry) Send EXIT

    Disconnecting --> [*] : Recv always OK
```

## Server Side

This section explains how the server manages the states of each client internally.

Understanding the server-side state management is critical for ensuring proper coordination and
control of client processes.

```mermaid
stateDiagram-v2
    [*] --> None
    None --> Sync : Recv JOIN
    Sync --> Ready : Recv READY

    Sync --> None : Recv EXIT
    Sync --> Exiting : going to shutdown
    Active --> Exiting : going to shutdown
    Exiting --> None : Recv EXIT
    Active --> None : Recv EXIT

    State Active {
        Ready --> Running : cycle and dependency met
        Running --> Idle : Recv DONE
        Idle --> Ready : Recv READY

        Ready --> Skip : skipped current cycle
        Ready --> SkipPrev : skipped previous cycle

        Running --> Overrun : detected overrun
        Overrun --> SkipPrev : Recv DONE

        SkipPrev --> Skip : Recv READY
        Skip --> Ready : Recv READY

        Idle --> SkipPrev : missed READY for next cycle
    }
```

Note:

- None
  - Client is Disconnected.
  - Server is waiting for `JOIN`.
- Sync
  - Client is Connecting.
  - Server is waiting for `READY`.
- Ready
  - Client is Ready.
  - Server holds the response until the target cycle starts and all dependencies are met.
- Running
  - Client is Running.
  - Server is waiting for `DONE`.
- Idle
  - Client is Idle.
  - Server is waiting for `READY`.
- Overrun
  - Client is Running.
  - Server detected an overrun and is waiting for `DONE`.
    - An overrun occurs when the previous execution has not completed by the start of the next cycle.
- Skip
  - Client is Ready.
  - Server skips the run for the current cycle, sends `SKIP` to the client, and waits for `READY` again.
  - This happens when:
    - The client is Ready, but a dependent process is not Ready for the current cycle.
    - The client is Ready from SkipPrev.
- SkipPrev
  - Client is Ready or Idle.
  - Client cannot complete process for the previous cycle.
  - This happens when:
    - The client is Ready, but a dependent process was not completed in the previous cycle.
      Sends `SKIP` to the client and waits for `READY` again.
    - The server detected that an overrun process is complete and is waiting for `READY`.
    - The server cannot receive `READY` until the next cycle starts and keeps waiting for `READY`.
- Exiting
  - Client is Disconnecting.
  - Server is waiting for `EXIT`.

EOF
