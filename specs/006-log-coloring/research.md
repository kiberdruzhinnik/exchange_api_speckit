# Research: Configurable Log Coloring

## Decision 1: Keep the setting in the existing application configuration

- **Decision**: Add `EXCHANGE_API_LOG_COLOR` to the existing application configuration model and use it when the logging subscriber is initialized.
- **Rationale**: The service already loads its `EXCHANGE_API_*` settings in `src/config.rs`; centralizing the new setting there keeps parsing and default behavior consistent. Logging initialization currently occurs in `src/main.rs`, so the parsed choice can be passed to the formatter at startup.
- **Alternatives considered**: Read the environment variable independently inside logging initialization. Rejected because it duplicates configuration parsing and makes behavior harder to test alongside the other settings.

## Decision 2: Use the clarified false-value set and retain enabled behavior for all other values

- **Decision**: Treat `false`, `0`, `no`, and `off` as disabled values regardless of letter case. Treat all other values, including an absent setting, as enabled.
- **Rationale**: This is the operator-selected behavior recorded in the feature spec. It gives common shell-friendly ways to disable color and keeps typo or unrecognized values from unexpectedly removing colors.
- **Alternatives considered**: Accept only the literal `false`; reject unsupported values at startup. These alternatives would differ from the clarified requirement.

## Decision 3: Configure the existing formatter without adding a dependency

- **Decision**: Apply the resolved boolean to the existing `tracing_subscriber::fmt::layer()` ANSI setting.
- **Rationale**: The repository uses `tracing-subscriber` 0.3.23 with its `fmt` feature. Its formatting layer exposes `with_ansi(bool)` to enable or suppress ANSI formatting. This directly addresses the behavior without adding another logger or dependency.
- **Alternatives considered**: Add a separate logging library or implement custom output filtering. Rejected because the existing formatter already exposes the required control.
- **Reference**: [tracing-subscriber 0.3.23 `fmt::Layer`](https://docs.rs/tracing-subscriber/0.3.23/tracing_subscriber/fmt/struct.Layer.html) documents `with_ansi` as controlling ANSI terminal escape codes.

## Decision 4: Keep configuration documentation with the feature's operator quickstart

- **Decision**: Document the environment variable, default, recognized disabling values, and verification steps in this feature's `quickstart.md`.
- **Rationale**: The repository has no top-level application configuration guide. The feature quickstart is the established place for operator configuration and validation instructions.
- **Alternatives considered**: Create a new repository-wide configuration manual. Rejected for this single setting because it would establish a broader document without a current project-wide configuration guide.
