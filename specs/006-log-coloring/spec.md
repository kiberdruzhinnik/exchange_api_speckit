# Feature Specification: Configurable Log Coloring

**Feature Branch**: `006-log-coloring`

**Created**: 2026-10-10

**Status**: Completed

**Input**: User description: "user should have ability to configure log coloring
via env variable. default is enabled, but option should be to disable coloring
to support colorless terminals."

## Clarifications

### Session 2026-10-10

- Q: What environment variable name should operators use to control log
    coloring? → A: `EXCHANGE_API_LOG_COLOR`.
- Q: Which values should `EXCHANGE_API_LOG_COLOR` accept to disable colored
    logs? → A: `false`, `0`, `no`, and `off`, case-insensitively; all other
    values leave coloring enabled.
- Q: Should this feature explicitly require a final image build plus Semgrep and
    Trivy scans, with results recorded in its quickstart? → A: Include explicit
    release gates, as required by the project constitution.
- Q: Should the disabling-value acceptance scenario be grouped under the story
    about disabling log colors? → A: Move it to User Story 1; User Story 2
    covers enabled and fallback behavior.
- Q: Should the plan’s project structure list `tests/log_color.rs` as the new
    formatter-output test file? → A: Yes, list `tests/log_color.rs` as the
    subprocess formatter-output test file.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Disable log colors (Priority: P1)

A service operator running the application in a terminal or log collector that
cannot display ANSI colors can disable colored log output through an environment
setting. When disabled, log messages remain readable and contain no color
control sequences.

**Why this priority**: Color control sequences can make logs difficult to read
in colorless terminals and downstream log tools. Disabling them directly
addresses that operational need.

**Independent Test**: Start the application with the color setting disabled,
produce representative log messages, and verify that messages contain no color
control sequences while retaining their normal text and severity information.

**Acceptance Scenarios**:

1. **Given** the color setting is disabled, **When** the application writes log
      messages, **Then** the messages contain no color control sequences.
2. **Given** the color setting is disabled, **When** the application writes log
      messages, **Then** message text, severity, and ordering remain available
      as usual.
3. **Given** `EXCHANGE_API_LOG_COLOR` is set to `false`, `0`, `no`, or `off` in
      any letter case, **When** the application writes log messages, **Then**
      coloring is disabled.

### User Story 2 - Keep colored logs by default (Priority: P1)

An operator who does not configure the color setting receives the existing
colored log output by default.

**Why this priority**: Existing deployments should retain the current log
presentation without requiring configuration changes.

**Independent Test**: Start the application without the color setting and verify
that representative log messages use the existing colored presentation.

**Acceptance Scenarios**:

1. **Given** the color setting is absent, **When** the application writes log
      messages, **Then** colored output is enabled.
2. **Given** the color setting is explicitly enabled, **When** the application
      writes log messages, **Then** colored output is enabled.
3. **Given** `EXCHANGE_API_LOG_COLOR` has any other value, **When** the
      application writes log messages, **Then** coloring remains enabled.

### Edge Cases

- Disabling colors must not suppress, alter, or omit log messages.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The application MUST allow an operator to configure log coloring
    through the `EXCHANGE_API_LOG_COLOR` environment variable.
- **FR-002**: The application MUST enable log coloring by default when the
    environment variable is absent.
- **FR-003**: The application MUST disable log coloring when
    `EXCHANGE_API_LOG_COLOR` is set to `false`, `0`, `no`, or `off`, regardless
    of letter case. Any other value MUST leave coloring enabled.
- **FR-004**: When coloring is disabled, the application MUST emit log messages
    without color control sequences and MUST preserve the messages' normal
    content and severity information.
- **FR-005**: The application MUST document the environment variable, its
    default, and how to disable coloring.

### Key Entities *(include if data involved)*

Not applicable. This feature changes log presentation configuration and
introduces no user-managed data.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a run with coloring disabled, 100% of sampled log messages
    contain no color control sequences.
- **SC-002**: In a run with the setting absent or enabled, colored output
    matches the current default behavior.
- **SC-003**: Disabling coloring leaves all sampled log messages and their
    severity information intact.
- **SC-004**: An operator can identify the setting, its default, and the
    disabling value from the application configuration documentation.

## Assumptions

- The setting is process-wide and is applied when the application starts.
- The application currently emits colored logs by default, and this behavior is
    to remain compatible.
- Feature completion includes the constitution-required final container build,
    Semgrep source analysis, and Trivy image scan; results and any unavailable
    scan or fix are recorded in the feature quickstart.
