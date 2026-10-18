Ritsu Server
==================================================

Ritsu Server is the core server component of Ritsu Scheduler.

■ How to Run

This program is a console application.
While it can be started by double-clicking, it is highly recommended to run it from a terminal (such as Command Prompt or PowerShell) so that you can see the log output and keep the window open in case of errors.

By default, the server loads "config.toml" from the current working directory:

    .\rt-server.exe

You can also specify command-line arguments to override settings:

    .\rt-server.exe --config path/to/your_config.toml
    .\rt-server.exe --log-level debug --no-log-file

Common command-line options:
- -c, --config <FILE>       : Path to the configuration file (default: ./config.toml).
- --log-level <LEVEL>       : Override console log level (off, error, warn, info, debug, trace).
- --log-file-dir <DIR>      : Specify log output directory to enable or override file logging.
- --log-file-level <LEVEL>  : Override file log level (default: debug).
- --no-log-file             : Disable file logging regardless of configuration.
- -h, --help                : Show help information.

■ Log Output & Rotation

When file logging is enabled (the default sample config sets log_file_dir = "./logs"), logs are written asynchronously to "rt-server.log" located relative to the configuration file.
On each startup, existing logs are automatically shifted into backup files (up to 5 generations: rt-server.1.log through rt-server.5.log) to prevent data loss.

■ Prerequisites

To run the server with default settings, "config.toml" must be located in the same directory as this executable.
Please copy or edit the provided sample config.toml before running.
Alternatively, you can specify any configuration file path using the options above.

■ Documentation

For detailed specifications and further details, please refer to the documents in the "docs/" directory of the GitHub repository.
Particularly useful documents include:
- docs/configuration.md    : Configuration rules, command-line overrides, and precedence.
- docs/logging.md          : Logging guidelines, levels, and rotation specifications.
- docs/message-sequence.md : Communication sequence between server and clients.
- docs/state-management.md : Process state transitions and lifecycles.

GitHub Repository:
https://github.com/nao-kawano/ritsu-scheduler
