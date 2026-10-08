# Tasks: SPBEX Ticker History and Quote API

**Input**: Design documents from `specs/002-spbex-ticker-update/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/openapi.yaml`, `quickstart.md`

**Tests**: Included because the design calls for fixture-backed API, cache, contract, restart, and performance validation.

## Phase 1: Setup

**Purpose**: Prepare the SPBEX adapter fixtures and configuration points.

- [X] T001 [P] Add representative, empty, and malformed SPBEX chart-feed fixtures in `tests/fixtures/spbex/history.json`, `tests/fixtures/spbex/empty.json`, and `tests/fixtures/spbex/malformed.json`
- [X] T002 Add SPBEX base URL and response byte limit settings with documented defaults in `src/config.rs`

## Phase 2: Foundational

**Purpose**: Build shared adapter, error, and route foundations required by all stories.

- [X] T003 [P] Define source candle and chart-feed response types in `src/spbex/models.rs`
- [X] T004 [P] Implement SPBEX symbol normalization and validation using trimmed uppercase ASCII and `[A-Za-z0-9._-]+` in `src/spbex/validation.rs`
- [X] T005 Implement bounded asynchronous chart-feed requests with daily resolution, configurable base URL, response size limit, status handling, and timeout in `src/spbex/client.rs`
- [X] T006 Implement candle validation and history/quote mapping, preserving UTC timestamps, requiring finite positive OHLC with `low <= close <= high`, rejecting duplicate timestamps, sorting history ascending, setting `volume: null` and `facevalue: 1`, in `src/spbex/mapping.rs`
- [X] T007 Expose the SPBEX adapter modules in `src/spbex/mod.rs` and add reusable client state and construction in `src/lib.rs` and `src/main.rs`
- [X] T008 Add distinct SPBEX dependency and history-store error mappings using the existing JSON error envelope in `src/http/errors.rs`
- [X] T009 Register `/v1/spbex/{symbol}` and `/v1/spbex/{symbol}/quote` routes and handler wiring in `src/http/routes.rs`

## Phase 3: User Story 1 - Retrieve an SPBEX ticker's daily history (Priority: P1) 🎯 MVP

**Goal**: Return normalized SPBEX daily history as validated six-field records, backed by the existing durable history cache.

**Independent Test**: Fixture-backed history requests verify mapping, UTC ascending order, empty history, cache behavior, and persistence after service restart.

### Tests for User Story 1

- [X] T010 [P] [US1] Test candle parsing, invalid values, duplicate timestamps, mapping, and ascending order in `tests/spbex_mapping.rs`
- [X] T011 [P] [US1] Test chart-feed request parameters, bounded body handling, successful empty feeds, upstream rejections, and malformed payloads in `tests/spbex_client.rs`
- [X] T012 [P] [US1] Test history response fields, empty arrays, symbol normalization, cache hit/miss, and history-store error behavior in `tests/api_spbex.rs`
- [X] T013 [P] [US1] Test SPBEX history schema and status codes against the OpenAPI contract in `tests/api_contract_schema.rs`

### Implementation for User Story 1

- [X] T014 [US1] Implement the history handler to normalize symbols, fetch on cache miss or expiry, map successful empty feeds to `[]`, and cache only complete validated responses under `SPBEX:{symbol}` using `src/http/routes.rs` and `src/cache.rs`
- [X] T015 [US1] Reuse durable `CacheStore` behavior for SPBEX history and return HTTP 503 with the standard history-store error on store read/write failures in `src/cache_store.rs` and `src/http/routes.rs`
- [X] T016 [US1] Document the history route, six-field record, empty response, and 400/502/503 errors in `specs/002-spbex-ticker-update/contracts/openapi.yaml` and `specs/002-spbex-ticker-update/quickstart.md`

**Checkpoint**: User Story 1 works independently with fresh history, cached history, empty data, and durable store failure behavior.

## Phase 4: User Story 2 - Retrieve the latest SPBEX quote (Priority: P2)

**Goal**: Return the newest available daily candle from a fresh chart-feed read on every quote request.

**Independent Test**: Route tests verify one-record shape, upstream re-fetch between calls, adaptive lookback for inactive symbols, and all-null market fields after a successful empty full-range search.

### Tests for User Story 2

- [X] T017 [P] [US2] Test latest-candle mapping, empty quote record, and adaptive lookback window expansion through Unix epoch in `tests/spbex_mapping.rs` and `tests/api_spbex.rs`
- [X] T018 [P] [US2] Test that each quote request performs fresh upstream reads and never reads or writes either history cache layer in `tests/api_spbex.rs`

### Implementation for User Story 2

- [X] T019 [US2] Implement uncached quote handling with 1-day initial lookback, doubling empty windows until a candle is found or Unix epoch is reached, limit concurrent chart-feed requests to eight, and select the newest candle in `src/http/routes.rs` and `src/spbex/client.rs`
- [X] T020 [US2] Map successful empty full-range quote results to one record with nullable market fields in `src/spbex/mapping.rs`
- [X] T021 [US2] Document quote freshness, adaptive lookback, one-record shape, and empty quote response in `specs/002-spbex-ticker-update/contracts/openapi.yaml` and `specs/002-spbex-ticker-update/quickstart.md`

**Checkpoint**: User Stories 1 and 2 return history and fresh quotes independently, including their distinct empty-data behavior.

## Phase 5: User Story 3 - Distinguish invalid symbols from SPBEX and store failures (Priority: P2)

**Goal**: Return the documented client, dependency, and history-store errors without conflating them with valid empty data.

**Independent Test**: API tests exercise malformed symbols, explicit upstream rejection, unsuccessful/malformed/oversized source responses, and history-store failure, checking 400, 502, and 503 respectively.

### Tests for User Story 3

- [X] T022 [P] [US3] Test standard JSON error envelope and distinct invalid-symbol, SPBEX-dependency, and history-store error codes in `tests/api_errors.rs` and `tests/api_spbex.rs`
- [X] T023 [P] [US3] Test malformed, oversized, unsuccessful, and unusable chart-feed responses map to dependency errors while a successful empty response remains valid in `tests/spbex_client.rs` and `tests/api_spbex.rs`

### Implementation for User Story 3

- [X] T024 [US3] Map malformed symbols and explicitly identified upstream symbol rejection to HTTP 400, other unusable upstream responses to HTTP 502, and history-store failures to HTTP 503; log normalized symbols, cache outcomes, statuses, and upstream/store failures in `src/http/errors.rs` and `src/http/routes.rs`
- [X] T025 [US3] Align OpenAPI error responses and examples for both routes, including the history-only 503 outcome, in `specs/002-spbex-ticker-update/contracts/openapi.yaml`

**Checkpoint**: Clients can distinguish malformed/rejected symbols, SPBEX failures, store failures, and valid empty market data.

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Validate persistence, documentation, performance, builds, and required scans.

- [X] T026 [P] Verify history survives application restart and expires according to the configured history TTL in `tests/cache_store.rs` and `tests/api_spbex.rs`
- [X] T027 [P] Update SPBEX implementation examples, environment configuration, local usage, persistence behavior, and validation steps in `specs/002-spbex-ticker-update/quickstart.md`
- [X] T028 Build the release binary and implement production latency measurement for history-only, quote-only, and combined profiles at 10 concurrent clients and 10 requests per second total in `scripts/measure-spbex-latency.sh`
- [X] T029 Optimize the request/source path and repeat valid production measurements until every route-specific and combined successful-response p95 is under one second; record issue rate, errors, p95, and each optimization in `specs/002-spbex-ticker-update/quickstart.md`
- [X] T030 Build the Docker image for `linux/amd64`, include the SPBEX-scoped Russian Trusted Root CA in the build, and confirm the existing `linux/arm64` build configuration remains available in `Dockerfile`, `src/spbex/client.rs`, and `certs/russian_trusted_root_ca.crt`
- [X] T031 Run `cargo fmt --check` and `cargo test --locked` to verify MOEX compatibility, then run Semgrep against source and Trivy against the built deliverable; resolve all Semgrep findings and all fixable High/Critical Trivy findings, recording outcomes in `specs/002-spbex-ticker-update/quickstart.md`

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on setup; blocks all user stories.
- **User Story 1 (Phase 3)**: Depends on foundational work; delivers the MVP history endpoint.
- **User Story 2 (Phase 4)**: Depends on foundational work; can proceed alongside User Story 1 after shared adapter contracts stabilize.
- **User Story 3 (Phase 5)**: Depends on foundational work and integrates with both route handlers.
- **Polish (Phase 6)**: Depends on all three user stories.

### User Story Dependencies

- **US1 (P1)**: Independent after Phase 2; recommended MVP.
- **US2 (P2)**: Independent after Phase 2, sharing the SPBEX client and mapping types.
- **US3 (P2)**: Uses both route paths and their shared error mapping; complete after US1 and US2.

### Parallel Opportunities

- T001 and T002 can proceed in parallel.
- T003 and T004 can proceed in parallel; T010–T013 can proceed in parallel after the foundational data contracts are defined.
- US1 and US2 tests or isolated mapping/documentation work can proceed in parallel after Phase 2.
- T026 and T027 can proceed in parallel after route behavior stabilizes.

## Parallel Example: User Story 1

```text
Task: T010 tests/spbex_mapping.rs
Task: T011 tests/spbex_client.rs
Task: T013 tests/api_contract_schema.rs
```

## Implementation Strategy

### MVP First (User Story 1)

1. Complete Setup and Foundational phases.
2. Implement and validate User Story 1 history behavior and durable cache reuse.
3. Confirm the history route’s independent acceptance criteria before proceeding.

### Incremental Delivery

1. Add User Story 2 fresh latest-candle quote behavior.
2. Add User Story 3 distinct input, upstream, and store error behavior.
3. Complete persistence restart validation, documentation, hard p95 acceptance, amd64 build, and final scans.

## Notes

- Every task uses a sequential ID; `[P]` marks independent work and `[US#]` maps story-specific work to `spec.md`.
- Quote responses must bypass Moka and SQLite even when history is cached.
- A latency run is invalid if it fails to sustain the required 10 requests per second or skips scheduled requests.
- amd64 acceptance requires a successful build only; amd64 runtime testing is not required.


## Phase 7: Convergence

- [X] T032 Exclude candles dated on the current calendar date from `/v1/spbex/{SYMBOL}` before returning, caching, or persisting history, while preserving current-date candles on `/quote`, per FR-016 (contradicts)
- [X] T033 Add regression tests proving current-date candles are excluded from history, including cache-hit behavior, and remain available through quote, per US1/AC4 and FR-016 (missing)
- [X] T034 Align the plan, OpenAPI contract, and quickstart with the rule that current-date candles are reserved for quote and excluded from history, per FR-016 and Constitution II (contradicts)
