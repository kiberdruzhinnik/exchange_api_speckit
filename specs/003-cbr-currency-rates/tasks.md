# Tasks: CBR Currency Rates API

**Input**: Design documents from `specs/003-cbr-currency-rates/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/openapi.yaml`, and `quickstart.md`.

**Tests**: Include automated tests required by the project constitution and feature acceptance requirements. Use recorded CBR XML fixtures and mocked HTTP for deterministic checks; use production CBR only for the final latency acceptance run.

**Organization**: Tasks are grouped by user story. Each story phase states its independent acceptance criteria.

## Phase 1: Setup

**Purpose**: Add the XML parsing capability required by the CBR source adapter.

- [X] T001 Add the `quick-xml` Serde-enabled dependency to `Cargo.toml` and update `Cargo.lock`.

---

## Phase 2: Foundational

**Purpose**: Build shared source, configuration, validation, and error handling required by all CBR routes.

- [X] T002 [P] Add `CBR_API_BASE_URL` (default `https://www.cbr.ru/`) and positive `CBR_MAX_RESPONSE_BYTES` configuration fields in `src/config.rs`.
- [X] T003 [P] Define CBR directory and rate XML models in `src/cbr/models.rs` and declare the CBR module in `src/cbr/mod.rs` and `src/lib.rs`.
- [X] T004 [P] Implement trimmed, uppercase, three-letter ISO symbol normalization and directory matching in `src/cbr/validation.rs`.
- [X] T005 Implement the bounded Rustls CBR XML client in `src/cbr/client.rs` to cache the supported-currency directory for 60 seconds, then refresh it; fetch history from the earliest source-available date through the latest published date and fetch the latest rate on demand; enforce `CBR_MAX_RESPONSE_BYTES` while reading bodies and retain normal TLS certificate and hostname verification.
- [X] T006 Add the CBR client to application state and construct it from configuration in `src/lib.rs` and `src/main.rs`.
- [X] T007 Add a CBR-specific upstream error variant and map CBR dependency failures to HTTP 502 with the standard `cbr_unavailable` JSON error in `src/http/errors.rs`.
- [X] T008 Add CBR XML fixtures for currency directory, non-unit nominal history/latest rate, empty results, and malformed source documents under `tests/fixtures/cbr/`.

**Checkpoint**: Shared CBR client and source parsing are ready for independent history and quote route work.

---

## Phase 3: User Story 1 - Retrieve a supported currency's rate history (Priority: P1) 🎯 MVP

**Goal**: Return all available official daily currency rates in ascending date order with normalized close values and the six-field response contract.

**Independent Test**: With fixture-backed CBR responses, request `/v1/cbr/USD`; verify the full sorted history, `close = Value / Nominal` (including 5 RUB per 100 units → `0.05`), `facevalue = Nominal`, and null `high`, `low`, and `volume`. Confirm successful empty history returns `[]` and successful results persist through a store reopen.

### Tests for User Story 1

- [X] T009 [P] [US1] Add mapping tests in `tests/cbr_mapping.rs` for oldest-to-newest dates, UTC midnight, the exact six fields, non-unit nominal normalization, `close: null` when `Value` or `Nominal` is absent, nullable optional fields, and empty history.
- [X] T010 [P] [US1] Add history route tests in `tests/api_cbr.rs` for supported and unsupported symbols, response JSON, cache reuse, restart persistence, and HTTP 503 when the history store fails.

### Implementation for User Story 1

- [X] T011 [US1] Implement CBR history validation and mapping in `src/cbr/mapping.rs`: `close` is RUB per one currency unit (`Value / Nominal`) or null if either source input is absent; `facevalue` is the source nominal or null, unavailable optional fields are null, and dates are UTC midnight.
- [X] T012 [US1] Add `GET /v1/cbr/{SYMBOL}` in `src/http/routes.rs`; validate against the CBR supported-currency directory, return all available records oldest to newest, cache only fully validated successful history under `CBR:{SYMBOL}`, reuse the durable history store, and return `[]` for valid empty history.
- [X] T013 [US1] Map history cache/store failures to HTTP 503 with `history_store_unavailable` in `src/http/routes.rs` and `src/http/errors.rs`, without changing existing MOEX or SPBEX behavior; implement this after the failure test in T010.

**Checkpoint**: User Story 1 works independently with complete history, normalized values, empty results, and persistent history.

---

## Phase 4: User Story 2 - Retrieve the current official currency rate (Priority: P2)

**Goal**: Return one fresh latest-rate record with the same six fields and effective date behavior as history.

**Independent Test**: Request `/v1/cbr/USD/quote` twice against a mock source that changes its response; verify each response is one record with the latest fetched rate and that both requests reach the source. Verify a successful source result with no rate returns one all-null record.

### Tests for User Story 2

- [X] T014 [P] [US2] Add quote mapping tests in `tests/cbr_mapping.rs` for normalized rates, nominal facevalue, no-rate all-null record, and latest effective date.
- [X] T015 [P] [US2] Add quote route tests in `tests/api_cbr.rs` proving `/v1/cbr/{SYMBOL}/quote` returns one six-field record and bypasses both memory and persistent quote caching.

### Implementation for User Story 2

- [X] T016 [US2] Implement latest CBR rate mapping in `src/cbr/mapping.rs` using the same field semantics as history and return `LatestTradeRecord::no_trade()` when the successful source lookup has no rate.
- [X] T017 [US2] Add `GET /v1/cbr/{SYMBOL}/quote` in `src/http/routes.rs`; fetch rate data on every request, return the latest official effective date and one-record array, and bypass both history cache layers.

**Checkpoint**: User Stories 1 and 2 work independently; quote requests always reach the upstream source.

---

## Phase 5: User Story 3 - Handle unsupported currencies and source failures (Priority: P2)

**Goal**: Make invalid input, valid empty data, source failures, and persistent store failures distinguishable for both routes.

**Independent Test**: Exercise malformed and unsupported symbols, malformed/oversized/unavailable CBR responses, empty history, empty quote data, and history-store failure; verify documented HTTP status and standard JSON error bodies for both endpoints.

### Tests for User Story 3

- [X] T018 [P] [US3] Add error contract cases in `tests/api_cbr.rs` for HTTP 400 `invalid_symbol`, HTTP 502 `cbr_unavailable`, and valid empty/nullable outcomes on both routes.
- [X] T019 [P] [US3] Add CBR client failure tests in `tests/cbr_client.rs` for transport/status failures, malformed XML, declared and streamed size limits, invalid rates, and unsupported CBR codes.

### Implementation for User Story 3

- [X] T020 [US3] Complete CBR client and route error classification in `src/cbr/client.rs`, `src/cbr/mapping.rs`, and `src/http/routes.rs`: malformed/unsupported symbols return HTTP 400, upstream transport/status/oversize/malformed or unusable data returns HTTP 502, and store failures return HTTP 503.
- [X] T021 [US3] Add structured CBR source and history-store diagnostics in `src/http/errors.rs` while keeping client-facing messages stable and actionable.

**Checkpoint**: All three stories return distinct, documented outcomes for invalid input, successful absence of data, upstream failure, and store failure.

---

## Phase 6: Polish & Cross-Cutting Acceptance

**Purpose**: Keep documentation aligned and complete the performance, build, and security gates.

- [X] T022 [P] Document both CBR endpoints, supported currency codes, six-field schema, null mapping, nominal normalization, effective dates, and errors in `specs/003-cbr-currency-rates/contracts/openapi.yaml`.
- [X] T023 [P] Document CBR configuration, local history/quote requests, persistence behavior, and error outcomes in `specs/003-cbr-currency-rates/quickstart.md`.
- [X] T024 [P] Add OpenAPI contract schema coverage for both CBR paths and six required response fields in `tests/api_contract_schema.rs`.
- [X] T025 Add a fixed-arrival production CBR latency harness in `scripts/measure-cbr-latency.py` and `scripts/measure-cbr-latency.sh` that reports issued/skipped/error counts, actual request rate, and successful-response p95 by route for history-only, quote-only, and combined profiles.
- [X] T026 Run the production latency acceptance using the optimized local binary and CBR source with 10 concurrent clients and 10 total requests per second; record valid results and optimize/repeat until every profile has successful-response p95 under one second in `specs/003-cbr-currency-rates/quickstart.md`.
- [X] T027 Build the container for `linux/amd64` using `Dockerfile` and verify the build succeeds; retain the existing `linux/arm64` build/runtime configuration without adding amd64 runtime tests.
- [X] T028 Run Semgrep on the final source and resolve every finding; record results in `specs/003-cbr-currency-rates/quickstart.md`.
- [X] T029 Run Trivy against the final built deliverable, resolve all fixable High and Critical findings, and record results or unavailable fixes in `specs/003-cbr-currency-rates/quickstart.md`.
- [X] T030 Run the `quickstart.md` local validation scenarios and record any checks that cannot run in the development environment in `specs/003-cbr-currency-rates/quickstart.md`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies; add the XML parser dependency first.
- **Foundational (Phase 2)**: Depends on Setup; blocks all user story implementation.
- **User Story 1 (Phase 3)**: Depends on Foundational; delivers history MVP.
- **User Story 2 (Phase 4)**: Depends on Foundational and may proceed independently of US1 once shared CBR mapping/client APIs are stable.
- **User Story 3 (Phase 5)**: Depends on both routes so error outcomes can be verified for history and quote.
- **Polish (Phase 6)**: Contract/docs can proceed after the schema is stable; production acceptance, amd64 build, and final scans require all stories complete.

### User Story Dependencies

- **US1 (P1)**: Can start after Phase 2; no dependency on other user stories.
- **US2 (P2)**: Can start after Phase 2; shares the CBR client and mapping module but not history route behavior.
- **US3 (P2)**: Requires US1 and US2 routes to verify the full invalid-input, upstream, and store error matrix.

### Parallel Opportunities

- T002, T003, and T004 can proceed in parallel because they modify separate configuration/model/validation files.
- T009 and T010 can proceed in parallel; they cover separate mapping and HTTP test files.
- T014 and T015 can proceed in parallel; quote mapping and route tests are in separate files.
- T018 and T019 can proceed in parallel; route errors and client transport failures use separate test files.
- T022, T023, and T024 can proceed in parallel after the contract schema is stable.
- US1 and US2 implementation can proceed in parallel after the shared foundation if the team coordinates edits to `src/cbr/mapping.rs` and `src/http/routes.rs`.

## Implementation Strategy

### MVP First (User Story 1)

1. Complete Setup and Foundational phases.
2. Complete US1 history mapping, route, persistence, and automated checks.
3. Validate full history ordering, normalized per-unit rates, null fields, empty history, and persistence independently.
4. Continue with US2 quote and US3 failure outcomes before release.

### Incremental Delivery

1. Shared CBR client and foundation.
2. US1 history route as the first demonstrable increment.
3. US2 fresh quote route.
4. US3 error distinctions across both routes.
5. Documentation, production p95 gate, amd64 build, and required security scans.

## Notes

- Every task uses the required checkbox, sequential ID, optional `[P]`, story label when applicable, and explicit file paths.
- The latency requirement is a hard completion gate; a run below the issue rate or with skipped arrivals is invalid.
- The amd64 requirement is build success only; amd64 runtime acceptance testing is not required.
- Final delivery is blocked until all Semgrep findings and all fixable High/Critical Trivy findings are resolved.

## Phase 7: Convergence

- [X] T031 Add an oversized chunked or unknown-length history response test in `tests/cbr_client.rs` and verify the streamed byte limit in `src/cbr/client.rs` rejects it per T019 and FR-013 (partial).
- [X] T032 Instrument the production latency run in `scripts/measure-cbr-latency.py` to verify and report history cold-source fetches, warm-cache hits, and expiry-triggered refreshes in `specs/003-cbr-currency-rates/quickstart.md` per plan: Performance Goals (partial).
- [X] T033 Run the shared SIGINT shutdown acceptance (`cargo test --test shutdown`) for CBR per FR-019 and SC-008, confirm new requests stop while in-flight work drains and the 30-second deadline cancels remaining work, and record the result in `specs/003-cbr-currency-rates/quickstart.md`.

## Phase 8: Convergence

- [X] T034 Run the existing SIGINT shutdown acceptance with `cargo test --test shutdown` and record the actual result in `specs/003-cbr-currency-rates/quickstart.md`, confirming request acceptance stops, in-flight work drains, and remaining work is canceled by the 30-second deadline per T033, FR-019, and SC-008 (partial).
