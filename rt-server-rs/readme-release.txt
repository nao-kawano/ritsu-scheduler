Ritsu Server
==================================================

Ritsu Server is the core server component of Ritsu Scheduler.

■ How to Run
This program is a console application. While it can be started by double-clicking, it is highly recommended to run it from a terminal (such as Command Prompt or PowerShell) so that you can see the log output and keep the window open in case of errors:

    .\rt-server.exe

■ Prerequisites
To run the server, "config.toml" must be located in the same directory as this executable. Please copy or edit the provided sample config.toml before running.

■ Documentation
For detailed specifications and further details, please refer to the documents in the "docs/" directory of the GitHub repository. Particularly useful documents include:
- docs/configuration.md   : Configuration rules and options.
- docs/message-sequence.md : Communication sequence between server and clients.
- docs/state-management.md : Process state transitions and lifecycles.

GitHub Repository:
https://github.com/nao-kawano/ritsu-scheduler
