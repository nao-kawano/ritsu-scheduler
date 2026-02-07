# Design Study for UDP Packet Loss scenarios

This document outlines the expected behavior during UDP packet loss
based on the state management and message format definitions.

## Retransmission Control

### Retransmission

- The client retransmits if there is no response from the server.
- The retransmission interval and number of retries are defined as guidelines according to the message type.
- **NOTE**:
  - For cyclic operations, the client employs a request-response model.
  - It sends a request and waits for a response. If a response is not received within a timeout period, it retransmits the request.

#### Guidelines

- `JOIN`
  - Retransmission interval: approximately 20ms
  - Number of retries: Until connected (or timeout after a few seconds)
  - On expiration: Error/Terminate
- `READY` (before cyclic operation)
  - Retransmission interval: Set to a value longer than one execution cycle (e.g., 200ms if cycle=2 with a cycle_time 100ms).
  - Number of retries: Until cyclic operation starts
  - On expiration: sends `EXIT`
- `READY` (in cyclic operation)
  - Retransmission timeout: Set to a value longer than one execution cycle (e.g., 200ms if cycle=2 with a cycle_time 100ms).
  - Number of retries: 3
  - On expiration: sends `EXIT`
- `DONE`
  - Retransmission interval: approximately 20ms
  - Number of retries: 3
  - On expiration: sends `EXIT`
- `EXIT`
  - Retransmission interval: approximately 20ms
  - Number of retries: 3
  - On expiration: terminate

### Duplicate Discard

- The client assigns a unique `MessageID` to each request (see `message-format.md`).
  - **Important**: The same `MessageID` MUST be used for retransmissions of the same request.
  - A new `MessageID` is assigned when sending a new request (e.g., for the next cycle).
- The server includes the `MessageID` of the corresponding request in the response.
- The client checks the ID in the response:
  - Discards responses that do not match the current waiting `MessageID`.

## Scenario Verification

Refer to `state-management.md` for state definitions.

## Upstream Case (Client to Server)

The server does not receive the client's request.

- `JOIN`
  - Server state: `None`
  - Action: Server ignores the lost packet.
  - Recovery: Client retransmits `JOIN`. Server receives it, sends `OK`, and transitions to `Sync`.
  - Duplicate: If Server receives `JOIN` while in `Sync`, it resends `OK`.
- `READY` (before cyclic operation)
  - Server state: `Sync`
  - Action: Server keeps waiting.
  - Recovery: Client retransmits `READY`. Server receives it, transitions to `Ready` (internal wait).
  - Duplicate: If Server receives `READY` while in `READY`, holds response to client.
- `READY` (in cyclic operation)
  - Server state: `Idle`
  - Action: Server does not receive `READY`. At the start of the next cycle, the server transitions the client state from `Idle` to `Late`.
  - Recovery:
    - The client's request times out (after >1 cycle time). The client retransmits `READY` with the same `MessageID`.
    - The server, now in the `Late` state for that client, receives the retransmitted `READY`.
    - The server responds with `LATE`, including the `MessageID` of the request.
    - The client receives `LATE`, sends a new `READY` for the next cycle, and waits again.
  - Duplicate: If Server receives `READY` while in `READY`, holds response to client.
- `DONE`
  - Server state: `Running`
  - Action: Server keeps waiting. If the next cycle starts before `DONE` is received, the server transitions the client state to `Overrun`.
  - Recovery: Client retransmits `DONE`. Server receives it, sends `OK`, and transitions to `Idle` (or `Late` if `Overrun` occurred).
  - Duplicate: If Server receives `DONE` while in `Idle` (already processed), it resends `OK`.
- `EXIT`
  - Server state: Any
  - Action: Server keeps current state.
  - Recovery: Client retransmits `EXIT`. Server sends `OK` and transitions to `None`.
  - Duplicate: If Server receives `EXIT` while in `None`, it resends `OK` to confirm termination.

## Downstream Case (Server to Client)

The server receives the client's request, but the client does not receive the response.

- `JOIN`
  - Server state: `Sync` (Transitioned from `None`)
  - Action: Client does not receive the response.
  - Recovery: Client retransmits `JOIN`. Server receives duplicate `JOIN`.
  - Duplicate: Server keeps current state and resends `OK`.
- `READY` (before cyclic operation)
  - Server state: `Ready` (Transitioned from `Sync`)
  - Action: Client does not receive the response.
  - Recovery: Client retransmits `READY`. Server receives duplicate `READY`.
  - Duplicate: Server keeps current state and resends `OK` (or holds response if trigger is not ready).
- `READY` (in cyclic operation)
  - Server state: `Running`, `Skip` or `Idle` (Transitioned from `Ready`, `Ready` or `Late`)
  - Action: Client does not receive the response (`OK`, `SKIP` or `LATE`).
  - Recovery: Client retransmits `READY` after timeout. Server receives duplicate `READY`.
  - Duplicate:
    - If Server state is `Running`, keeps current state and resends `OK`.
    - If Server state is `Skip` or `Idle`, transitions to `Ready` and waits for the next cycle.
      - (Note: This simplifies server logic. The client won't be notified about the missed `SKIP`/`LATE`.)
- `DONE`
  - Server state: `Idle` (Transitioned from `Running`)
  - Action: Client does not receive the response.
  - Recovery: Client retransmits `DONE`. Server receives duplicate `DONE`.
  - Duplicate: Server keeps current state and resends `OK`.
- `EXIT`
  - Server state: `None`
  - Action: Client does not receive the response.
  - Recovery: Client retransmits `EXIT`. Server receives duplicate `EXIT`.
  - Duplicate: Server keeps current state and resends `OK`.

## Summary of Server Responsibilities

1. **Idempotency**:

- The server handles duplicate requests based on its current state.
- It ensures the response includes the `MessageID` from the request so the client can verify it.

2. **State Consistency**:

- The server must ensure that retransmissions do not cause invalid state transitions (e.g., starting a new cycle prematurely).

EOF
