# Data Model: Configurable Log Coloring

This feature adds no persisted or user-managed data. It adds one process configuration value.

## Log Color Setting

- **Purpose**: Controls whether the process's human-readable log formatter emits ANSI color and formatting control sequences.
- **Source**: `EXCHANGE_API_LOG_COLOR` environment variable.
- **Effective value**: Boolean, resolved when application configuration is loaded at startup.
- **Default**: Enabled when the variable is absent.
- **Parsing**: `false`, `0`, `no`, and `off` disable coloring, case-insensitively. All other values resolve to enabled.
- **Lifecycle**: Read once at process startup; changing the environment requires restarting the process.
- **Relationships**: Passed from application configuration to logging initialization. It does not affect log filtering, message content, severity, or persistence.
