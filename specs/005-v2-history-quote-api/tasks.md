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

- [ ] T001 [P] Add the provider enum, v2 path templates, response and error schemas, and retained v1 routes to `specs/005-v2-history-quote-api/contracts/openapi-v2.yaml` and `specs/contracts/openapi.yaml`.
- [ ] T002 [P] Map unknown providers to HTTP 400 with the shared `invalid_provider` error envelope in `src/http/errors.rs`.
- [ ] T003 Add provider selection for exactly `moex`, `spbex`, and `cbr` in `src/http/routes.rs`, using the error from T002 for unsupported values.

**Checkpoint**: Both user stories can use the shared provider selection and public error contract.

## Phase 3: User Story 1 - Fetch symbol history from v2 (Priority: P1)

**Goal**: Return the selected provider's complete retained history on the provider-qualified v2 route, including the established empty, validation, upstream, and store outcomes.

**Independent Test**: For each provider, request `/v2/history/{PROVIDER}/{SYMBOL}` and verify ordered six-field records or `[]` for valid empty history; verify invalid inputs and source/store failures, and confirm the v1 history routes remain available.

### Tests for User Story 1

- [ ] T004 [US1] Add v2 history provider-dispatch, empty-history (`[]`), invalid-input, upstream/store error, and v1 compatibility tests in `tests/api_contract.rs`, `tests/api_spbex.rs`, `tests/api_cbr.rs`, and `tests/api_errors.rs`.
- [ ] T005 [P] [US1] Assert both v2 paths, provider values, response schemas, errors, and retained v1 paths in `tests/api_contract_schema.rs`.

### Implementation for User Story 1

- [ ] T006 [US1] Register `GET /v2/history/{provider}/{symbol}` in `src/http/routes.rs`, dispatch through the selected adapter, and reuse `serve_history` for normalization, complete-history assembly, persistence, and established error behavior.

**Checkpoint**: V2 history works for MOEX, SPBEX, and CBR without removing existing v1 history routes.

## Phase 4: User Story 2 - Fetch the current quote from v2 (Priority: P1)

**Goal**: Return the selected provider's existing fresh quote representation through the provider-qualified v2 route.

**Independent Test**: For each provider, request `/v2/quote/{PROVIDER}/{SYMBOL}` and verify provider dispatch, the one-record six-field response, no-data and source-error outcomes, and v1 quote compatibility.

### Tests for User Story 2

- [ ] T007 [US2] Add v2 quote dispatch, response shape, freshness, no-quote, error, and v1 compatibility checks in `tests/api_quote.rs`, `tests/api_spbex.rs`, and `tests/api_cbr.rs`.

### Implementation for User Story 2

- [ ] T008 [US2] Register `GET /v2/quote/{provider}/{symbol}` in `src/http/routes.rs`, dispatch through the selected adapter, and reuse `serve_quote` to preserve provider-specific quote meaning and error behavior.

**Checkpoint**: V2 quotes work for all three providers and all existing v1 quote routes remain available.

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Meet and document the six-route performance acceptance criterion, including MOEX's separate cold-fetch measurement and warm-cache gate, then complete release validation.

- [ ] T009 Run the six separate v2 profiles with 10 concurrent clients and 10 total requests per second using `scripts/measure-v2-latency.py`; for MOEX history first report the initial uncached full-fetch HTTP status and elapsed time separately, then run its gated profile after a successful full-history cache population; record p95, under-one-second success rate, errors, and missed slots for every profile in `specs/005-v2-history-quote-api/quickstart.md`.
- [ ] T010 If a populated-cache history profile fails SC-005, inspect and resolve the request-latency or arrival-rate bottleneck in `src/http/routes.rs`, `src/cache.rs`, `src/cache_store.rs`, `src/provider.rs`, `src/moex/provider.rs`, `src/moex/client.rs`, `src/spbex/provider.rs`, `src/spbex/client.rs`, `src/cbr/provider.rs`, or `src/cbr/client.rs`; rerun the affected profile and record results in `specs/005-v2-history-quote-api/quickstart.md` without weakening SC-005.
- [ ] T011 Update `specs/005-v2-history-quote-api/quickstart.md` with verified v2 examples, invalid-provider behavior, v1 compatibility, the separate MOEX cold-fetch procedure, and cache-populated acceptance results.
- [ ] T012 Build the Linux amd64 image from `Dockerfile`, run Semgrep against the source and Trivy against the built image, resolve required findings, and record commands and results in `specs/005-v2-history-quote-api/quickstart.md`.

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No initialization tasks are needed.
- **Foundational (Phase 2)**: T001 and T002 can run in parallel; T003 follows T002.
- **User Story 1 (Phase 3)**: T004 and T005 follow provider/error definitions; T006 follows T003 and the US1 checks.
- **User Story 2 (Phase 4)**: T007 follows the shared provider definitions; T008 follows T003 and T006 because both route registrations edit `src/http/routes.rs`.
- **Polish (Phase 5)**: T009 follows T006 and T008. T010 is conditional on a measured SC-005 miss. T012 follows the performance work so its quickstart update does not conflict with T009/T010; T011 consolidates performance and scan results after T009/T010/T012.

### User Story Dependencies

- **User Story 1 (P1)**: Depends on the shared provider selector and error mapping. It is the MVP and can be validated independently of quote routing.
- **User Story 2 (P1)**: Depends on the same shared selector and error mapping. Its route registration follows US1 because both edit `src/http/routes.rs`.

### Parallel Opportunities

- T001 and T002 modify separate contract and error files.
- After T003, T004 and T005 can run in parallel because they edit separate test files.
- T007 follows T004 because both tasks edit `tests/api_spbex.rs` and `tests/api_cbr.rs`; T005 can run alongside either task because it edits only `tests/api_contract_schema.rs`.
- Performance measurement and build/security validation both update the quickstart, so T012 follows T009/T010 and T011 consolidates the final results afterward.

## Parallel Example: User Story 1 and User Story 2

```text
After T001-T003 are complete, work in parallel:
Task: "T004 Add v2 history behavior tests in tests/api_contract.rs, tests/api_spbex.rs, tests/api_cbr.rs, and tests/api_errors.rs"
Task: "T005 Add v2 OpenAPI schema assertions in tests/api_contract_schema.rs"

After T004 completes, run T007 (which shares tests/api_spbex.rs and tests/api_cbr.rs with T004); T005 may proceed alongside it because it uses a separate file.
Task: "T007 Add v2 quote behavior tests in tests/api_quote.rs, tests/api_spbex.rs, and tests/api_cbr.rs"

Then register route handlers sequentially in src/http/routes.rs:
Task: "T006 Register the v2 history handler"
Task: "T008 Register the v2 quote handler"
```

## Implementation Strategy

### MVP First (User Story 1)

1. Complete shared contract, provider selection, and invalid-provider error handling.
2. Complete T004-T006 for v2 history, including the empty-history result and v1 compatibility.
3. Validate v2 history independently for all three providers.

### Incremental Delivery

1. Deliver v2 history as the MVP.
2. Add v2 quotes using the shared provider selector and quote-serving function.
3. Run separate route performance profiles; report the MOEX cold full fetch separately and gate its cache-populated profile against SC-005.
4. Resolve any measured cache-populated history performance misses, then complete build and source/image security validation.
5. Record verified outcomes in the feature quickstart.

## Notes

- Task IDs are sequential and every task uses the required checkbox, ID, and file-path format.
- `[P]` is used only when tasks touch separate files and have no incomplete dependency.
- Tests are included because the spec explicitly requires API tests for empty history and the plan calls for route and OpenAPI contract coverage.
- No storage migration, new dependency, or configuration change is required.
