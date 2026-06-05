# Logging Guidelines

This document defines the logging standards for the Ritsu scheduler server to ensure consistent, searchable, and parsable output for both operational monitoring and behavioral visualization.

## Log Format

The server outputs logs in the following structured format:

`[Timestamp] [Level] ModuleName - <ActionTag> Message`

- **ModuleName:** Automatically derived from the Rust module path (e.g., `process_running`).
- **Separator (` - `):** Distinguishes metadata from the log body.
- **ActionTag (`<...>`):** Categorizes the type of processing using angle brackets to differentiate from the log level.

## Log Levels

| Level | Description | Usage Examples |
| :--- | :--- | :--- |
| **ERROR** | Critical failures that prevent the server from continuing. | Port binding failure, configuration corruption. |
| **WARN** | Abnormal conditions that require attention but allow the server to continue. | `OVERRUN`, `LATE`, `SKIP`, `Retransmit`. |
| **INFO** | Significant system lifecycle events. | Server Start/Stop, Client Join/Exit, Major State transitions. |
| **DEBUG** | Standard operational events and scheduling decisions. | Cycle Start, Client Start/Done, `<STAT>` transitions. |
| **TRACE** | Detailed internal processing and raw message content. | Raw packet hex, internal function entries. |

## Identifiers

To ensure logs are easily searchable, use the following fixed-width formats for identifiers:

- **ClientID (CID):** `CID:{:03}` (e.g., `CID:010`)
- **MessageID (MID):** `MID:{}` (e.g., `MID:3`)
- **Cycle Number (CYC):** `CYC:{:012}` (e.g., `CYC:000000000042`)

## Action Tags

- `<RECV>` : Reception of a message from a client.
- `<SEND>` : Transmission of a response to a client.
- `<STAT>` : Transition of an entity's state (Main source for visualization).
- `<CONFIG>` : Raw configuration data (TOML) for reproducibility in analysis tools.

## Component Responsibilities

### Manager / State Layer
- Must output `<STAT>` logs including `CYC` and `MID` when applicable.
- Must log abnormal scheduling events (e.g., `Overrun`) as `WARN`.

Example:
`[DEBUG] process_running - <STAT> CYC:000000000008 CID:010 MID:3 Running -> Idle`

### Client Connector Layer
- Must output `<RECV>` and `<SEND>` logs using the common ClientConnector logic.
- Abstracts the underlying transport (e.g., UDP).

Example:
`[TRACE] clients - <RECV> CID:010 MID:2 (Ready)`

### Core Layer (Scheduler / ProcessEntry)
- Should focus on local state changes and dependency updates.
- Does not need to know about `CYC` or `MID`.

Example:
`[TRACE] entry - CID:010 Ready -> Running`

## Visualization Support

The `<STAT>` tag is specifically designed for timeline visualization tools. It covers both client-specific transitions and system-level events.

### Client-specific Transitions
`<STAT> CYC:{Cycle} CID:{ClientID} MID:{MessageID} {Event/Transition} ({Details})`

For state changes, use the `{Before} -> {After}` format. Use prefixes to distinguish between connection layers and internal process layers.

- **Connection Layer:** `CID:010 MID:1 [Conn] None -> Sync`
- **Process Layer:** `CID:010 MID:2 Ready -> Running (Cycle)`
- **Retransmission:** `CID:010 MID:2 Ready -> Ready (Retransmit)`
- **Discrete Event:** `CID:010 MID:1 JOIN`

### System-level Events
For global events like cycle starts or server-initiated aborts, `MID` and `CID` are omitted or set to N/A.

`<STAT> CYC:{Cycle} {Event} ({Details})`

- **Cycle Start:** `<STAT> CYC:000000000008 START`
- **Manager Transition:** `<STAT> CYC:000000000008 [Manager] Starting -> Running`
- **Server Abort:** `<STAT> CYC:000000000008 ABORT (Reason: abort requested)`
