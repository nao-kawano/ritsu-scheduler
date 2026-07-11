Ritsu Server
==================================================

Ritsu Server is the core server component of Ritsu Scheduler.

■ How to Run
This program is a console application. While it can be started by double-clicking, it is highly recommended to run it from a terminal (such as Command Prompt or PowerShell) so that you can see the log output and keep the window open in case of errors.

By default, the server loads "config.toml" from the current working directory:

    .\rt-server.exe

You can also specify a custom configuration file path using the "--config" (or "-c") option:

    .\rt-server.exe --config path/to/your_config.toml

■ Prerequisites
To run the server with default settings, "config.toml" must be located in the same directory as this executable. Please copy or edit the provided sample config.toml before running. Alternatively, you can specify any configuration file path using the options above.

■ Documentation
For detailed specifications and further details, please refer to the documents in the "docs/" directory of the GitHub repository. Particularly useful documents include:
- docs/configuration.md   : Configuration rules and options.
- docs/message-sequence.md : Communication sequence between server and clients.
- docs/state-management.md : Process state transitions and lifecycles.

GitHub Repository:
https://github.com/nao-kawano/ritsu-scheduler
