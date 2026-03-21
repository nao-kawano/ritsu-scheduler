# Documentation

### Structural Diagram

The following diagram shows the structural relationships and ownership between the components.

```mermaid
graph TD
    %% Server Project
    subgraph "rt-server-rs (Scheduler)"
        Main[Main]
        SC[SchedulerConfig]

        EM[EventManager]

        CG[CycleGenerator]
        CT["&laquo;interface&raquo;<br/>CycleTrigger"]
        IT[IntervalTrigger]

        CC[ClientConnector]
        CTr["&laquo;interface&raquo;<br/>ClientTransport"]
        UT[UdpTransport]

        Main -.-> SC
        Main --> EM
        Main --> CC
        Main --> CG

        CG --> CT
        CT -.-> IT

        CC --> CTr
        CTr -.-> UT
    end

    %% Core Project
    subgraph "rt-core-rs (Core)"
        SCH[Scheduler]
    end

    %% Client Projects
    subgraph "rt-client-rs (Rust Client Lib)"
        RCL[Rust Client API]
    end
    subgraph "rt-client-py (Python Client)"
        PCL[Python Client API]
    end

    %% Message Project
    subgraph "rt-message-rs (Common)"
        Msg[Message Struct / Types]
    end

    %% Inter-project dependencies
    EM --> SCH
    EM -.-> Msg
    CTr -.-> Msg
    RCL -.-> Msg
    %% for layout
    PCL ~~~ Msg
    SCH ~~~ Msg

    %% Communication Flow
    UT <-.->|UDP| RCL
    UT <-.->|UDP| PCL

    %% Styling
    style CT fill:#f9f9f9,stroke:#666,stroke-dasharray: 5 5
    style CTr fill:#f9f9f9,stroke:#666,stroke-dasharray: 5 5
```

## Basic Design and Specs

- [Process Configuration](./configuration.md)
  - Configuration for process dependencies and timing.
- [Message Sequence](./message-sequence.md)
  - Basic message flow between clients and server.
- [Message Sequence of SKIP scenario](./message-sequence-skip.md)
  - Advanced message flow for Deadline Exceeded.
- [Message Format](./message-format.md)
  - Messages format exchanged between clients and server.
- [State Management](./state-management.md)
  - State management in the client and server.

## Design Studies & Internal Resources

- [Logging Guidelines](./logging.md)
  - Standards for logs and visualization support.
- [Design Study for UDP Packet Loss](./study_udp-packet-loss.md)
  - Detailed analysis and expected behavior for packet loss scenarios.

EOF
