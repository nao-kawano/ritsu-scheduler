# Ritsu Client for C/C++

## 1. Overview

`rt-client-c` is a lightweight C/C++ client library for the Ritsu process scheduler.

- **Single-Header Library**: The entire library is implemented within `rtclient.h`. Simply include it in your project.
- **Zero-Allocation**: Designed for high reliability and embedded systems, using no dynamic memory allocation (`malloc`/`free`).
- **C99/C++ Compatible**: Written in pure C (C99 compatible) and can be used seamlessly in C++ projects.
- **Cross-Platform**: Out-of-the-box support for both Linux (POSIX sockets) and Windows (Winsock2).

## 2. Installation / Setup

To use this library, copy `rtclient.h` into your source tree.

In exactly **one** C or C++ source file, define `RTCLIENT_IMPLEMENTATION` before including `rtclient.h` to instantiate the implementation:

```c
#define RTCLIENT_IMPLEMENTATION
#include "rtclient.h"
```

In other source files, simply include the header normally:

```c
#include "rtclient.h"
```

### Linker Requirements

- **Windows**: You must link against the following system libraries:
  - `ws2_32` (for Winsock2 socket operations)
  - `winmm` (for high-precision multimedia timers)
- **Linux**: No additional system libraries are required.

## 3. Quick Start

Here is a minimal C++ example demonstrating how to initialize the client, join the server, wait for scheduling events in a loop, notify task completion, and exit cleanly:

```cpp
#include <iostream>
#define RTCLIENT_IMPLEMENTATION
#include "rtclient.h"

int main() {
    RtClient client;

    // Initialize client config (host, port, client_id, run_cycle_sec, startup_wait_sec)
    if (!rtclient_init(&client, "127.0.0.1", 7878, 1, 2.0, 60.0)) {
        std::cerr << "Initialization failed" << std::endl;
        return -1;
    }

    // Join the scheduler server
    if (!rtclient_join(&client)) {
        std::cerr << "Failed to join server" << std::endl;
        rtclient_cleanup(&client);
        return -1;
    }
    std::cout << "Successfully joined the server!" << std::endl;

    // Main execution loop
    for (int i = 0; i < 5; ++i) {
        RtMessage msg;
        if (!rtclient_wait_next(&client, &msg)) {
            std::cerr << "Failed to wait next event" << std::endl;
            break;
        }

        if (msg.mtype == RT_MSG_START) {
            std::cout << "Got START message, executing task..." << std::endl;
            // Execute your processing logic here
            rtclient_notify_done(&client);
        } else if (msg.mtype == RT_MSG_ERROR) {
            std::cerr << "Error message received" << std::endl;
            break;
        }
    }

    // Exit cleanly and cleanup socket resources
    rtclient_exit(&client);
    rtclient_cleanup(&client);

    return 0;
}
```

## 4. Running the Example

You can build and run the provided C++ example program using CMake.

First, navigate to the `examples` directory:

```bash
cd examples
```

### On Windows (MSVC)

Run the batch file to configure and build the project using Visual Studio 2022:

```cmd
build_vs2022.bat
```

After a successful build, run the executable:

```cmd
build\Release\example_cpp.exe
```

### On Linux (GCC/Clang)

Run the shell script to build:

```bash
chmod +x build.sh
./build.sh
```

After a successful build, run the executable:

```bash
./build/example_cpp
```

## 5. Notes & Platform-specific details

- **High-Precision Windows Timer**: Windows platforms default to a 15.6ms timer resolution. To guarantee 1ms precision for retry and timeout handling, the library dynamically invokes `timeBeginPeriod(1)` and `timeEndPeriod(1)`.
- **Disabling Internal Logging**: The library prints communication logs (`>> send` and `<< recv`) to the standard output. If you want to disable these logs completely to run silently, define `RTCLIENT_NO_LOG` before including `rtclient.h` in your implementation file:
  ```c
  #define RTCLIENT_NO_LOG
  #define RTCLIENT_IMPLEMENTATION
  #include "rtclient.h"
  ```
