# Tasks: Configurable Log Coloring

**Input**: Design documents from `specs/006-log-coloring/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/log-color-config.md`, and `quickstart.md`

**Tests**: Test tasks are included because the feature scenarios and plan specify configuration and captured-output validation.

**Organization**: Tasks are grouped by user story. Shared configuration and logger behavior are implemented in User Story 1; User Story 2 independently verifies enabled defaults and documents operator configuration.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Tasks touch different files and have no dependency on incomplete tasks.
- **[Story]**: User story traced to `spec.md`.
- Every task names its exact file path.

## Phase 1: Setup

**Purpose**: No new project scaffold, dependency, or tooling is required; the existing Rust service and test setup are used.

No setup tasks.

## Phase 2: Foundational

**Purpose**: No separate foundational changes are needed. The first user story adds the shared configuration and logger behavior.

No foundational tasks.

## Phase 3: User Story 1 - Disable log colors (Priority: P1) 🎯 MVP

**Goal**: Let operators disable ANSI coloring for colorless terminals while preserving log messages and severity.

**Independent Test**: Run the service with `EXCHANGE_API_LOG_COLOR=off` and confirm captured logs contain no ANSI escape sequences and retain their message text and severity.

### Tests for User Story 1

- [X] T001 [P] [US1] Add configuration unit tests for `false`, `0`, `no`, `off` in mixed case and an unrecognized value in `src/config.rs`; verify only the four disabling values resolve to disabled.
- [X] T002 [P] [US1] Add a subprocess output test in `tests/log_color.rs` that starts the service with `EXCHANGE_API_LOG_COLOR=off`, captures its startup log, and verifies there are no ANSI escape sequences while text and severity remain present.

### Implementation for User Story 1

- [X] T003 [US1] Add the process-wide `log_color` setting to `AppConfig` and parse `EXCHANGE_API_LOG_COLOR` in `src/config.rs`; default to enabled and treat `false`, `0`, `no`, and `off` case-insensitively as disabled while all other values remain enabled.
- [X] T004 [US1] Load `AppConfig` before initializing tracing and apply its `log_color` value to the existing formatter in `src/main.rs`, without changing log filtering or message formatting beyond ANSI control sequences.

**Checkpoint**: With the setting disabled, the process emits the same log content and severity without ANSI color or formatting control sequences.

## Phase 4: User Story 2 - Keep colored logs by default (Priority: P1)

**Goal**: Preserve colored output when the setting is absent or explicitly enabled, and document how operators configure the behavior.

**Independent Test**: Start the service with the variable unset and with `EXCHANGE_API_LOG_COLOR=true`; confirm both captured outputs retain colored formatting.

### Tests for User Story 2

- [X] T005 [US2] Extend the subprocess checks in `tests/log_color.rs` to verify the variable-unset and explicitly enabled runs retain ANSI coloring, and that other unrecognized values remain enabled.

### Implementation and Documentation for User Story 2

- [X] T006 [P] [US2] Document `EXCHANGE_API_LOG_COLOR`, its enabled default, disabling values, startup lifecycle, and shell examples in `specs/006-log-coloring/quickstart.md` and `specs/006-log-coloring/contracts/log-color-config.md`.

**Checkpoint**: The service retains its enabled-by-default behavior, and operators can find the setting and its accepted values in the feature documentation.

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Validate the feature and meet the constitution's release gates.

- [X] T007 Run the documented Rust checks (`cargo test config::tests` and `cargo test --test log_color`) from `specs/006-log-coloring/quickstart.md` and record outcomes there.
- [X] T008 Build the final Linux amd64 container image using the command in `specs/006-log-coloring/quickstart.md` and record the image tag and build result there.
- [X] T009 Run Semgrep source analysis using the command in `specs/006-log-coloring/quickstart.md`, resolve all findings, and record the result there.
- [X] T010 Scan the built image with Trivy using the command in `specs/006-log-coloring/quickstart.md`, fix every High or Critical finding with an available fix, and document findings and any unavailable fixes there.

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No work; no project initialization is needed.
- **Foundational (Phase 2)**: No separate work; the feature's shared setting is implemented in US1.
- **User Story 1 (Phase 3)**: Starts immediately. T001 and T002 can be prepared in parallel; T003 follows T001; T004 follows T003. T002's runtime assertion is expected to pass after T004.
- **User Story 2 (Phase 4)**: Depends on US1's configuration and logger behavior. T005 follows T002 and T004. T006 can proceed in parallel with T005.
- **Polish (Phase 5)**: T007–T010 follow T001–T006. T010 depends on T008 because it scans the built image.

### User Story Dependencies

- **User Story 1 (P1)**: No dependency on another story; delivers the disabled-color behavior and is the MVP.
- **User Story 2 (P1)**: Depends on US1's shared setting and logger initialization to verify the enabled default; adds default-path acceptance and operator documentation.

### Parallel Opportunities

- **US1**: T001 (`src/config.rs`) and T002 (`tests/log_color.rs`) can be prepared in parallel because they touch separate files; implementation waits for the relevant tests.
- **US2**: T005 and T006 can proceed in parallel because tests and documentation are in separate files, after the US1 runtime behavior exists.
- The user stories themselves cannot be completed in parallel because US2 verifies the shared logger configuration introduced by US1.

## Parallel Example: User Story 1

```text
Task: T001 - Add configuration parsing tests in src/config.rs
Task: T002 - Add disabled-color subprocess coverage in tests/log_color.rs
```

## Implementation Strategy

### MVP First (User Story 1)

1. Prepare T001 and T002 in parallel and confirm their assertions expose the missing behavior.
2. Implement configuration parsing in T003 and wire it to tracing initialization in T004.
3. Validate US1: disabled output has no ANSI sequences and preserves log text and severity.
4. Complete US2 to verify enabled defaults and publish operator documentation.
5. Run the Rust checks in T007, build the image in T008, and complete the required source and image scans in T009–T010.

### Incremental Delivery

1. Deliver US1 for colorless terminals.
2. Deliver US2 coverage for the existing default and configuration documentation.
3. Run final tests, build the release image, complete Semgrep and Trivy scans, and record all results in the quickstart.

## Notes

- `[P]` appears only for tasks that use different files and have no incomplete-task dependency.
- Every task has a sequential ID, a story label where required, and an explicit file path or validation command with its result path.
- Run the new tests before implementation and confirm the expected failure, then rerun them after implementation.

## Phase 6: Convergence

- [X] T011 Extend `tests/log_color.rs` to capture multiple representative log messages with coloring enabled and disabled, then verify disabling colors preserves message content, severity, and ordering per US1/AC2, FR-004, and SC-003 (partial).
