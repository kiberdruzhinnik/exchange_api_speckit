# Tasks: V2 History and Quote Routes

**Input**: Design documents from `specs/005-v2-history-quote-api/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/openapi-v2.yaml`, `quickstart.md`

**Tests**: Include route and contract checks called for by the feature's independent test criteria and implementation plan.

**Organization**: Tasks are grouped by user story. The existing project and service infrastructure require no initialization or new dependencies.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel because files do not conflict and prerequisites are complete.
- **[Story]**: Maps a task to a user story from `spec.md`.
- Every task names the exact files to change.

## Path Conventions

- Single Rust service: `src/`, `tests/`, and `specs/` from the repository root.

## Phase 1: Setup

**Purpose**: No setup tasks are needed; the Rust service, router, provider adapters, test harness, and canonical OpenAPI document already exist.

## Phase 2: Foundational

**Purpose**: Publish shared v2 contract definitions and implement provider selection before story-specific handlers.

- [ ] T001 [P] Add provider parameter, v2 path templates, response schemas, provider error examples, and v1 compatibility to `specs/005-v2-history-quote-api/contracts/openapi-v2.yaml` and `specs/contracts/openapi.yaml`.
- [ ] T002 Add `InvalidProvider` as an HTTP 400 error with the `invalid_provider` envelope code in `src/http/errors.rs`.
- [ ] T003 Add a provider selector in `src/http/routes.rs` that maps exactly `moex`, `spbex`, and `cbr` to the existing provider adapters and returns `InvalidProvider` for other values.

**Checkpoint**: Shared provider selection and error behavior are available for both v2 routes; all existing v1 routes remain registered.

## Phase 3: User Story 1 - Fetch symbol history from v2 (Priority: P1) 🎯 MVP

**Goal**: Return complete retained history through the provider-qualified v2 history path without changing v1 behavior.

**Independent Test**: Request history for each supported provider through `/v2/history/{PROVIDER}/{SYMBOL}` and verify provider dispatch, the complete six-field ordered response, established error behavior, and continued v1 history access.

### Tests for User Story 1

- [ ] T004 [P] [US1] Add v2 history route and provider-dispatch coverage in `tests/api_contract.rs`, including successful empty history (`[]`), invalid provider/symbol, upstream/store errors, and v1 history compatibility.
- [ ] T005 [P] [US1] Add OpenAPI assertions for both v2 path templates, provider enum, schemas, errors, and retained v1 paths in `tests/api_contract_schema.rs`.

### Implementation for User Story 1

- [ ] T006 [US1] Register `GET /v2/history/{provider}/{symbol}` in `src/http/routes.rs`, dispatch to the selected adapter, and reuse `serve_history` so normalization, complete-history assembly, persistence, and provider errors match v1.

**Checkpoint**: The v2 history route works for MOEX, SPBEX, and CBR, and all existing v1 history routes remain available.

## Phase 4: User Story 2 - Fetch the current quote from v2 (Priority: P1)

**Goal**: Return the selected provider's existing fresh one-record quote through the provider-qualified v2 quote path.

**Independent Test**: Request quotes for supported provider-symbol pairs through `/v2/quote/{PROVIDER}/{SYMBOL}` and verify source selection, provider-specific quote meaning, one-record response shape, no-data and failure outcomes, and continued v1 quote access.

### Tests for User Story 2

- [ ] T007 [P] [US2] Add v2 quote dispatch, response-shape, freshness, no-quote, error, and v1 compatibility checks in `tests/api_quote.rs`.

### Implementation for User Story 2

- [ ] T008 [US2] Register `GET /v2/quote/{provider}/{symbol}` in `src/http/routes.rs`, dispatch to the selected adapter, and reuse `serve_quote` to preserve each provider's current quote behavior and error mapping.

**Checkpoint**: The v2 quote route returns MOEX trades, SPBEX candles, or CBR rates as selected by the path, while all v1 quote routes remain available.

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Verify the agreed performance target and record implementation validation for operators and reviewers.

- [ ] T009 Run the v2 acceptance workload described in `specs/005-v2-history-quote-api/quickstart.md` in six separate profiles, one per provider history and quote route, each with 10 concurrent clients and 10 total requests per second; confirm at least 95% of successful responses in each profile are under one second and capture per-route p95 and error rates.
- [ ] T010 Build the Linux amd64 deliverable from `Dockerfile`, run the required Semgrep source and Trivy image scans, resolve required findings, and capture commands and results for the release record in `specs/005-v2-history-quote-api/quickstart.md`.
- [ ] T011 Update `specs/005-v2-history-quote-api/quickstart.md` with verified v2 examples, invalid-provider behavior, confirmed v1 compatibility outcomes, and the completed performance and release-scan results.

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No initialization required.
- **Foundational (Phase 2)**: T001 and T002 can start in parallel. T003 follows T002.
- **User Story 1 (Phase 3)**: T004 and T005 follow the foundational contract and provider selector; T006 follows T003 and the US1 route checks.
- **User Story 2 (Phase 4)**: T007 follows the foundational provider selector; T008 follows T003 and T006 because both handler registrations modify `src/http/routes.rs`.
- **Polish (Phase 5)**: T009 and T010 follow T006 and T008 and can run independently. T011 follows T009 and T010 to record both completed results in the quickstart.

### User Story Dependencies

- **User Story 1 (P1)**: Depends on shared provider selection and error mapping. It is the MVP and can be validated independently of the v2 quote handler.
- **User Story 2 (P1)**: Depends on the same shared provider selection and error mapping. Its route-handler edit follows US1's edit because both modify `src/http/routes.rs`; its behavior does not depend on history results.

### Parallel Opportunities

- T001 (OpenAPI contract definitions) and T002 (shared error type) touch separate files and can run in parallel.
- After T001-T003 are complete, T004, T005, and T007 can be prepared concurrently because they use separate test files.
- Once the route implementation is complete, performance acceptance (T009) and the build/security scans (T010) can run independently; T011 records their results after both finish.

## Parallel Example: User Story 1 and User Story 2

```text
After T001-T003 are complete, work in parallel:
Task: "T004 Add v2 history and v1 compatibility checks in tests/api_contract.rs"
Task: "T005 Add v2 route schema checks in tests/api_contract_schema.rs"
Task: "T007 Add v2 quote dispatch and compatibility checks in tests/api_quote.rs"

Then implement handlers sequentially in src/http/routes.rs:
Task: "T006 Register the v2 history handler"
Task: "T008 Register the v2 quote handler"
```

## Implementation Strategy

### MVP First (User Story 1)

1. Complete the shared OpenAPI definitions, provider selection, and invalid-provider error handling.
2. Complete T004-T006 for v2 history.
3. Validate the history route for all three providers and confirm v1 history routes still work.

### Incremental Delivery

1. Deliver v2 history as the MVP.
2. Add v2 quotes using the shared provider selector and quote-serving function.
3. Run the six-route v2 performance acceptance workload and required release scans.
4. Record results and verified v1/v2 examples in the quickstart.

## Notes

- Task IDs are sequential and every task uses the required checkbox, ID, and file-path format.
- `[P]` is used only where tasks touch separate files and have no incomplete dependency.
- No storage migration, new dependency, or configuration change is required.
- Test execution and release scans are implementation tasks; they were not run during task generation.
