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
  - Represents the current `running_cycle`.

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
`<STAT> CYC:{RunningCycle} CID:{ClientID} MID:{MessageID} {Event/Transition} ({Details})`

For state changes, use the `{Before} -> {After}` format. Use prefixes to distinguish between connection layers and internal process layers.

- **Connection Layer:** `CID:010 MID:1 [Conn] None -> Sync`
- **Process Layer:** `CID:010 MID:2 Ready -> Running (Cycle)`
- **Retransmission:** `CID:010 MID:2 Ready -> Ready (Retransmit)`
- **Discrete Event:** `CID:010 MID:1 JOIN`

### System-level Events
For global events like cycle starts or server-initiated aborts, `MID` and `CID` are omitted or set to N/A.

`<STAT> CYC:{RunningCycle} {Event} ({Details})`

- **Cycle Start:** `<STAT> CYC:000000000008 START`
- **Manager Transition:** `<STAT> CYC:000000000008 [Manager] Starting -> Running`
- **Server Abort:** `<STAT> CYC:000000000008 ABORT (Reason: abort requested)`

## Asynchronous File Logging & Performance

The Ritsu scheduler executes high-frequency periodic control loops (e.g., 50ms cycle time). To prevent synchronous disk I/O blocking from causing scheduling jitter, deadline misses (`LATE`), or execution overruns (`OVERRUN`), the server implements asynchronous file logging via `flexi_logger`.

- **Asynchronous File I/O:** Log records destined for the file are dispatched through an in-memory queue and written to disk by a dedicated background worker thread (`WriteMode::Async`).
- **Console Synchronization Note:** While file I/O is fully asynchronous, console output (`duplicate_to_stderr`) is executed synchronously on the calling thread. In production environments with high-frequency cycles, it is strongly recommended to set the console log level to `info` (or higher) to keep the main event loop lightweight, while capturing detailed events (`debug` or `trace`) in the log file.
- **Graceful Shutdown:** On normal shutdown, pending asynchronous log buffers are flushed. Note that forcible termination (e.g., `SIGKILL` or `Stop-Process -Force`) may cause unwritten buffer records to be lost.

## Log Level Clamping Rule

Due to the architectural design of `flexi_logger`, the console duplication stream cannot be configured to a more detailed verbosity than the primary file logger stream.

- When file logging is active and the requested console log level is more detailed than the file log level (e.g., console `debug`, file `info`), the server automatically **clamps** the console log level to match the file log level (`info`).
- The startup log banner explicitly displays when clamping has occurred:
  ```
  - Log level:      info (clamped from debug)
  - Log file:       logs/rt-server.log
  - Log file level: info
  ```
- When file logging is disabled (via `--no-log-file` or omitting `log_file_dir`), the console operates independently at its requested log level without clamping.

## Startup 5-Generation Log Rotation

To prevent unintended log file growth and accidental log erasure upon restart while keeping file access simple and predictable for external visualization tools (such as `rt-vis`):

- **Predictable Active File:** The active log is always written to `{file_basename}.log` (default: `rt-server.log`).
- **Startup Shift Rotation:** Whenever the server starts with file logging enabled, existing log files in the target directory are automatically shifted:
  1. The oldest backup (`rt-server.5.log`) is removed if it exists.
  2. Existing backup files are shifted down in descending order: `rt-server.4.log` -> `rt-server.5.log`, ..., `rt-server.1.log` -> `rt-server.2.log`.
  3. The previous active log `rt-server.log` is renamed to `rt-server.1.log`.
  4. A fresh `rt-server.log` is opened for the new session.
- **Generation Limit:** Up to 5 backup generations (`.1.log` through `.5.log`) are retained (`MAX_LOG_BACKUPS = 5`).
- **Fault Tolerance:** If file deletion or renaming fails (e.g., due to file locking by another application), a warning is printed to stderr and server startup proceeds without crashing.

EOF
