# Tasks: V2 History and Quote Routes

**Input**: Design documents from `specs/005-v2-history-quote-api/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/openapi-v2.yaml`, `quickstart.md`

**Tests**: Route and contract tests are explicitly required by the specification and plan.

**Organization**: Tasks are grouped by user story so each route capability can be implemented and validated independently.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel because files do not conflict and prerequisites are complete.
- **[Story]**: Maps a user story task to `spec.md`.
- Every task names the exact files to change or the validation artifact to update.

## Path Conventions

- Single Rust service: `src/`, `tests/`, `scripts/`, and `specs/` from the repository root.

## Phase 1: Setup

**Purpose**: No setup is required; the Rust service, provider adapters, router, test harness, and API contracts already exist.

## Phase 2: Foundational

**Purpose**: Define the public route contract and provider selection shared by both user stories.

- [X] T001 [P] Add the provider enum, v2 path templates, response and error schemas, and retained v1 routes to `specs/005-v2-history-quote-api/contracts/openapi-v2.yaml` and `specs/contracts/openapi.yaml`.
- [X] T002 [P] Map unknown providers to HTTP 400 with the shared `invalid_provider` error envelope in `src/http/errors.rs`.
- [X] T003 Add provider selection for exactly `moex`, `spbex`, and `cbr` in `src/http/routes.rs`, using the error from T002 for unsupported values.

**Checkpoint**: Both user stories can use the shared provider selection and public error contract.

## Phase 3: User Story 1 - Fetch symbol history from v2 (Priority: P1)

**Goal**: Return the selected provider's complete retained history on the provider-qualified v2 route, including the established empty, validation, upstream, and store outcomes.

**Independent Test**: Build the history API test app from default configuration with both refresh settings absent; for each provider, verify `/v2/history/{PROVIDER}/{SYMBOL}` returns ordered six-field records or `[]` for valid empty history, and verify invalid inputs, source/store failures, and v1 compatibility.

### Tests for User Story 1

- [X] T004 [US1] Add v2 history provider-dispatch, empty-history (`[]`), invalid-input, upstream/store error, and v1 compatibility tests in `tests/api_contract.rs`, `tests/api_spbex.rs`, `tests/api_cbr.rs`, and `tests/api_errors.rs`.
- [X] T005 [P] [US1] Assert both v2 paths, provider values, response schemas, errors, and retained v1 paths in `tests/api_contract_schema.rs`.

- [X] T006 [US1] Start the real service with both refresh settings absent and verify a valid fixture-backed v2 history request returns HTTP 200 in `tests/api_default_config.rs`; verify the default values in `src/config.rs`.

### Implementation for User Story 1

- [X] T007 [US1] Register `GET /v2/history/{provider}/{symbol}` in `src/http/routes.rs`, dispatch through the selected adapter, and reuse `serve_history` for normalization, complete-history assembly, persistence, and established error behavior.

**Checkpoint**: V2 history works for MOEX, SPBEX, and CBR without removing existing v1 history routes.

## Phase 4: User Story 2 - Fetch the current quote from v2 (Priority: P1)

**Goal**: Return the selected provider's existing fresh quote representation through the provider-qualified v2 route.

**Independent Test**: Build the quote API test app from default configuration with both refresh settings absent; for each provider, verify `/v2/quote/{PROVIDER}/{SYMBOL}` returns the one-record six-field response, and verify provider dispatch, no-data and source-error outcomes, and v1 compatibility.

### Tests for User Story 2

- [X] T008 [US2] Add v2 quote dispatch, response shape, freshness, no-quote, error, and v1 compatibility checks in `tests/api_quote.rs`, `tests/api_spbex.rs`, and `tests/api_cbr.rs`.
- [X] T009 [US2] Extend the unset-settings subprocess regression in `tests/api_default_config.rs` to verify a valid fixture-backed v2 quote request returns HTTP 200.

### Implementation for User Story 2

- [X] T010 [US2] Register `GET /v2/quote/{provider}/{symbol}` in `src/http/routes.rs`, dispatch through the selected adapter, and reuse `serve_quote` to preserve provider-specific quote meaning and error behavior.

**Checkpoint**: V2 quotes work for all three providers and all existing v1 quote routes remain available.

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Meet and document the six-route performance acceptance criterion, including MOEX's separate cold-fetch measurement and warm-cache gate, then complete release validation.

- [X] T011 Run the six separate v2 profiles with 10 concurrent clients and 10 total requests per second using `scripts/measure-v2-latency.py`; for MOEX history first report the initial uncached full-fetch HTTP status and elapsed time separately, then run its gated profile after a successful full-history cache population; record p95, under-one-second success rate, errors, and missed slots for every profile in `specs/005-v2-history-quote-api/quickstart.md`.
- [X] T012 If a populated-cache history profile fails SC-005, inspect and resolve the request-latency or arrival-rate bottleneck in `src/http/routes.rs`, `src/cache.rs`, `src/cache_store.rs`, `src/provider.rs`, `src/moex/provider.rs`, `src/moex/client.rs`, `src/spbex/provider.rs`, `src/spbex/client.rs`, `src/cbr/provider.rs`, or `src/cbr/client.rs`; rerun the affected profile and record results in `specs/005-v2-history-quote-api/quickstart.md` without weakening SC-005.
- [X] T013 Update `specs/005-v2-history-quote-api/quickstart.md` with verified v2 examples, invalid-provider behavior, v1 compatibility, the separate MOEX cold-fetch procedure, and cache-populated acceptance results.
- [X] T014 Build the Linux amd64 image from `Dockerfile`, run Semgrep against the source and Trivy against the built image, resolve required findings, and record commands and results in `specs/005-v2-history-quote-api/quickstart.md`.
- [X] T015 Document startup and v2 route checks with both refresh settings omitted in `specs/005-v2-history-quote-api/quickstart.md`, including the seven-day and 900-second defaults.

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No initialization tasks are needed.
- **Foundational (Phase 2)**: T001 and T002 can run in parallel; T003 follows T002.
- **User Story 1 (Phase 3)**: T004, T005, and T006 follow the provider/error definitions and can run in parallel because they edit separate files; T007 follows T003 and T006.
- **User Story 2 (Phase 4)**: T008 follows the shared provider definitions; T009 follows T006 because both edit `tests/api_default_config.rs`; T010 follows T003 and T007 because both route registrations edit `src/http/routes.rs`, and follows T008 and T009 so quote behavior tests precede route implementation.
- **Polish (Phase 5)**: T011 follows both route stories. T012 is conditional on a measured SC-005 miss. T013 records performance results after T011/T012; T014 performs build and security scans after the performance work and records results in the same quickstart; T015 updates that quickstart after T014.

### User Story Dependencies

- **User Story 1 (P1)**: Depends on the shared provider selector and error mapping. It is the MVP and can be validated independently of quote routing. T006 verifies history availability when refresh settings are unset.
- **User Story 2 (P1)**: Depends on the same shared selector and error mapping. Its route registration follows US1 because both edit `src/http/routes.rs`. T009 verifies quote availability when refresh settings are unset.

### Parallel Opportunities

- T001 and T002 modify separate contract and error files.
- After T003, T004 and T005 can run in parallel because they edit separate test files.
- T008 follows T004 because both tasks edit `tests/api_spbex.rs` and `tests/api_cbr.rs`; T005 can run alongside either task because it edits only `tests/api_contract_schema.rs`.
- T004, T005, and T006 can run in parallel after T003; T009 follows T006 because both edit `tests/api_default_config.rs` and can run alongside T008, which edits other test files. Performance measurement and release scans update the same quickstart sequentially: T011/T012, then T013, T014, and T015.

## Parallel Example: User Story 1 and User Story 2

```text
After T001-T003 are complete, work in parallel:
Task: "T004 Add v2 history behavior tests in tests/api_contract.rs, tests/api_spbex.rs, tests/api_cbr.rs, and tests/api_errors.rs"
Task: "T005 Add v2 OpenAPI schema assertions in tests/api_contract_schema.rs"

After T004 completes, run T008 (which shares tests/api_spbex.rs and tests/api_cbr.rs with T004); T005 may proceed alongside it because it uses a separate file.
Task: "T008 Add v2 quote behavior tests in tests/api_quote.rs, tests/api_spbex.rs, and tests/api_cbr.rs"

Then register route handlers sequentially in src/http/routes.rs:
Task: "T007 Register the v2 history handler"
Task: "T010 Register the v2 quote handler"
```

## Implementation Strategy

### MVP First (User Story 1)

1. Complete shared contract, provider selection, and invalid-provider error handling.
2. Complete T004-T007 for v2 history, including the empty-history result and v1 compatibility.
3. Validate v2 history independently for all three providers.

### Incremental Delivery

1. Deliver v2 history as the MVP.
2. Add v2 quotes using the shared provider selector and quote-serving function, including the default-configuration regression in T009.
3. Run separate route performance profiles; report the MOEX cold full fetch separately and gate its cache-populated profile against SC-005.
4. Resolve any measured cache-populated history performance misses, then complete build and source/image security validation.
5. Record verified outcomes in the feature quickstart, including route availability when the refresh settings are omitted (T013-T015).

## Notes

- Task IDs are sequential and every task uses the required checkbox, ID, and file-path format.
- `[P]` is used only when tasks touch separate files and have no incomplete dependency.
- Tests are included because the spec explicitly requires API tests for empty history and the plan calls for route and OpenAPI contract coverage.
- No storage migration, new dependency, or configuration change is required.
- T006 and T009 start the actual service with both refresh settings absent and verify route availability; T015 documents the same startup case and defaults.

## Phase 6: Convergence

- [X] T016 Extend `tests/api_default_config.rs` to start the real service with both refresh settings unset and fixture-backed MOEX, SPBEX, and CBR sources, then verify all six v2 provider history and quote routes return their documented successful responses per SC-006.
