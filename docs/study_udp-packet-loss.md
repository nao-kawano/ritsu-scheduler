# Design Study for UDP Packet Loss scenarios

This document outlines the expected behavior during UDP packet loss
when the sender implements the following retransmission control mechanisms.

## Retransmission Control

### Retransmission

- The client retransmits if there is no response from the server.
- The retransmission interval and number of retries are defined as guidlines according to the message type:
  - `JOIN`
    - Retransmission interval: approximately 20ms
    - Number of retries: 3
    - On expiration: terminate
  - `READY` (before cyclic operation)
    - Retransmission interval: set based on the startup time until all other clients are started
    - Number of retries: set based on the startup time until all other clients are started
    - On expiration: send `EXIT`
  - `READY` (in cyclic operation)
    - Retransmission interval: execution cycle (e.g., 200ms if cycle=2 with a cycle_time 100ms)
    - Number of retries: 3
    - On expiration: send `EXIT`
  - `DONE`
    - Retransmission interval: approximately 50ms (depends on process count)
    - Number of retries: 3
    - On expiration: terminate
  - `EXIT`
    - Retransmission interval: approximately 20ms
    - Number of retries: 3
    - On expiration: terminate

### Duplicate Discard

- The client assigns an arbitrary ID to each request.
  - The same ID is used for retransmissions.
  - Different IDs are assigned to different requests (e.g., incrementing).
- The server includes the ID of the corresponding request in the response.
- The client checks the ID in the response:
  - Discards responses that do not match the expected response.
  - This allows discarding duplicate responses from previous requests when the sequence has advanced.

## Scenario Verification

## Upstream Case (Client to Server)

The server does not receive the client's request.

- `JOIN`
  - The server does not start cyclic operation while waiting for `JOIN`.
  - The server returns cyclic operation if it receives a retransmitted `JOIN`.
    - If the server receives a duplicate `JOIN`, it sends `OK`.
- `READY` (before cyclic operation)
  - The server does not receive `READY`, and the client state remains `SYNC`.
  - The server returns cyclic operation if it receives a retransmitted `READY` after the
    execution cycle time has elapsed.
    - If the server receives a duplicate `READY`, it discards it.
      - NOTE: Sending `OK` would start the client process, so discard during startup.
- `READY` (in cyclic operation)
  - The server does not receive `READY`, and the client state remains `IDLE`.
  - In the next execution cycle, the server sets the client state to `SKIP_PREV` and waits for `READY`.
  - The server sends `SKIP` if it receives a retransmitted `READY` after the execution cycle time has elapsed.
    - NOTE: If retransmission is not received within this cycle, error handling is initiated.
  - The server receives `READY` again and returns cyclic operation.
- `DONE`
  - The server does not receive `DONE`, and the client state remains `IDLE`.
  - The server returns cyclic operation if it receives a retransmitted `DONE`.
    - If the server receives a duplicate `DONE`, it sends `OK`.
- `EXIT`
  - The server does not receive `EXIT`, and the client state remains `EXITTING`.
  - The server returns cyclic operation if it receives a retransmitted `EXIT`.
    - If the server receives a duplicate `EXIT`, it sends `OK`.

## Downstream Case (Server to Client)

The server receives the client's request, but the client does not receive the response.

- `JOIN`
  - The server sends `OK` and sets the client state to `SYNC`.
  - The client does not receive `OK` and retransmits `JOIN`.
  - => The server keeps the client state as `SYNC` and sends `OK`.
- `READY` (before cyclic operation)
  - N/A
- `READY` (in cyclic operation)
  - The server sends `OK` for start, and sets the client state to `RUNNING`.
  - The client does not receive `OK` and retransmits `READY` after the cycle time has elapsed.
  - => The server keeps the client state as `RUNNING` and sends `OK`.
- `DONE`
  - The server sends `OK` and sets the client state to `IDLE`.
  - The client does not receive `OK` and retransmits `DONE`.
  - => The server keeps the client state as `IDLE` and sends `OK`.
- `EXIT`
  - The server sends `OK` and sets the client state to `NONE`.
  - The client does not receive `OK` and retransmits `EXIT`.
  - => The server keeps the client state as `NONE` and sends `OK`.

NOTE: The server must send `OK` to prevent continuous retransmission from the client.

EOF
