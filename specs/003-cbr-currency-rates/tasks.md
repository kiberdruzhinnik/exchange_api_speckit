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

---

## Phase 9: User Story 3 - Unify provider contracts and shared configuration (Priority: P2)

**Goal**: Make MOEX, SPBEX, and CBR use one provider-neutral history/quote interface, common response/error behavior, and the same application-wide configuration names while preserving each provider's quote semantics.

**Independent Test**: Run provider contract and API tests for MOEX, SPBEX, and CBR. Confirm routes trim and uppercase symbols; return the same six-field array shape; use HTTP 400 `invalid_symbol`, HTTP 502 with the established provider code (`moex_unavailable`, `spbex_unavailable`, or `cbr_unavailable`), and HTTP 503 `history_store_unavailable`; return `[]` for valid empty history and one all-null record when a successful quote has no data. Confirm each adapter retains its provider-specific quote meaning and all shared settings use canonical `EXCHANGE_API_*` names without legacy aliases.

### Tests for User Story 3

- [X] T035 [P] [US3] Add provider-interface contract tests in `tests/provider_contract.rs` covering normalized history and quote return types, common invalid-symbol/upstream categories, each provider's existing upstream code, and retained MOEX trade, SPBEX candle, and CBR official-rate quote meanings.
- [X] T036 [P] [US3] Add shared configuration tests in `src/config.rs` covering the five canonical `EXCHANGE_API_*` defaults and overrides, provider-specific URL/size settings, and rejection/absence of compatibility aliases for `LISTEN_ADDR` and old shared `MOEX_*` settings.
- [X] T037 [P] [US3] Update API error tests in `tests/api_errors.rs`, `tests/api_cbr.rs`, and `tests/api_spbex.rs` to assert common HTTP status/envelope behavior, shared `invalid_symbol` and `history_store_unavailable` codes, and the existing provider-specific 502 codes (`moex_unavailable`, `spbex_unavailable`, `cbr_unavailable`).
- [X] T038 [P] [US3] Extend `tests/api_contract_schema.rs` to resolve each feature contract's external path references and validate all six canonical routes, shared record schemas, nullable values, error envelope, and provider-specific upstream code examples.

### Implementation for User Story 3

- [X] T039 [US3] Rename `LatestTradeRecord` to `LatestQuoteRecord` in `src/domain.rs`, `src/moex/mapping.rs`, `src/spbex/mapping.rs`, `src/cbr/mapping.rs`, `src/http/routes.rs`, `tests/moex_mapping.rs`, `tests/spbex_mapping.rs`, `tests/cbr_mapping.rs`, `tests/api_quote.rs`, `tests/api_spbex.rs`, and `tests/api_cbr.rs` while keeping provider-specific quote mapping unchanged.
- [X] T040 [US3] Define `ExchangeProvider`, `ProviderError`, and provider-neutral history/quote signatures in `src/provider.rs`; distinguish invalid-symbol and upstream failures, and leave persistent-store failures to shared route/cache orchestration.
- [X] T041 [P] [US3] Implement the common provider interface for MOEX in `src/moex/provider.rs`, delegating source requests, board selection, and latest-trade mapping to existing MOEX modules.
- [X] T042 [P] [US3] Implement the common provider interface for SPBEX in `src/spbex/provider.rs`, delegating chart requests and latest-daily-candle mapping to existing SPBEX modules.
- [X] T043 [P] [US3] Implement the common provider interface for CBR in `src/cbr/provider.rs`, delegating supported-currency lookup, XML requests, and latest-official-rate mapping to existing CBR modules.
- [X] T044 [US3] Store the three provider implementations through one shared provider abstraction in `src/lib.rs` and construct them from `AppConfig` in `src/main.rs` without changing provider-specific source settings.
- [X] T045 [US3] Refactor `src/http/routes.rs` to use shared history and quote orchestration for all providers, trim and uppercase every path symbol before provider validation, and preserve provider-specific quote meanings and exchange-qualified history keys.
- [X] T046 [US3] Unify HTTP status and error-envelope mapping in `src/http/errors.rs` while preserving `moex_unavailable`, `spbex_unavailable`, and `cbr_unavailable` for their existing `/v1` routes and sharing `invalid_symbol` and `history_store_unavailable` codes.
- [X] T047 [US3] Read the shared listener, timeout, history-cache TTL, capacity, and database path from `EXCHANGE_API_LISTEN_ADDR`, `EXCHANGE_API_REQUEST_TIMEOUT_SECS`, `EXCHANGE_API_HISTORY_CACHE_TTL_SECS`, `EXCHANGE_API_HISTORY_CACHE_MAX_BYTES`, and `EXCHANGE_API_HISTORY_CACHE_DB_PATH` in `src/config.rs`; remove legacy-name lookups and keep source URL/response-limit variables provider-prefixed.
- [X] T048 [US3] Change process setup in `tests/shutdown.rs` and the root `docker-compose.yml` example to use the canonical listener and cache environment names.

### Contract and documentation for User Story 3

- [X] T049 [US3] Create `specs/contracts/openapi.yaml` as the canonical OpenAPI contract with all six provider routes, shared history/quote/error schemas, and provider-specific quote descriptions.
- [X] T050 [US3] Update `specs/001-moex-ticker-update/contracts/openapi.yaml`, `specs/002-spbex-ticker-update/contracts/openapi.yaml`, and `specs/003-cbr-currency-rates/contracts/openapi.yaml` to reference canonical route/schema definitions and document the shared envelope/statuses with preserved provider-specific upstream codes.
- [X] T051 [US3] Replace remaining legacy shared environment names and document the canonical names in `specs/001-moex-ticker-update/quickstart.md`, `specs/002-spbex-ticker-update/quickstart.md`, `specs/003-cbr-currency-rates/quickstart.md`, `specs/001-moex-ticker-update/plan.md`, `specs/002-spbex-ticker-update/plan.md`, `specs/003-cbr-currency-rates/plan.md`, `specs/001-moex-ticker-update/research.md`, `specs/002-spbex-ticker-update/research.md`, `specs/003-cbr-currency-rates/research.md`, and `docker-compose.yml`; retain provider-prefixed source URL and response-limit names.
- [X] T052 [US3] Align `specs/003-cbr-currency-rates/plan.md`, `specs/003-cbr-currency-rates/research.md`, and `specs/003-cbr-currency-rates/quickstart.md` with the planned common interface, neutral quote record, preserved provider error codes, canonical settings, and provider-specific quote meanings.

### Cross-provider acceptance for User Story 3

- [X] T053 [US3] Run fixture-backed and API contract suites for MOEX, SPBEX, and CBR, including symbol normalization, empty history, absent quote, upstream failure, and history-store failure cases; resolve regressions in the affected `src/` and `tests/` files.
- [X] T054 [US3] Run `scripts/measure-moex-latency.sh`, `scripts/measure-spbex-latency.sh`, and `scripts/measure-cbr-latency.sh` against production sources using the local optimized binary, 10 concurrent clients, and 10 total requests per second; record per-route successful-response p95 and optimize/repeat until every route meets the one-second target in `specs/001-moex-ticker-update/quickstart.md`, `specs/002-spbex-ticker-update/quickstart.md`, and `specs/003-cbr-currency-rates/quickstart.md`.
- [X] T055 [US3] Build the final Linux amd64 image successfully using `Dockerfile`, then run Semgrep and Trivy against the final source and built deliverable; resolve every Semgrep finding and every fixable High/Critical Trivy finding and record results in `specs/003-cbr-currency-rates/quickstart.md`.
- [X] T056 [US3] Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo test --test shutdown`, and the local scenarios in all three provider quickstarts; record any unavailable checks in the corresponding quickstarts.

**Checkpoint**: The three adapters satisfy one public and internal contract, canonical shared configuration is used throughout, and cross-provider latency, build, shutdown, and scan gates pass.

## Dependencies & Execution Order for Shared Provider Work

- T035-T038 define the expected interface, error, configuration, and schema behavior. T039-T048 implement it; T041-T043 can proceed in parallel after T039 defines the neutral quote type and T040 defines the interface. T044 depends on all three adapters, and T045 depends on T044.
- T049-T052 align published contracts and documentation after the shared response and configuration design is fixed.
- T053-T056 depend on all implementation and documentation tasks. Production latency, amd64 build, Semgrep, and Trivy are final acceptance gates.

### Parallel Opportunities

- T035-T038 can proceed in parallel because they cover separate test files or configuration unit tests.
- T041-T043 can proceed in parallel after T040 because each adapter has a separate provider module.
- T049 and T051 can proceed in parallel after shared codes and setting names are fixed.

### Parallel Execution Examples

```text
US1 history tests (already complete): tests/cbr_mapping.rs and tests/api_cbr.rs
US2 quote tests (already complete): tests/cbr_mapping.rs and tests/api_cbr.rs
US3 contract/config tests: T035 tests/provider_contract.rs; T036 src/config.rs; T037 API error tests; T038 tests/api_contract_schema.rs
US3 provider adapters after T039-T040: T041 src/moex/provider.rs; T042 src/spbex/provider.rs; T043 src/cbr/provider.rs
```

## Incremental Strategy for Remaining Work

1. Keep completed CBR history and quote behavior as the existing MVP baseline.
2. Define the neutral record and provider interface, then adapt all three sources behind it.
3. Centralize route, error, and configuration behavior and align the canonical contract and deployment documentation.
4. Re-run cross-provider functional, latency, amd64 build, SIGINT, Semgrep, and Trivy acceptance before completion.

**Remaining work**: T057 records the completed MOEX and SPBEX production history-cache lifecycle probes.

## Phase 10: Convergence

- [X] T057 Extend the MOEX and SPBEX production latency harnesses to report history cold-fetch, warm-cache-hit, and expiry-refresh lifecycle probes separately from fixed-arrival p95 profiles; record each state’s result with provider-specific and combined workload results in the corresponding quickstarts per plan Performance Goals (partial).
