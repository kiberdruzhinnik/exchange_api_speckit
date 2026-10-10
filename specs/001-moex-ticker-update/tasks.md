# Tasks: MOEX Ticker History API

**Input**: Design documents from `specs/001-moex-ticker-update/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/openapi.yaml`

**Tests**: Fixture-backed API behavior, persistence, OpenAPI contract, and acceptance validation tasks are included because SC-003 and the project constitution require them.

**Organization**: Tasks are grouped by user story. Completed implementation and validation work retains its recorded status.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Tasks target separate files and can proceed without waiting on unfinished tasks.
- **[Story]**: Maps story tasks to `spec.md` user stories.
- Each task includes its target file path.

## Phase 1: Setup

**Purpose**: Add shared libraries needed by the Rust service and durable history cache.

- [X] T001 Add SQLx SQLite and `chrono-tz` dependencies with compatible features in `Cargo.toml` and update `Cargo.lock`.

## Phase 2: Foundational

**Purpose**: Establish configuration, shared response types, persistent-store startup, and application state required by all user stories.

- [X] T002 Add validated configuration for upstream timeout and response limits, history TTL and byte limits, and SQLite database path in `src/config.rs`.
- [X] T003 Define shared history and quote response record types, preserving all six fields and nullable quote `date`, in `src/domain.rs`.
- [X] T004 Implement SQLite schema initialization and indexed cache-entry lookup primitives for symbol, expiry, body, fetch time, and byte count in `src/cache_store.rs`.
- [X] T005 Initialize the durable history store and inject it with HTTP client and cache configuration into application state in `src/main.rs` and `src/lib.rs`.
- [X] T006 Map documented client, upstream dependency, and history-store errors to the shared JSON error envelope in `src/http/errors.rs`.
- [X] T007 Expose liveness and readiness routes, with readiness reflecting required startup dependencies, in `src/http/routes.rs` and `src/http/mod.rs`.

**Checkpoint**: Validated startup, shared state, operational routes, and standard error mapping are available to all user stories.

## Phase 3: User Story 1 - Retrieve ticker history and latest trade (Priority: P1) 🎯 MVP

**Goal**: Serve complete daily history from unauthenticated MOEX ISS and return an uncached latest-trade response in the specified six-field array shape.

**Independent Test**: Fixture-backed API checks verify ascending history rows from the board primary on each trading date, current selected-board LOTSIZE mapping, quote array shape and field mappings, empty history, and no-trade behavior.

### Tests for User Story 1

- [X] T008 [P] [US1] Add paginated history, board-assignment, current board LOTSIZE, nullable-field, and SBER 2026-10-06 fixtures in `tests/fixtures/moex/`.
- [X] T009 [P] [US1] Add mapping checks for named ISS columns, per-date primary-board selection, ascending dates, midnight-UTC history dates, null values, and LOTSIZE mapping in `tests/moex_mapping.rs`.
- [X] T010 [P] [US1] Add history API checks for required fields, empty-array behavior, SBER `facevalue: 1`, cache hits, and OpenAPI conformance in `tests/api_contract.rs` and `tests/api_contract_schema.rs`.
- [X] T011 [P] [US1] Add quote checks for a trade row and valid empty trades, asserting the one-element array and exact six-field mappings in `tests/api_quote.rs`.
- [X] T012 [P] [US1] Add durable-cache checks for expiry, complete-response replacement, capacity pruning, and reuse after reopening the SQLite store in `tests/cache_store.rs`.

### Implementation for User Story 1

- [X] T013 [P] [US1] Define the typed latest-trade model and named-column ISS tables for history, listing, and security-reference data in `src/moex/models.rs`.
- [X] T014 [US1] Implement bounded unauthenticated ISS requests, status handling, response-size limits, and cursor pagination with at most 128 parallel history pages per symbol in `src/moex/client.rs`.
- [X] T015 [US1] Resolve the board primary on each trading date and retrieve current LOTSIZE for each selected board in `src/moex/board.rs` and `src/moex/client.rs`.
- [X] T016 [US1] Map history rows by named field, preserve null market values, reject missing or invalid trading dates, map selected-board LOTSIZE to `facevalue`, and sort ascending in `src/moex/mapping.rs`.
- [X] T017 [US1] Implement complete-response SQLite cache reads and atomic writes with TTL checks, WAL configuration, expiry cleanup, and aggregate payload-byte pruning in `src/cache_store.rs`.
- [X] T018 [US1] Implement the bounded Moka hot cache and same-symbol miss coalescing, checking SQLite before ISS and caching only complete successful responses in `src/cache.rs`.
- [X] T019 [US1] Implement `GET /v1/moex/{SYMBOL}` with symbol resolution, history retrieval, cache orchestration, and the documented JSON array response in `src/http/routes.rs`.
- [X] T020 [US1] Fetch the latest executed trade on every request using the ISS trades resource and map Moscow-local execution time to UTC in `src/moex/client.rs` and `src/moex/mapping.rs`.
- [X] T021 [US1] Implement `GET /v1/moex/{SYMBOL}/quote` as a one-element six-field array, including all-null valid no-trade output, while bypassing both history caches in `src/http/routes.rs`.
- [X] T022 [US1] Align both route schemas and examples with the history and quote array contracts in `specs/001-moex-ticker-update/contracts/openapi.yaml`.
- [X] T023 [US1] Document both routes, examples, freshness behavior, cache configuration, no-trade behavior, errors, and local validation commands in `specs/001-moex-ticker-update/quickstart.md`.

**Checkpoint**: History and quote success paths meet their independent acceptance criteria; unexpired history survives store reopening and quote requests fetch fresh data.

## Phase 4: User Story 2 - Handle invalid or unavailable ticker data (Priority: P2)

**Goal**: Give clients distinct documented outcomes for invalid symbols, valid empty history, upstream failures, and history-store failures.

**Independent Test**: Exercise malformed and unknown symbols, valid empty history, ISS denial/timeout/5xx/malformed responses, unusable rows, quote failures, and store errors; verify the documented `400`, `502`, and `503` outcomes.

### Tests for User Story 2

- [X] T024 [P] [US2] Add API error checks for malformed and unsupported symbols, valid empty history, ISS failures, malformed responses, and persistent-store failures in `tests/api_errors.rs`.
- [X] T025 [P] [US2] Check that failed or malformed ISS trade responses produce the documented dependency error rather than an all-null quote in `tests/api_quote.rs`.

### Implementation for User Story 2

- [X] T026 [US2] Validate non-empty `SYMBOL` path values against the documented ticker character set before constructing ISS URLs in `src/moex/validation.rs` and `src/http/routes.rs`.
- [X] T027 [US2] Distinguish recognized tickers with no history from unknown or unsupported symbols, returning `[]` only for valid empty history in `src/moex/client.rs` and `src/http/routes.rs`.
- [X] T028 [US2] Convert ISS transport/status/parse failures and unusable history or quote data to the documented `502` envelope, and history-store failures to `503`, in `src/moex/client.rs`, `src/http/errors.rs`, and `src/http/routes.rs`.
- [X] T029 [US2] Document symbol validation and client, dependency, and store error responses in `specs/001-moex-ticker-update/contracts/openapi.yaml` and `specs/001-moex-ticker-update/quickstart.md`.

**Checkpoint**: Invalid input, valid empty data, upstream failures, and store failures are distinguishable and match the API contract.

## Phase 5: User Story 3 - Stop the service with Ctrl+C (Priority: P2)

**Goal**: On SIGINT, stop accepting new requests, drain in-flight work, and exit within 30 seconds, cancelling work still active at the deadline.

**Independent Test**: Start the local service with controlled in-flight requests, send SIGINT, verify new requests are no longer accepted, verify a request can complete during the drain window, and verify unfinished work is cancelled and the process exits by 30 seconds.

### Tests for User Story 3

- [X] T042 [US3] Add process-level SIGINT acceptance coverage for stopping new connections, draining a controlled in-flight request, and exiting within 30 seconds in `tests/shutdown.rs`.
- [X] T043 [US3] Add a controlled non-completing request case that verifies in-flight work is cancelled at the 30-second deadline in `tests/shutdown.rs`.

### Implementation for User Story 3

- [X] T044 [US3] Handle Tokio Ctrl+C and initiate Axum graceful shutdown, stopping acceptance of new connections and logging the shutdown signal in `src/main.rs`.
- [X] T045 [US3] Implement a bounded shutdown coordinator that waits for Axum graceful serving to finish and returns at the 30-second deadline so Tokio runtime teardown cancels remaining connection tasks in `src/shutdown.rs`; expose and wire it from `src/main.rs`.
- [X] T046 [US3] Document Ctrl+C behavior, the 30-second drain limit, and the shutdown acceptance procedure in `specs/001-moex-ticker-update/quickstart.md`.

**Checkpoint**: The process responds to SIGINT by refusing new work, allowing in-flight work to finish where possible, and cancelling unfinished work before exiting within 30 seconds.

---

## Phase 6: Polish and Cross-Cutting Acceptance

**Purpose**: Verify persistence, performance, container builds, documentation, and final security scans across all user stories.

- [X] T030 Add structured request-duration and history cache/store outcome logging in `src/http/routes.rs` and `src/main.rs`.
- [X] T031 Add a production ISS acceptance script using the local binary, 10 concurrent clients, and 10 total requests per second; measure complete-response p95 separately for history, quote, and combined workloads, including cache hits, cold misses, and expirations in `scripts/measure-moex-latency.sh` and `scripts/measure-moex-latency.py`.
- [X] T032 Run production ISS acceptance using the locally compiled binary, verify SBER 2026-10-06 `facevalue: 1`, record route measurements and upstream failures, and apply required performance corrections in `specs/001-moex-ticker-update/quickstart.md` and affected implementation files.
- [X] T033 Build Docker images for `linux/amd64` and supported `linux/arm64` with Buildx, the distroless Debian 13 nonroot runtime, and persistent-cache directory permissions in `Dockerfile` and `.dockerignore`.
- [X] T034 Validate `linux/arm64` container startup and health/readiness; validate AMD64 acceptance by successful `linux/amd64` image build only, recording results in `specs/001-moex-ticker-update/quickstart.md`.
- [X] T035 Validate restart persistence by populating history, restarting with the same configured cache store, and confirming unexpired data is reused while expired history refreshes in `tests/cache_store.rs` and `tests/api_contract.rs`.
- [X] T036 Run selected formatting, lint, unit/integration, and OpenAPI contract checks and record commands/results or environment limitations in `specs/001-moex-ticker-update/quickstart.md`.
- [X] T037 Run Semgrep against source/build files and Trivy against the final built deliverable; resolve all Semgrep findings and all fixable High/Critical Trivy findings and record scan results or limitations in `specs/001-moex-ticker-update/quickstart.md`.
- [X] T038 Constrain the load generator to ten client workers at 10 total requests per second and record production local-binary measurements in `scripts/measure-moex-latency.py` and `specs/001-moex-ticker-update/quickstart.md`.
- [X] T039 Add quote completion logs for success, invalid-symbol, upstream, and malformed-response outcomes, including duration and cache-bypass status, in `src/http/routes.rs`.
- [X] T040 Align `plan.md`, `research.md`, and `quickstart.md` with FR-015/SC-008 by removing AMD64 runtime-start and route-check requirements while retaining successful Linux AMD64 image-build verification in `specs/001-moex-ticker-update/plan.md`, `specs/001-moex-ticker-update/research.md`, and `specs/001-moex-ticker-update/quickstart.md`.

## Dependencies and Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: T001 has no code dependencies.
- **Foundational (Phase 2)**: T002–T007 depend on setup and block all user stories.
- **User Story 1 (Phase 3)**: Depends on Phase 2. Fixture and test tasks T008–T012 can proceed in parallel; implementation follows the shared contracts.
- **User Story 2 (Phase 4)**: Depends on the shared route and ISS client from US1.
- **User Story 3 (Phase 5)**: Depends on Phase 2 and the running service entry point from US1; shutdown behavior is independently testable.
- **Polish (Phase 6)**: Depends on desired stories being complete. T031 and container work can proceed independently; final checks and scans follow implementation and the final image build.
- **Convergence (Phase 7)**: T041 and T047 record or rerun acceptance work; T047 depends on US3 and Phase 6 completion.

### User Story Dependencies

- **US1 (P1)**: Depends on Setup and Foundational phases; delivers the history and quote endpoints as MVP.
- **US2 (P2)**: Depends on US1's shared route and ISS client; adds distinct invalid-input and failure behavior.
- **US3 (P2)**: Depends on the foundational Tokio/Axum application and is otherwise independent of the history and quote behaviors.

### Parallel Opportunities

- T008–T012 target separate fixture/test files and can be authored in parallel.
- T013, T017, and T020 target separate model, cache-store, and quote-mapping concerns, subject to shared client integration.
- T031 measurement tooling and T033 container build work can proceed independently after interfaces stabilize.
- T042 and T043 both target `tests/shutdown.rs` and should be authored together; implementation T044/T045 follows those acceptance cases.

## Parallel Example: User Story 1

```text
Task: T008 Add MOEX history, board-change, LOTSIZE, null, and SBER fixtures in tests/fixtures/moex/
Task: T009 Add named-column mapping checks in tests/moex_mapping.rs
Task: T010 Add history API contract checks in tests/api_contract.rs and tests/api_contract_schema.rs
Task: T011 Add quote contract checks in tests/api_quote.rs
Task: T012 Add durable-cache checks in tests/cache_store.rs
```

## Parallel Example: User Story 3

```text
Task: T042 Add SIGINT acceptance coverage for stopping acceptance and draining requests in tests/shutdown.rs
Task: T043 Add shutdown-deadline cancellation coverage in tests/shutdown.rs
```

## Implementation Strategy

### MVP First (User Story 1)

1. Complete Setup and Foundational phases.
2. Deliver history retrieval, per-date board selection, LOTSIZE mapping, and restart-safe caching.
3. Deliver the uncached latest-trade endpoint with the one-element six-field array shape.
4. Validate US1 against fixtures and the OpenAPI contract.

### Incremental Delivery

1. Complete Setup and Foundational phases.
2. Deliver US1 as the first usable API increment.
3. Add US2's documented validation and error distinctions.
4. Add US3's bounded graceful shutdown behavior.
5. Complete latency, restart, architecture build, and security-scan acceptance.

## Notes

- `[P]` marks work on separate files with no unfinished prerequisites.
- `[US1]`, `[US2]`, and `[US3]` map tasks to `spec.md` user stories.
- Task completion states reflect the implementation record; T040 aligns AMD64 acceptance documentation, T041 records bounded-scheduler behavior and actual production issue rates, and T042–T047 implement, document, and validate SIGINT shutdown.
- Each task line has a checkbox, sequential ID, applicable story label, imperative action, and target file path.

## Phase 7: Convergence

- [X] T041 Bound latency workload scheduling so slow requests cannot accumulate in an executor queue and create catch-up bursts, then rerun and record production local-binary history, quote, and combined profiles under a 10-client, 10-requests-per-second schedule in `scripts/measure-moex-latency.py` and `specs/001-moex-ticker-update/quickstart.md` per SC-005/T031/T038 (partial; history and combined runs were capacity-limited and actual rates are documented).
- [X] T047 Run formatting, lint, unit/integration and OpenAPI checks, verify the SIGINT drain and deadline scenarios, rebuild the Linux amd64 image, run Semgrep and Trivy on the final deliverable, resolve all Semgrep findings and fixable High/Critical Trivy findings, and record outcomes in `specs/001-moex-ticker-update/quickstart.md`.
