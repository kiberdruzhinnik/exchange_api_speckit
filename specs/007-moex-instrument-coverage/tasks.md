# Tasks: MOEX Benchmark and Currency Instrument Support

**Input**: Design documents from `specs/007-moex-instrument-coverage/`

**Prerequisites**: `spec.md`, `plan.md`, `research.md`, `data-model.md`, `contracts/`, and `quickstart.md`

**Tests**: Fixture-backed Rust and SQLite/API tests are required by the project constitution for important API behavior and service boundaries.

**Organization**: Tasks are grouped by user story. Shared provider preflight, market resolution, and validated per-symbol history migration are foundational prerequisites.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Task can run in parallel with marked tasks because it changes a different file and has no unmet dependency.
- **[Story]**: User story label maps to `spec.md`.
- Every task names the file path it changes or validates.

## Phase 1: Setup

**Purpose**: The existing Rust service and dependencies already provide the required project structure. No initialization task is needed.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Validate MOEX recognition before cache migration, retain resolved context for history retrieval, and build shared market-aware requests.

- [X] T001 Add SQLite migration tests in `tests/cache_store.rs` for an exact `MOEX:{SYMBOL}` cache key, an existing collection, repeated import, malformed cached data, and preservation of the legacy row after failed persistence.
- [X] T002 Change the bulk legacy importer in `src/cache_store.rs` to defer MOEX response rows while retaining startup migration for CBR and SPBEX rows and preserving rows skipped for MOEX.
- [X] T003 Implement an idempotent, atomic per-symbol legacy import in `src/cache_store.rs`; read the exact normalized `MOEX:{SYMBOL}` key, skip an existing collection, and retain the source row unless the durable merge succeeds, including when the global schema migration marker already exists.
- [X] T004 Add route-level migration tests in `tests/api_moex.rs` proving unknown metadata returns HTTP 400 without migration, metadata failure returns the MOEX dependency error without migration, recognized metadata permits migration before history retrieval, and the first response includes migrated plus fetched records; also assert the preflight metadata result is reused rather than fetched twice.
- [X] T005 Add symbol validation tests in `tests/api_moex.rs` for `IMOEX` and `GLDRUB_TOM`, rejection of leading, trailing, or repeated underscores for newly supported symbols, and unchanged validation behavior for existing symbols.
- [X] T006 Add an overridable async history-preflight method to `src/provider.rs` with a default success implementation so existing CBR and SPBEX providers retain their current behavior.
- [X] T007 Define the resolved MOEX instrument context in `src/moex/models.rs` with category, engine, market, board, primary status, effective dates, and optional board LOTSIZE.
- [X] T008 Extend `src/moex/board.rs` to parse and select board assignments by engine, market, primary flag, and effective dates; retain the primary-board-per-history-date rule.
- [X] T009 Extend `src/moex/client.rs` to fetch and parse instrument metadata and construct market-aware history/current-marketdata URLs from engine, market, and board context; preserve existing shares trades requests, pagination, response limits, and timeout handling. Leave supported-instrument classification and preflight error mapping to the provider.
- [X] T010 Implement MOEX history preflight in `src/moex/provider.rs`: resolve parsed metadata after syntax validation, classify unknown symbols or symbols without supported board assignments as `InvalidSymbol`, map failed or unusable metadata to the upstream error, and retain recognized context for the subsequent history call.
- [X] T011 Call provider preflight from `src/http/routes.rs` after normalized syntax validation and before migration or collection lookup; only after successful preflight import the targeted cache, then read the collection and call history using the retained context. Map invalid symbols, dependency failures, and migration failures to their documented API errors.

**Checkpoint**: Unknown MOEX symbols and metadata failures do not import cache; recognized symbols import cache before history retrieval and reuse the same metadata context.

---

## Phase 3: User Story 1 - Retrieve benchmark history and current value (Priority: P1)

**Goal**: Serve IMOEX history and its latest published index value through existing v1 and v2 routes.

**Independent Test**: With fixture-backed ISS responses, request IMOEX through v1 and v2 history/quote routes and verify ordered six-field records, metadata preflight reuse, current index value mapping, empty/no-value behavior, and standard dependency errors.

### Tests for User Story 1

- [X] T012 [P] [US1] Add representative IMOEX security metadata, index history, and current-marketdata fixtures in `tests/fixtures/moex/imoex-security.json`, `tests/fixtures/moex/imoex-history.json`, and `tests/fixtures/moex/imoex-marketdata.json`.
- [X] T013 [P] [US1] Add fixture-backed metadata preflight, current-marketdata request, index mapping, v1/v2 route, empty-history, no-current-value, and dependency-error tests in `tests/moex_index.rs`, including the resolved `stock/index/SNDX` context.

### Implementation for User Story 1

- [X] T014 [US1] Implement index history and quote mapping in `src/moex/index.rs`: map `CLOSE`, `HIGH`, and `LOW`; leave absent `VOLUME` null; use the latest non-null `CURRENTVALUE` or fall back to `LASTVALUE`; leave inapplicable quote fields null and retain exactly six public fields.
- [X] T015 [US1] Register the index mapping module in `src/moex/mod.rs` and integrate index history and current-marketdata branches in `src/moex/provider.rs`; select the `stock/index` board assignment for IMOEX and keep quote requests uncached.

**Checkpoint**: IMOEX history and quote are independently usable over v1/v2 without changing the public response shape.

---

## Phase 4: User Story 2 - Retrieve currency instrument history and quote (Priority: P1)

**Goal**: Serve GLDRUB_TOM and similar currency-market symbols over existing v1 and v2 routes using their primary currency-market board.

**Independent Test**: With fixture-backed ISS responses, request GLDRUB_TOM through v1 and v2 history/quote routes and verify CETS-only records, nullable absent history volume, last-trade quote mapping, no-trade behavior, and standard dependency errors.

### Tests for User Story 2

- [X] T016 [P] [US2] Add representative GLDRUB_TOM security metadata, CETS history, and currency current-marketdata fixtures in `tests/fixtures/moex/gldrub_tom-security.json`, `tests/fixtures/moex/gldrub_tom-history.json`, and `tests/fixtures/moex/gldrub_tom-marketdata.json`.
- [X] T017 [P] [US2] Add fixture-backed metadata preflight, current-marketdata request, CETS board selection, history/quote mapping, v1/v2 route, empty-data, and dependency-error tests in `tests/moex_currency.rs`, including LOTSIZE data.

### Implementation for User Story 2

- [X] T018 [US2] Implement currency-market history and quote mapping in `src/moex/currency.rs`: map available OHLC fields and leave absent `VOLUME` null; map `LAST`, `TIME`, and `QTY`; convert `QTY` lots to instrument units using LOTSIZE; normalize the timestamp to UTC.
- [X] T019 [US2] Register the currency mapping module in `src/moex/mod.rs` and integrate currency-market history and current-marketdata branches in `src/moex/provider.rs`; select the primary board such as CETS rather than mixing CNGD or LICU rows, and leave quote requests uncached after T015.

**Checkpoint**: GLDRUB_TOM history and quote are independently usable over v1/v2 without affecting index handling.

---

## Phase 5: User Story 3 - Preserve existing MOEX behavior for other instruments (Priority: P2)

**Goal**: Keep existing equity routes, primary-board behavior, mappings, cache responses, and errors compatible.

**Independent Test**: Compare fixture-backed SBER history and quote through v1 and v2 after preflight and market-aware source resolution; verify existing board selection, field mapping, and errors.

### Tests for User Story 3

- [X] T020 [P] [US3] Add SBER regression coverage for shares history, latest-trade quote, invalid symbols, and upstream failures in `tests/api_moex.rs`.

### Implementation for User Story 3

- [X] T021 [US3] Preserve existing shares-market paths, trade timestamp mapping, LOTSIZE selection, and error behavior in `src/moex/provider.rs`, `src/moex/client.rs`, and `src/moex/mapping.rs` while using shared market context.

**Checkpoint**: Existing MOEX equity consumers retain the same routes, response shape, and documented outcomes.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Align published documentation, run security analysis before final validation, and satisfy project-level quality gates.

- [X] T022 Update canonical MOEX history and quote descriptions in `specs/contracts/openapi.yaml` with index/currency examples, current-value versus latest-trade quote semantics, nullable fields, symbol validation, unrecognized-symbol errors, metadata dependency errors, and unchanged response schemas.
- [X] T023 Align runnable fixture and live validation scenarios in `specs/007-moex-instrument-coverage/quickstart.md` with fixture names, metadata preflight ordering/reuse, no-migration error cases, quote request paths, first-request migration and merge behavior, and API outcomes.
- [X] T024 Run the fixture-backed Rust test commands documented in `specs/007-moex-instrument-coverage/quickstart.md`; resolve failures in `tests/` and `src/moex/` before security analysis.
- [X] T025 Run Semgrep source analysis on `src/moex/` after implementation fixes and resolve all findings before final validation.
- [X] T026 Rerun the fixture-backed Rust suite and the functional API scenarios documented in `specs/007-moex-instrument-coverage/quickstart.md` after Semgrep fixes; resolve any regressions in `tests/` and `src/moex/`. Leave live-service checks and performance measurement to their separately scoped validation tasks.
- [X] T027 Measure the 10-client, 10-request-per-second MOEX history and quote profile after code changes are final, and record per-route outcomes in `specs/007-moex-instrument-coverage/quickstart.md`.
- [X] T028 Build the Linux amd64 deliverable from `Dockerfile` after source and performance validation; record the result in `specs/007-moex-instrument-coverage/quickstart.md`.
- [X] T029 Run Trivy against the image built from `Dockerfile` and resolve available High or Critical findings; record any unfixable findings and affected artifact in `specs/007-moex-instrument-coverage/quickstart.md`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No project initialization changes are required.
- **Foundational (Phase 2)**: T001 precedes importer changes T002–T003. T004–T005 define API and validation expectations before implementation. T006–T009 provide the preflight contract and shared context/client support; T010 implements MOEX recognition and context reuse; T011 then orders preflight, migration, collection lookup, and history retrieval.
- **User Stories (Phases 3–5)**: Start after Phase 2. IMOEX and currency fixtures/tests can be developed in parallel. Provider integration tasks T015 and T019 share `src/moex/provider.rs` and `src/moex/mod.rs`, so apply them sequentially.
- **Polish (Phase 6)**: Run the initial suite T024, then Semgrep T025, then the final suite T026 so any Semgrep fixes are covered. Run performance T027 and image build T028 only after source changes are complete; Trivy T029 depends on T028.

### User Story Dependencies

- **US1 (P1)**: Depends on foundational metadata preflight, market context, and client support. No product dependency on US2 or US3.
- **US2 (P1)**: Depends on the same foundation. No product dependency on US1; sequence module/provider integration after T015 to avoid concurrent edits to shared files.
- **US3 (P2)**: Regression coverage follows both market integrations because it validates their combined effect on existing shares behavior.

### Within Each User Story

- Add fixtures and regression tests before story-specific mapping and provider integration.
- Complete mapping before registering/integrating it in shared provider modules.
- Validate each independent story at its checkpoint before moving to the next story.

### Parallel Opportunities

- T001 cache-store tests, T004 migration API tests, and T012/T016 fixture authoring change separate files and can proceed in parallel after their listed prerequisites.
- IMOEX tests T013 and currency tests T017 use separate test files and can be developed in parallel after the shared client foundation.
- Category mappings T014 and T018 use separate source files; module/provider integration T015 and T019 must remain sequential because they share `src/moex/mod.rs` and `src/moex/provider.rs`.
- Run Semgrep after the initial suite and before the final suite, performance profile, and image build; this lets the final suite validate any source fixes found by Semgrep.

## Parallel Example: User Stories 1 and 2

```text
After Phase 2:
Workstream A: T012 -> T013 -> T014 -> T015 (IMOEX)
Workstream B: T016 -> T017 -> T018 (GLDRUB_TOM)
Integrate T019 after T015 to avoid simultaneous edits to src/moex/mod.rs and src/moex/provider.rs.
```

## Implementation Strategy

### MVP First

User Story 1 delivers the benchmark slice after the shared foundation. It is the first independently testable checkpoint, but the complete feature also requires User Story 2 for GLDRUB_TOM and related currency symbols.

### Incremental Delivery

1. Complete Phase 2 and verify symbol syntax validation, metadata recognition/error mapping, no-migration rejection paths, context reuse, and per-symbol cache migration.
2. Complete US1 and validate IMOEX history and quote independently.
3. Complete US2 and validate GLDRUB_TOM history and quote independently.
4. Complete US3 regression checks for existing equities.
5. Run Semgrep, the final Rust/quickstart suite, performance profile, amd64 image build, and Trivy in order.

## Notes

- `[P]` means a task has no dependency on other incomplete tasks and changes a distinct file.
- User-story labels map to the P1/P2 stories in `specs/007-moex-instrument-coverage/spec.md`.

## Phase 7: Convergence

- [X] T030 Reject malformed or overlapping primary-board effective-date intervals during metadata resolution and map unusable assignments to the MOEX dependency error, per FR-009 and plan: effective board dates (partial).

## Phase 8: Convergence

- [X] T031 Add fixture-backed metadata and route tests proving malformed, inverted, and overlapping primary-board effective-date intervals return the standard MOEX dependency error, per FR-009 and Constitution V (partial).
