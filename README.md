# Ritsu: A Deterministic Process Scheduler

<p align="left">
  <img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License">
  <img src="https://img.shields.io/badge/Built_with-Rust-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/GUI-Tauri-24C8DB?logo=tauri&logoColor=white" alt="Tauri">
  <img src="https://img.shields.io/badge/Platform-Linux%20%7C%20Windows%20%7C%20macOS-lightgrey" alt="Platform">
</p>

A lightweight process scheduler that deterministically coordinates multi-process pipelines based on periodic cycles and DAG dependencies.

![Ritsu Visualizer Screenshot](docs/assets/rt-vis_screenshot.png)

Note: Ritsu targets predictable, soft real-time coordination and reproducible execution order across processes, not hard real-time execution guarantees.

## Features

- **Periodic Execution**: Predictable execution based on predefined timing and cycles.
- **Dependency Management**: Flexible scheduling based on process dependencies (sequential and parallel).
  - Example: Image processing pipelines using camera frames.
- **Multi-Language Support**: A simple messaging protocol supporting multiple programming languages.
  - Modular transport support (UDP-based by default).
  - This repository includes sample clients for **Rust**, **Python**, and **C/C++**.
- **Simulation & Visualization**: Interactive GUI for designing schedules and analyzing runtime execution.
  - Visual schedule editing and simulation with real-time timing validation.
  - Execution log overlay against planned schedules with microsecond-precision timing deltas.
  - Quantitative statistics (duration percentiles, budget margins, concurrency, cycle jitter) with instant jump to anomalies.

**Visualizer Highlights:**

<table>
  <thead>
    <tr>
      <th colspan="2">Create Mode</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td align="center" width="50%">
        <img src="docs/assets/rt-vis_create-client-edit.png" width="400" /><br />
        <strong>Client &amp; DAG Configuration</strong><br />
        Configure client intervals, offsets, start cycles, expected durations, and DAG dependencies interactively.
      </td>
      <td align="center" width="50%">
        <img src="docs/assets/rt-vis_create-realtime-validation.png" width="400" /><br />
        <strong>Real-Time Validation &amp; Planned Load</strong><br />
        Drag-and-drop editing immediately highlights deadline overruns and skipped cycles alongside planned concurrency metrics.
      </td>
    </tr>
  </tbody>
</table>

<table>
  <thead>
    <tr>
      <th colspan="2">Analyze Mode</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td align="center" width="50%">
        <img src="docs/assets/rt-vis_analyze-actual-overlay.png" width="400" /><br />
        <strong>Actual vs. Planned Overlay &amp; Client Stats</strong><br />
        Overlay actual execution logs onto planned schedules with microsecond deltas, expected budget margins, and duration histograms.
      </td>
      <td align="center" width="50%">
        <img src="docs/assets/rt-vis_analyze-system-health.png" width="400" /><br />
        <strong>System Health Overview</strong><br />
        Inspect clean cycle rates, aggregated anomaly counts, and per-client health status at a glance.
      </td>
    </tr>
    <tr>
      <td align="center" width="50%">
        <img src="docs/assets/rt-vis_analyze-events.png" width="400" /><br />
        <strong>Anomaly Filtering &amp; Timeline Jump</strong><br />
        Filter anomaly events and jump directly to the exact cycle on the timeline.
      </td>
      <td align="center" width="50%">
        <img src="docs/assets/rt-vis_analyze-jitter-concurrency.png" width="400" /><br />
        <strong>Cycle Jitter &amp; Concurrency Analysis</strong><br />
        Track cycle-to-cycle jitter bias, percentiles, and distribution alongside planned vs. actual concurrency.
      </td>
    </tr>
  </tbody>
</table>

## Why Ritsu?

Coordinating multi-language pipelines (such as C sensor drivers, Python ML inference, and Rust motion planners) often requires deterministic timing across heterogeneous processes without adding large runtime overhead or relying on fragile ad-hoc timer loops.

Ritsu complements existing systems with a focused, lightweight approach:

- **Minimal Footprint & Easy Integration**: Pure standard-library coordination over modular transports (UDP by default). Works cleanly alongside existing communication middleware, shared memory, or IPC, without imposing heavy dependencies or complex build systems on client processes.
- **Control-Plane Focus**: Ritsu strictly manages process execution timing, DAG dependencies, and lifecycle states—leaving payload data transfer (such as shared memory via [Eclipse iceoryx2](demo/iceoryx2-pipeline/README.md), ROS topics, or custom IPC) decoupled and flexible.
- **Deterministic & Fail-Safe**: Built-in state management handles deadline overruns and cycle skips predictably, allowing clients to transition safely into fail-safe states.
- **Simulation & Verification First**: Design and inspect schedules visually using `rt-vis` before deploying to real hardware, and verify actual execution logs against your planned timeline.

## Quickstart

Experience Ritsu's deterministic scheduling through self-contained reference demos. Each demo isolates its own build artifacts and dependencies, and runs with a single script.

### Available Demos

| Demo | Focus | Stack |
| :--- | :--- | :--- |
| **[robot-pipeline](demo/robot-pipeline/README.md)** | 5 Hz **Fork-Join DAG** coordination (zero external dependencies) | C, Python, Rust |
| **[iceoryx2-pipeline](demo/iceoryx2-pipeline/README.md)** | 10 Hz **Control + Data Plane** integration with fallback, `SKIP` recovery, and `E-STOP` | Rust, Python, [Eclipse iceoryx2](https://iceoryx.io/) ([GitHub](https://github.com/eclipse-iceoryx/iceoryx2)) |

**Prerequisites:** Rust (>= 1.85) and Python (>= 3.12) for all demos, plus a C/C++ compiler with CMake (>= 3.14) for C-based demos (`robot-pipeline`). See each demo's README for OS-specific setup (e.g., `venv` on Linux).

### Run & Inspect

1. **Run a demo** (automatically builds and launches `rt-server` and all client nodes in dedicated console windows):
   - **Windows (PowerShell):**
     ```powershell
     # e.g., demo/robot-pipeline or demo/iceoryx2-pipeline
     cd demo/robot-pipeline
     .\run_demo.ps1
     ```
   - **Linux / macOS (Bash):**
     ```bash
     # e.g., demo/robot-pipeline or demo/iceoryx2-pipeline
     cd demo/robot-pipeline
     ./run_demo.sh
     ```

2. **Inspect with the visualizer (`rt-vis`)**:  
   Launch `rt-vis` and open `demo/<demo-name>/logs/rt-server.log` in **Analyze** mode to inspect scheduled vs. actual timelines, timing and jitter statistics, budget margins, and anomaly events:
   - **Pre-built binary (Windows):** Download the latest release from [GitHub Releases](https://github.com/nao-kawano/ritsu-scheduler/releases) and run `rt-vis.exe`.
   - **From source (Windows / Linux / macOS, requires Node.js >= 18):**
     ```bash
     # Install frontend dependencies (first time only)
     npm --prefix rt-vis-rs install

     # Launch visualizer in development mode
     npm --prefix rt-vis-rs run tauri dev
     ```

Refer to each demo's guide in the table above for architecture diagrams, OS-specific prerequisites, and cycle-by-cycle scenario details.

## Process Scheduling

The sequence diagram below illustrates how Ritsu schedules processes.
For more details, see the [documentation](./docs/README.md).

### Example Scenario: A -> (B, C)

- Process A starts periodically.
- Processes B and C start simultaneously once A completes.

```mermaid
sequenceDiagram
  participant S as Scheduler (Ritsu)
  participant A as Process A
  participant B as Process B
  participant C as Process C

  A -) S: READY
  B -) S: READY
  C -) S: READY

  Note over S: Time-based trigger -> Run A
  S --) A: START
  Note over A: Processing...
  A -) S: DONE

  Note over S: Dependency check -> Run B and C
  S --) B: START
  S --) C: START
  A -) S: READY (Next cycle)
  Note over B,C: Processing...

  B -) S: DONE
  B -) S: READY (Next cycle)
  C -) S: DONE
  C -) S: READY (Next cycle)
```

## Repository Structure

- **[rt-config-rs](./rt-config-rs)**: Common configuration structures and validation logic shared between the scheduler and visualization tools.
- **[rt-core-rs](./rt-core-rs)**: The core scheduling logic. Manages process states and dependency resolution.
- **[rt-server-rs](./rt-server-rs)**: The Ritsu scheduler server. Responsible for client management, timing control, and coordinating execution with the core scheduler.
- **[rt-message-rs](./rt-message-rs)**: Common message types and serialization protocol used for server-client communication.
- **[rt-client-rs](./rt-client-rs)**: Client library for Rust.
- **[rt-client-py](./rt-client-py)**: Client library for Python.
- **[rt-client-c](./rt-client-c)**: Client library for C/C++.
- **[rt-vis-rs](./rt-vis-rs)**: Tauri-based visualizer application for schedule configuration, simulation, execution log analysis, and timing statistics.
- **[demo](./demo)**: Reference demos showcasing multi-language DAG orchestration and external data-plane integration.

## Documentation

See the [docs](./docs/README.md) directory for detailed specifications and guides.

## License

This project is licensed under the Apache License, Version 2.0. See the [LICENSE](LICENSE) file for details.
