# Tasks: MOEX Ticker History API

**Input**: Design documents from `specs/001-moex-ticker-update/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/openapi.yaml`

**Tests**: Include automated API and MOEX fixture tests because the specification requires acceptance coverage for success, empty history, invalid symbols, and upstream failures (SC-003).

**Organization**: Tasks are grouped by user story so each story can be implemented and validated as an increment.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel because it touches different files and has no dependency on incomplete tasks.
- **[Story]**: Maps a task to a user story from `spec.md`.
- Every task names its target file path.

## Path Conventions

- Single Rust web service: `src/` and `tests/` at repository root.
- Feature design artifacts: `specs/001-moex-ticker-update/`.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Align design artifacts with the clarified behavior and initialize the Rust project.

- [X] T001 [P] Reconcile `specs/001-moex-ticker-update/plan.md`, `research.md`, `data-model.md`, and `contracts/openapi.yaml` with the latest spec: use the primary board on each trading date, populate `facevalue` from its current `LOTSIZE`, and send no MOEX subscription credentials.
- [X] T002 [P] Initialize the Rust web service manifest and lockfile with the planned Axum, Tokio, Reqwest/Rustls, Serde, tracing, Moka cache, and response-bytes dependencies in `Cargo.toml` and `Cargo.lock`.
- [X] T003 [P] Configure the stable Rust toolchain and formatting settings in `rust-toolchain.toml` and `rustfmt.toml`.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Provide shared configuration, HTTP routing, error responses, and logging needed by both user stories.

- [X] T004 Load the listen address, port 8080 default, upstream request timeout, history-cache TTL, and cache byte capacity into typed application configuration in `src/config.rs` and `src/main.rs`.
- [X] T005 Define the shared JSON error response and HTTP status mapping from `contracts/openapi.yaml` in `src/http/errors.rs`.
- [X] T006 Create the Axum application router and liveness/readiness endpoints in `src/http/routes.rs` and `src/http/mod.rs`.
- [X] T007 Initialize structured request and upstream dependency logging with `tracing`, including request duration and cache hit/miss status, in `src/main.rs` and `src/http/routes.rs`.

**Checkpoint**: The service can start with shared configuration, expose its base routes, and produce consistent errors and logs.

---

## Phase 3: User Story 1 - Retrieve a ticker's daily history (Priority: P1) 🎯 MVP

**Goal**: Return all unauthenticated MOEX ISS daily records available for a ticker, using the board that was primary on each trading date and the agreed six-field JSON shape.

**Independent Test**: Request a known ticker against fixture-backed MOEX responses and confirm a chronologically ordered JSON array with all six fields, nulls for missing values, and current primary-board `LOTSIZE` in the `facevalue` property for each record.

### Tests for User Story 1

- [X] T008 [P] [US1] Update MOEX fixtures to cover primary-board current `LOTSIZE` references and the SBER 2026-10-06 expected value in `tests/fixtures/moex/`.
- [X] T009 [P] [US1] Update fixture-driven tests for named-column mapping, null and missing trailing value handling, date normalization, ascending order, malformed date rejection before board filtering, and `LOTSIZE` to `facevalue` mapping in `tests/moex_mapping.rs`.
- [X] T010 [P] [US1] Update API contract tests to assert the SBER 2026-10-06 `facevalue` value is `1`, confirm a warm cache hit, and preserve empty-array behavior in `tests/api_contract.rs` using `specs/001-moex-ticker-update/contracts/openapi.yaml`.

### Implementation for User Story 1

- [X] T011 [US1] Define MOEX response DTOs and the public `DailyMarketRecord` in `src/moex/models.rs` and `src/domain.rs`, preserving required `date` and numeric-or-null `close`, `high`, `low`, `volume`, and `facevalue` fields.
- [X] T012 [US1] Implement retrieval of current board-specific `LOTSIZE` reference data in the unauthenticated MOEX ISS client at `src/moex/client.rs`, using the primary board selected for each trading date.
- [X] T013 [US1] Resolve the primary board for each trading date from MOEX security listing history in `src/moex/board.rs`; do not hard-code a board or apply today's board to older dates.
- [X] T014 [US1] Map named MOEX market-history columns and selected-board `LOTSIZE` into the API's `facevalue` property in `src/moex/mapping.rs`, retaining null values and rejecting rows without a valid trading date.
- [X] T023 [US1] Add a bounded process-local async cache of complete successful ticker responses in `src/cache.rs`, configured by TTL and byte capacity in `src/config.rs`; coalesce concurrent misses per symbol, cache only after complete pagination and mapping, never serve stale entries after upstream errors, and log request duration with cache hit/miss outcome for latency monitoring.
- [X] T015 [US1] Update `GET /v1/moex/{SYMBOL}` in `src/http/routes.rs` to orchestrate cache lookup, per-date board selection and current board LOTSIZE lookup, full pagination on cache misses, date serialization at midnight UTC, and ascending ordering.

**Checkpoint**: User Story 1 returns complete publicly available history and can be validated independently with the fixtures and API contract.

---

## Phase 4: User Story 2 - Handle invalid or unavailable ticker data (Priority: P2)

**Goal**: Give clients distinct documented responses for invalid or unknown symbols and MOEX ISS failures, without converting failures into empty history.

**Independent Test**: Exercise malformed and unknown symbols, a valid symbol with no records, and simulated MOEX timeout/403/5xx/malformed-date responses; confirm the specified client error, empty array, or upstream dependency error for each case.

### Tests for User Story 2

- [X] T016 [P] [US2] Define fixture-backed API error tests for malformed/unknown symbols, empty history, MOEX 403/5xx, timeout, malformed JSON, and missing trading dates in `tests/api_errors.rs`.

### Implementation for User Story 2

- [X] T017 [US2] Validate `SYMBOL` against the documented non-empty ticker character set in `src/moex/validation.rs` and return the shared client-error response from `src/http/routes.rs`.
- [X] T018 [US2] Distinguish a recognized ticker with no publicly available history from an unknown or unsupported ticker in `src/moex/client.rs` and `src/http/routes.rs`; return `[]` only for the valid empty-history case.
- [X] T019 [US2] Map MOEX HTTP errors (including denied access), network timeouts, malformed responses, and rows without trading dates to the documented upstream dependency error in `src/moex/client.rs` and `src/http/errors.rs`.

**Checkpoint**: Invalid input, valid empty history, and upstream failure have distinct, documented outcomes.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Package the service and align operator-facing guidance with the final behavior.

- [X] T020 Update the multi-stage Docker build to cross-compile from `BUILDPLATFORM` for `linux/amd64` and `linux/arm64`, use the matching target-platform `gcr.io/distroless/cc-debian13:nonroot` runtime, remove architecture-specific runtime library paths in `Dockerfile` and `.dockerignore`, use an exec-form entrypoint, and expose port 8080.
- [X] T021 Update `specs/001-moex-ticker-update/contracts/openapi.yaml` and `specs/001-moex-ticker-update/quickstart.md` to match final response/error behavior, unauthenticated history limits, per-trading-date board selection, and Docker run instructions.
- [ ] T022 (partial) Revalidate formatting, lint, acceptance tests, OpenAPI contract, latency target, and Docker image startup after cache and current board LOTSIZE support using `cargo fmt --check`, `cargo clippy`, `cargo test`, OpenAPI lint, and the documented scenarios in `specs/001-moex-ticker-update/quickstart.md`. For live data and latency acceptance, run the locally compiled application binary against production `https://iss.moex.com/iss`; measure complete response bodies, include cold misses and cache hits, and verify at least 95% of successful requests complete under one second under the representative normal workload. Build both Docker platforms and smoke-check their startup. Formatting, lint, tests, OpenAPI validation, production-data comparison, representative four-symbol p95 measurement, both image builds, and ARM64 smoke check passed; AMD64 runtime smoke check remains outstanding because this host is ARM64 without AMD64 emulation.

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No code dependencies; T001, T002, and T003 can proceed in parallel.
- **Foundational (Phase 2)**: Depends on setup. T004–T007 block both user stories.
- **User Story 1 (Phase 3)**: Depends on the foundation. Fixture and contract tests (T008–T010) can be authored in parallel; implementation follows the response/model contracts.
- **User Story 2 (Phase 4)**: Depends on User Story 1 because it extends the same ticker lookup route and distinguishes unknown symbols from valid empty history.
- **Polish (Phase 5)**: Depends on both user stories.

### User Story Dependencies

- **US1 (P1)**: Starts after Phase 2 and is independently deliverable as the MVP.
- **US2 (P2)**: Starts after US1; it reuses the route, upstream client, and response models while adding negative-path behavior.

### Parallel Opportunities

- Setup tasks T001–T003 touch separate design/configuration files and can run concurrently.
- US1 tests T008–T010 target separate fixture/test paths and can be authored concurrently after the foundational interfaces are agreed.
- US2 test authoring T016 can run independently of the three implementation changes T017–T019 once US1's route and shared error format exist.

## Parallel Example: User Story 1

```text
Task: T008 Create MOEX response fixtures in tests/fixtures/moex/
Task: T009 Define data mapping tests in tests/moex_mapping.rs
Task: T010 Define success contract tests in tests/api_contract.rs
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Setup and Foundational phases.
2. Implement US1 history retrieval, date-specific primary-board selection, and response mapping.
3. Validate US1 independently with fixtures and the API contract.
4. Deliver the P1 endpoint before adding US2's detailed error distinctions.

### Incremental Delivery

1. Complete Setup and Foundational phases.
2. Deliver US1 as the first usable endpoint.
3. Add US2 input validation and upstream failure behavior.
4. Package the full service and validate the quickstart and Docker image.

## Notes

- `[P]` marks tasks in separate files that can proceed without waiting on unfinished tasks.
- `[US1]` and `[US2]` map tasks to the user stories in `spec.md`.
- Every task names its target file path.
- The task list includes acceptance test work because the specification requires acceptance coverage.

## Phase 6: Convergence

- [X] T024 Add an end-to-end regression proving `GET /v1/moex/SBER` sources `facevalue` from the selected primary board's current `LOTSIZE` and returns `1` for 2026-10-06, correcting the current generic `FACEVALUE=3` mapping per FR-005 and SC-001 (contradicts).
- [X] T025 Add cache acceptance tests for hit, cold miss, expiry, concurrent request coalescing, and upstream failure behavior, then measure a local cache-heavy workload including a cold miss against the one-second p95 target per FR-012 and plan: cache decision (missing); the full representative normal-workload measurement remains part of T022.
- [ ] T026 (partial) Build and start the Docker image explicitly for `linux/amd64`, confirming the runtime uses the planned distroless image without an ARM64-only library path. Production ISS acceptance and response-body measurements have been completed with the locally compiled application binary; only native AMD64 runtime startup remains. The AMD64 image build and architecture metadata pass; runtime startup requires validation on an AMD64 host.
- [X] T027 Validate every source history row's trading date before filtering by board, and add a regression where a non-primary-board row has a missing or malformed date per FR-011 and T019 (partial).

## Phase 7: Convergence

- [X] T028 Run production-data acceptance checks with the locally compiled service binary against `https://iss.moex.com/iss`; verify SBER on 2026-10-06 has `facevalue: 1` and compare representative response records with production ISS per SC-001, SC-002, and US1/AC1 (partial). Verified primary board `TQBR`; direct ISS values matched local API values (`close=282.22`, `high=284`, `low=280.12`, `volume=22099060`, `LOTSIZE=1`).
- [X] T029 Measure the representative normal workload against production ISS using the locally compiled service binary, recording complete response-body timings for cache hits and misses; verify the p95 target and implement any needed performance correction per FR-012, SC-005, and plan: performance goals (partial). Four-symbol run: 100 total requests, 4 cold misses (12.858–22.412s), 96 hits (all <1s; hit p95 0.002747s); aggregate p95 is therefore <1s. Separate expiry run: 2 misses and 98 hits; aggregate p95 0.000493s.
- [X] T030 Bound upstream response bytes and accumulated history data in `src/moex/client.rs` while preserving all available unauthenticated history or returning the documented upstream error when configured limits are exceeded per plan: bounded response handling (partial). Added configurable 4 MiB per-response and 64 MiB aggregate history defaults, streamed body limits, and oversized-response/history tests.
- [ ] T031 Start the `linux/amd64` distroless image on an AMD64 host and verify its health endpoint per plan: target platform and T026 (partial).

## Phase 8: Convergence

- [X] T032 Run Semgrep source analysis and Trivy scans against the final built deliverable, resolve every Semgrep finding and every fixable High or Critical Trivy finding before declaring completion, and document scan results plus any unavailable scan or fix per Constitution V (missing, CRITICAL). Final scoped Semgrep scan: 57 rules across 32 source/build files, zero findings and zero errors; its two initial Dockerfile findings were fixed by explicitly setting UID/GID 65532. Trivy full image scans: each architecture had 23 MEDIUM and 8 LOW findings, with zero HIGH/CRITICAL findings; filesystem dependency scan had zero findings.
