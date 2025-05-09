# Message Format

Massage length is up to 512 bytes.
Message content is a simple String-based format:

```
Message:
  MessageType:ClientID[,Extras]

MessageType:
  -> string of message type (see next section for details)
ClientID:
  -> string of 3-digit decimal with leading zero (e.g. "000" ~ "999")
Extras: *for future use*
  Extra[,Extras]
Extra:
  -> string of key-value (e.g. "key1=value1")
```

## MessageType

MessageType is divided into requests from the client and responses from the server.

### Client Requests

- "JOIN"
  - The client requests to join the scheduling group.
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

- "OK"
  - Returned for "JOIN", "READY", "DONE", "EXIT".
  - Indicates that the request was successful.
  - The client can continue processing.
- "SKIP"
  - Returned for "READY".
  - Indicates that the trigger has been canceled.
  - The client needs to send "READY" again to request the next trigger.
- "ERROR"
  - Returned for "JOIN", "READY", "DONE".
  - Indicates that the request is invalid or the server is in an invalid state.
  - Except for "JOIN" errors, The client must send "EXIT" to the server and exit.

EOF
