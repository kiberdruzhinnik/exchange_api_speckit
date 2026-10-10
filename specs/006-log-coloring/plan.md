# Implementation Plan: Configurable Log Coloring

**Branch**: `006-log-coloring` | **Date**: 2026-10-10 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification at `specs/006-log-coloring/spec.md`

## Summary

Add the `EXCHANGE_API_LOG_COLOR` startup setting to the existing application configuration and use it to control ANSI formatting in the existing tracing subscriber. Coloring stays enabled by default. Values `false`, `0`, `no`, and `off` disable it case-insensitively; all other values enable it. Document the setting and its verification steps for operators.

## Technical Context

**Language/Version**: Rust 2024 edition, minimum Rust 1.85

**Primary Dependencies**: Existing `tracing-subscriber` 0.3.23 (`fmt` feature); no new dependencies

**Storage**: None; the setting is process configuration only

**Testing**: Existing Rust unit tests in `src/config.rs`; add parsing coverage and subprocess formatter-output coverage in `tests/log_color.rs`

**Target Platform**: Existing Linux server/container service; environment configuration is process-wide

**Project Type**: Existing single Rust REST microservice

**Performance Goals**: No measurable request-path change; setting is resolved at startup and only changes log formatting

**Constraints**: Preserve enabled coloring by default, honor the four clarified disabling values case-insensitively, leave all other values enabled, and preserve log message content and severity. Do not add a logging dependency or persistent state.

**Scale/Scope**: One process-wide setting and the existing log formatter; no API routes or service boundaries change

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- REST API Contracts First: **PASS** — no REST interface changes.
- Documentation Is Part of Delivery: **PASS** — document the setting, default, accepted values, and validation in the feature operator quickstart and environment contract.
- Clear Microservice Boundaries: **PASS** — implement within the existing service configuration and logging startup path; add no service or shared runtime state.
- Compatibility and Change Management: **PASS** — preserve the current enabled-by-default behavior and existing log messages.
- Practical Quality and Operability: **PASS** — cover configuration parsing and actual output behavior; require a final container build, Semgrep source analysis, and Trivy image scan, with results and unavailable scans or fixes recorded.
- Private-network security: **PASS** — no authentication, authorization, data access, or exposure changes.

No constitution violations identified.

## Project Structure

### Documentation (this feature)

```text
specs/006-log-coloring/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
└── contracts/
    └── log-color-config.md
```

### Source Code (repository root)

```text
src/
├── config.rs       # Resolve EXCHANGE_API_LOG_COLOR and provide its default
└── main.rs         # Apply configured ANSI behavior during tracing initialization

tests/
└── log_color.rs    # Subprocess checks for disabled, default, and enabled output
```

Configuration parsing tests remain with `src/config.rs`; formatter output coverage should use a captured subscriber or subprocess so the global tracing subscriber does not make tests order-dependent.

**Structure Decision**: Extend the existing configuration and logging initialization in the single Rust service. No new production module, dependency, persistent model, API contract, or service is needed.

## Complexity Tracking

No constitution violations require a complexity exception.

## Phase 0: Outline & Research

Research decisions and alternatives are recorded in [research.md](research.md). No unresolved clarifications remain.

## Phase 1: Design & Contracts

- [data-model.md](data-model.md) defines the process-only configuration value and its parsing and lifecycle rules.
- [contracts/log-color-config.md](contracts/log-color-config.md) records the environment configuration contract; no HTTP or OpenAPI contract changes are needed.
- [quickstart.md](quickstart.md) documents operator configuration and validation scenarios.

### Constitution Re-check

- The setting remains within the existing service configuration and logger initialization: **PASS**.
- Existing output content and severity are preserved, and default coloring remains enabled: **PASS**.
- Configuration parsing and actual formatter output have explicit validation coverage: **PASS**.
- No REST contract, data storage, security boundary, dependency, or service boundary changes: **PASS**.
- Final container build, Semgrep scan, and Trivy scan are explicit completion gates; findings and any unavailable scan or fix are documented in the quickstart: **PASS**.
