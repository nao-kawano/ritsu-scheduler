# Message Format

Message length is up to 512 bytes.
Message content is a simple String-based format:

```
Message:
  MessageType@MessageID:ClientID[,Extras]

MessageType:
  -> string of message type (see next section for details)
MessageID:
  -> string of 1-digit decimal (e.g. "0" ~ "9")
ClientID:
  -> string of 3-digit decimal with leading zero (e.g. "000" ~ "999")
Extras:
  Extra[,Extras]
Extra:
  Key=Value
Key:
  - "version": Protocol version (e.g. "1"). Used in "JOIN" and "JOINED".
  - "cycle": Current cycle count.
    It is a 64-bit signed integer (`i64`) formatted as a string (up to 12 digits, e.g. "123" or "999999999999").
    Client applications parsing this value must use a 64-bit integer type (like `i64` in Rust, `long` in C++, 
    or standard `int` in Python) to prevent overflow.
    Included in "START", "SKIP", "LATE" for "READY" request.
  - "reason": Error reason (e.g. "IncompatibleVersion"). Included in "ERROR".
  - "cid": Client ID (e.g. "001"). Used with "reason=ClientExit" to specify the source of the exit.
```

## MessageType

MessageType is divided into requests from the client and responses from the server.

### Client Requests

- "JOIN"
  - The client requests to join the scheduling group.
  - **Extras**: Must include `version=N`.
- "READY"
  - The client notifies the server that it's ready to start the process.
  - The client must wait for a response from the server.
    - The server holds the response until the trigger for this client is ready.
- "DONE"
  - The client notifies the server that the process is completed.
    - The server triggers the next dependent process.
  - After sending "DONE", the client needs to send "READY" to request the next trigger.
- "EXIT"
  - The client notifies the server that it's going to exit.

### Server Responses

- "JOINED"
  - Returned for "JOIN".
  - Indicates that the client successfully joined the scheduling group.
  - **Extras**: Includes `version=N`.
- "START"
  - Returned for "READY".
  - Indicates that the trigger is ready and the process should start.
  - **Extras**: Includes `cycle=N`.
- "OK"
  - Returned for "DONE", "EXIT".
  - Indicates that the request was successful.
- "SKIP"
  - Returned for "READY".
  - Indicates that the trigger has been canceled.
  - The client needs to send "READY" again to request the next trigger.
  - **Extras**: Includes `cycle=N`.
- "LATE"
  - Returned for "READY".
  - Indicates that the process has overrun and missed the next trigger.
  - The client needs to send "READY" again to request the next trigger.
  - **Extras**: Includes `cycle=N`.
- "ERROR"
  - Returned for "JOIN", "READY", "DONE".
  - Indicates that the request is invalid or the server is in an invalid state.
  - Except for "JOIN" errors, the client must send "EXIT" to the server and exit.
  - **Extras**: Includes `reason=TEXT`.

## MessageID

- The client assigns an arbitrary ID to each request.
  - The same ID is used for retransmissions.
  - Different IDs are assigned to different requests (e.g., incrementing).
- The server includes the ID of the corresponding request in the response.
- The client checks the ID in the response:
  - Discards responses that do not match the expected response.
  - This allows discarding duplicate responses from previous requests when the sequence has advanced.

- NOTE:
  - Messages are exchanged sequentially, one at a time.
  - A single digit is used because it is sufficient to distinguish each message from the previous one.

EOF
