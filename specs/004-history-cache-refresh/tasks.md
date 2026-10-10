---
description: "Task list template for feature implementation"
---

# Tasks: Permanent History Cache Refresh

**Input**: Design documents from `/specs/004-history-cache-refresh/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md),
[research.md](research.md), [data-model.md](data-model.md),
[contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: Add automated checks for changed API, provider, configuration,
migration, and scheduler behavior, as required by the project constitution and
implementation plan.

**Organization**: Tasks are grouped by user story to enable independently
verifiable increments.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel with other marked tasks after dependencies
    complete and when editing separate files.
- **[Story]**: Maps a task in a user-story phase to `US1` or `US2`.
- Every task names exact project file paths.

## Phase 1: Setup

**Purpose**: The existing Rust service is initialized; no setup changes are
required.

## Phase 2: Foundational

**Purpose**: Add configuration and schema prerequisites used by both user
stories.

- [X] T001 [P] Add config unit tests for full-refresh interval and retry-cap
    defaults, positive and invalid values, acceptance of intervals longer than
    seven days, and startup rejection of `EXCHANGE_API_HISTORY_CACHE_TTL_SECS`
    in `src/config.rs`.
- [X] T002 [P] Add SQLite migration tests proving legacy `MOEX:{symbol}` and
    `CBR:{symbol}` keys map to provider/symbol collections, all
    `SPBEX:{symbol}:{UTC-date}` rows consolidate per symbol, and same-date
    legacy rows are preserved and deduplicated in `tests/cache_store.rs`.
- [X] T003 Parse `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS` as a positive
    integer defaulting to 604800 seconds and
    `EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS` as a positive integer
    defaulting to 900 seconds; reject `EXCHANGE_API_HISTORY_CACHE_TTL_SECS` with
    a migration message in `src/config.rs`.
- [X] T004 Migrate existing SQLite data without dropping retained responses and
    persist provider/symbol identity, dated records, latest record date, last
    successful full-refresh time, consecutive retryable failure count, next
    attempt time, and update time in `src/cache_store.rs`.

**Checkpoint**: Both settings parse with documented defaults and the persistent
store can migrate retained data and refresh metadata.

## Phase 3: User Story 1 - Keep cached history available (Priority: P1) 🎯 MVP

**Goal**: Retain successfully fetched history indefinitely across elapsed time
and service restarts, without age-based expiration or durable capacity eviction.

**Independent Test**: Fetch and persist history, restart with the same database,
and confirm records remain available after the former TTL has elapsed; verify
durable rows are not removed to satisfy a cache byte limit.

### Tests for User Story 1

- [X] T005 [P] [US1] Add SQLite checks for retention beyond expiry, no age-based
    deletion, no byte-limit eviction, and preservation of committed rows after a
    failed write in `tests/cache_store.rs`.
- [X] T006 [P] [US1] Add an API lifecycle check proving history remains
    available after reopening the same database beyond the former TTL in
    `tests/api_contract.rs`.

### Implementation for User Story 1

- [X] T007 [P] [US1] Remove expiry deletion, oldest-first byte eviction, and
    oversized-entry deletion from persistent history writes; preserve committed
    rows and return the existing store error when a write fails in
    `src/cache_store.rs`.
- [X] T008 [P] [US1] Remove TTL invalidation from the history response cache
    while retaining the configured in-memory capacity behavior in
    `src/cache.rs`.

**Checkpoint**: Stored history survives restarts and elapsed time; the removed
TTL setting fails startup and no durable record is automatically evicted.

## Phase 4: User Story 2 - Refresh history on requests and in the background (Priority: P1)

**Goal**: Fetch only records newer than the latest retained date for each user
request and full-refresh every retained provider/symbol collection without user
action, using the configured interval (default seven days).

**Independent Test**: Seed several collections, verify a later request fetches
only newer dates and returns complete merged history, then advance controlled
time past a configured interval longer than seven days and verify there is no
early refresh and every due collection is attempted when due. Inject provider
errors and invalid full responses and verify last-good data and metadata remain
intact.

### Tests for User Story 2

- [X] T009 [P] [US2] Add MOEX fixture checks for initial/full and date-bounded
    history requests, cursor-derived page counts, expected rows per page,
    consistent columns, and failed-page errors in `tests/moex_client.rs`.
- [X] T010 [P] [US2] Add SPBEX fixture checks for incremental time bounds, the
    current UTC-date history boundary, successful empty feed handling, and typed
    HTTP status/symbol-rejection errors in `tests/spbex_client.rs`.
- [X] T011 [P] [US2] Add CBR fixture checks for initial and date-range history
    requests, complete XML parsing/mapping, and typed HTTP status errors in
    `tests/cbr_client.rs`.
- [X] T012 [P] [US2] Add route checks for incremental merge, complete response
    ordering, no duplicate dates, provider-specific HTTP 502 errors, and
    history-store HTTP 503 errors in `tests/api_contract.rs` and
    `tests/api_errors.rs`.
- [X] T013 [P] [US2] Add scheduler tests in `tests/history_refresh.rs` using a
    controlled clock to verify: a configured interval longer than seven days is
    not due just before the interval and is attempted when due; network failures
    and timeouts plus HTTP 408, 425, 429, and representative 5xx including 502
    retry with exponential backoff; representative other 4xx responses defer the
    next attempt until one configured interval after that failed attempt;
    malformed/incomplete responses and duplicate identities with identical or
    conflicting values preserve history and last-success metadata; retry delay
    respects a custom cap, retry state survives restart, and valid success
    resets retry state.

### Implementation for User Story 2

- [X] T014 [US2] Extend the provider history interface in `src/provider.rs` to
    accept provider/symbol and optional latest-date inputs, and preserve typed
    transport/status/symbol-rejection error categories so the background worker
    can classify errors without changing public error envelopes.
- [X] T015 [P] [US2] Add date-bounded incremental fetching for MOEX while
    retaining full-history fetching and reporting cursor/page completeness and
    HTTP status failures in `src/moex/client.rs` and `src/moex/provider.rs`.
- [X] T016 [P] [US2] Add latest-date-bounded chart fetching for SPBEX while
    preserving its current UTC-date exclusion and typed status/symbol-rejection
    errors in `src/spbex/client.rs` and `src/spbex/provider.rs`.
- [X] T017 [P] [US2] Add date-range incremental fetching for CBR and preserve
    typed transport/HTTP status errors in `src/cbr/client.rs` and
    `src/cbr/provider.rs`.
- [X] T018 [US2] Add transactional date-keyed merges, complete-history reads,
    provider/symbol enumeration, last-success metadata, retry failure counts,
    and persisted next-attempt due-time operations in `src/cache_store.rs`.
- [X] T019 [P] [US2] Update history handlers to fetch records after the latest
    retained date, merge successful results, return complete retained history,
    and preserve existing provider/store error mappings in `src/http/routes.rs`.
- [X] T020 [P] [US2] Implement the bounded-concurrency worker in
    `src/history_refresh.rs`: wake for the nearest due collection; honor the
    configured interval; let persisted retry due times override normal cadence;
    retry transport failures/timeouts, HTTP 408/425/429/5xx, and
    invalid/incomplete full responses with exponential delays capped by
    `EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS`; defer other HTTP 4xx
    failures until one configured interval after the failed attempt; validate
    the complete provider result and reject any duplicate identity before atomic
    merge; preserve history and last-success time on failure; update success
    metadata and reset retry state only on success; log outcomes.
- [X] T021 [US2] Start and stop the refresh worker with the service, pass the
    configured full-refresh interval and retry backoff cap, and honor the
    existing 30-second shutdown bound in `src/lib.rs`, `src/main.rs`, and
    `src/shutdown.rs`.

**Checkpoint**: User requests refresh incrementally; background refresh follows
configured due times; failures preserve the last good collection and follow the
specified retry/defer policy.

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Keep operational and public contract documentation aligned and
complete release acceptance gates.

- [X] T022 Document indefinite retention, incremental requests, configured
    full-refresh cadence (seven-day default), retry/error classification,
    duplicate/full-response validation, last-good-data behavior, TTL-setting
    removal, and the non-evicting role of `EXCHANGE_API_HISTORY_CACHE_MAX_BYTES`
    in `specs/contracts/openapi.yaml` and
    `specs/004-history-cache-refresh/quickstart.md`; configure the interval and
    retry cap in `docker-compose.yml`.
- [X] T023 Run the 10-client, 10-request-per-second acceptance workload across
    all six routes, including incremental request and due background refresh
    traffic, and record actual request rate and per-route p95 results in
    `specs/004-history-cache-refresh/quickstart.md`; require at least 95% of
    successful responses per route to complete in under one second.
- [X] T024 Build the Linux amd64 container image, run Semgrep against source and
    Trivy against the built image, resolve all Semgrep findings and all fixable
    High/Critical Trivy findings, rebuild and rerun affected scans after fixes,
    and record results or unavailable fixes with reasons and affected artifacts
    in `specs/004-history-cache-refresh/quickstart.md`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No initialization work is required; the Rust service
    already exists.
- **Foundational (Phase 2)**: T001 and T002 are independent checks. T003 follows
    T001; T004 follows T002. Complete all four before story work.
- **User Story 1 (Phase 3)**: Depends on Phase 2. T005 and T006 can run in
    parallel; T007 and T008 follow their relevant checks and edit separate
    files.
- **User Story 2 (Phase 4)**: Depends on User Story 1's durable non-expiring
    history. T009–T013 are independent checks and can run in parallel. T014
    establishes the provider interface/error categories; T015–T017 follow T014
    and can run in parallel. T018 follows T004 and T007; T019 follows T014–T018;
    T020 follows T014–T018 and T015–T017; T021 follows T020.
- **Polish (Phase 5)**: T022 follows behavior stabilization. T023 follows
    implementation and documentation. T024 follows the final buildable source
    and performance work; if scans require fixes, rebuild and rerun affected
    scans.

### User Story Dependencies

- **User Story 1 (P1)**: Starts after Phase 2 and delivers durable history
    without TTL expiration or persistent capacity eviction.
- **User Story 2 (P1)**: Starts after User Story 1 because incremental requests
    and background enumeration require durable collections and metadata.

### Parallel Opportunities

- **Foundational checks**: T001 and T002.
- **User Story 1 checks**: T005 and T006; then T007 and T008 can proceed
    independently.
- **User Story 2 checks**: T009–T013.
- **User Story 2 provider clients**: T015, T016, and T017 after T014.
- **User Story 2 integration**: T019 and T020 after their listed prerequisites;
    they modify separate source files.
- **Polish**: T022 is documentation/configuration work after behavior
    stabilizes.

## Parallel Example: User Story 2

```text
After T014, work in parallel:
Task: "T015 Add incremental and complete-page MOEX fetching in src/moex/client.rs and src/moex/provider.rs"
Task: "T016 Add incremental SPBEX fetching and typed errors in src/spbex/client.rs and src/spbex/provider.rs"
Task: "T017 Add incremental CBR fetching and typed errors in src/cbr/client.rs and src/cbr/provider.rs"

After provider and store prerequisites complete, work in parallel:
Task: "T019 Integrate incremental history requests in src/http/routes.rs"
Task: "T020 Implement the scheduled refresh worker in src/history_refresh.rs"
```

## Implementation Strategy

### MVP First (User Story 1)

1. Complete Phase 2 configuration and migration prerequisites.
2. Complete User Story 1 checks and implementation for indefinite retention
      without TTL or durable capacity eviction.
3. Validate persistence after restart and beyond the former TTL using the same
      database.

### Incremental Delivery

1. Deliver User Story 1 as the persistent-retention foundation.
2. Implement provider date bounds and transactional merge for User Story 2.
3. Add and wire the background worker with the configured cadence, retry
      classification, atomic validation, and last-good-data preservation.
4. Update API and operational documentation, then pass the performance, Linux
      amd64 build, Semgrep, and Trivy gates.

## Notes

- Task IDs are sequential and every task uses the required checkbox format,
    story labels, and exact file paths.
- `[P]` marks tasks that can run concurrently after their stated dependencies
    complete.
- Automated checks are included for important API behavior and service
    boundaries as required by the project constitution.
