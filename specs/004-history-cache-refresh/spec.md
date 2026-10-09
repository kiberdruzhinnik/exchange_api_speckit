# Feature Specification: Permanent History Cache Refresh

**Feature Branch**: `[004-history-cache-refresh]`

**Created**: 2026-10-09

**Status**: Sealed

**Input**: User description: "make sure that cache on history endpoint lives forever and new data is fetched into cache on new subsequent requests"

## Clarifications

### Session 2026-10-09

- Q: Should each history request fetch the provider’s full available history, or fetch only records newer than the latest cached record? → A: Fetch only newer records on each request; refresh the full history for all cached symbols in the background without user consent, using the configured interval, defaulting to seven days.
- Q: How should `EXCHANGE_API_HISTORY_CACHE_TTL_SECS` behave after TTL-based history expiration is deprecated? → A: Remove the variable in this change; configurations that set it will fail startup.
- Q: If a background full refresh fails because the upstream service is unavailable or returns an error, how should retries proceed? → A: Retry with exponential backoff until a refresh succeeds, cap the delay at a configurable maximum, and let operators set that maximum with an environment variable.
- Q: If the configured full-refresh interval is longer than seven days, should the application honor that interval or still refresh at least weekly? → A: Honor any positive configured interval; seven days remains the default cadence.
- Q: If a full-refresh response contains duplicate provider/symbol/date identities, how should the application handle them? → A: Treat any duplicate identity as invalid, even if the duplicate rows have identical values; preserve the existing history and retry the refresh.
- Q: Which upstream HTTP errors should trigger background retries instead of waiting until the next scheduled refresh? → A: Retry network failures/timeouts, HTTP 408, 425, 429, all 5xx responses, and invalid or incomplete data; defer other 4xx responses, including permanent symbol rejection, until the next normal schedule.
- Q: How should the plan describe the background refresh schedule so it matches the spec? → A: Use “configured interval, defaulting to seven days” throughout.
- Q: What should acceptance verify when the configured refresh interval is longer than seven days? → A: Verify that a symbol is not due before the configured interval and becomes due when that interval elapses, using a controlled clock.
- Q: Which duplicate-record cases should acceptance cover to confirm invalid full refreshes preserve existing data? → A: Test both identical-value and conflicting-value duplicates; each must reject the refresh and preserve history and last-successful-refresh metadata.
- Q: Which HTTP status cases should acceptance explicitly verify for background retries and deferral? → A: Cover HTTP 408, 425, 429, representative 5xx responses including 502, and representative other 4xx responses; assert retry or deferral as specified.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Keep cached history available (Priority: P1)

As an API consumer, I want previously fetched history to remain available indefinitely so that restarting the service or waiting between requests does not discard useful history.

**Why this priority**: Persistent history is the core requested behavior and prevents the cache from expiring merely because time passed.

**Independent Test**: Fetch history, restart the service, and request the same symbol; previously fetched records remain available.

**Acceptance Scenarios**:

1. **Given** history has been fetched and stored, **When** the service restarts, **Then** the stored records remain available.
2. **Given** history has been stored for any length of time, **When** a later request is made, **Then** the stored records have not been removed due to age.

### User Story 2 - Refresh history on each later request (Priority: P1)

As an API consumer, I want each subsequent history request to fetch the latest available source data and update the stored history so that new records become visible without waiting for a cache expiration period.

**Why this priority**: Keeping records indefinitely must not make the endpoint serve old history indefinitely; each request needs to discover newly published records.

**Independent Test**: Make a history request, change the source fixture to include a newer record, then request the same history again; the response and stored history include the new record.

**Acceptance Scenarios**:

1. **Given** an earlier history response is stored, **When** another request for the same provider and symbol succeeds, **Then** the source is fetched again and records newer than the latest retained date are added; revisions to older dates are applied by the scheduled full refresh.
2. **Given** source data has not changed, **When** another request succeeds, **Then** the complete history is returned without duplicate records.
3. **Given** a later scheduled full refresh revises a previously stored record, **When** that refresh succeeds, **Then** the stored record reflects the latest source value for that record.
4. **Given** one or more symbols have cached history, **When** their configured full-refresh interval is due, **Then** the application refreshes full history for every such symbol in the background without requiring a user request or consent.
5. **Given** a full refresh receives an upstream error or invalid/incomplete data, **When** the refresh fails, **Then** the existing collection remains unchanged and the worker retries with increasing delays until a valid refresh succeeds.
6. **Given** the configured full-refresh interval is longer than seven days, **When** time advances to just before and then to the configured due time, **Then** the worker does not refresh early and attempts the refresh when due.
7. **Given** a full-refresh response contains duplicate record identities with either identical or conflicting values, **When** validation fails, **Then** the refresh is rejected and the existing history and last-successful-refresh time remain unchanged.
8. **Given** a background refresh receives HTTP 408, 425, 429, representative 5xx responses including 502, or representative other 4xx responses, **When** the worker handles the response, **Then** it retries the retryable statuses using backoff and defers the other 4xx statuses until the next normal schedule.

### Edge Cases

- If the source request fails, the endpoint returns the documented upstream error and does not treat the old cache as a successful refresh.
- If the history store cannot be queried or updated, the endpoint returns the documented history-store error.
- If a background full refresh receives a network failure or timeout, HTTP 408, 425, 429, any 5xx response (including 502), or invalid/incomplete upstream data, it MUST retain the last valid collection and retry using the configured backoff. Other 4xx responses, including permanent symbol rejection, MUST NOT be retried continuously and are retried at the next normal full-refresh schedule.
- A full refresh MUST validate the complete response and all mapped records before committing any changes. A failed page, missing/invalid record date, any duplicate provider/symbol/date identity (even with identical values), or other unusable data MUST NOT partially update the collection or its last-successful-refresh time.
- If a successful source response contains no history, the endpoint returns the documented empty history result; prior records remain retained for future successful refreshes.
- Repeated requests must not create duplicate entries for the same provider, symbol, and source record identity.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST retain successfully fetched history records indefinitely unless an operator explicitly removes the history store.
- **FR-002**: The system MUST NOT expire or evict history records based solely on their age or a time-to-live setting.
- **FR-003**: Each request to a history endpoint MUST fetch records newer than the latest cached record from that provider's source, even when stored history exists.
- **FR-004**: The system MUST perform a full source refresh for every symbol with cached history in the background, without requiring a user request or consent, according to the configured positive full-refresh interval, which MUST default to seven days.
- **FR-005**: After a successful source fetch or background refresh, the system MUST merge the fetched records into persistent history, adding new records and replacing stored values when the source revises the same record.
- **FR-006**: The history response MUST contain the complete retained history for the symbol, including newly fetched records, in the provider's documented order and record format.
- **FR-007**: Refreshing unchanged history MUST NOT produce duplicate records.
- **FR-008**: A failed source fetch during a user request MUST return the existing provider-specific upstream error and MUST NOT report stale cached data as a successful refresh.
- **FR-009**: A failed, invalid, or incomplete background refresh MUST leave the last successfully stored history and last-successful-refresh time intact.
- **FR-010**: A history-store failure during read or update MUST return the existing shared history-store error.
- **FR-011**: Existing history and quote response schemas, symbol validation, and provider-specific mappings MUST remain unchanged.
- **FR-012**: The system MUST allow operators to configure the full-refresh interval using `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS`, expressed in seconds.
- **FR-013**: API documentation MUST describe that history is retained indefinitely, incrementally refreshed on requests, and fully refreshed in the background according to the configured interval and its seven-day default. It MUST document retry behavior and configuration, state that `EXCHANGE_API_HISTORY_CACHE_MAX_BYTES` does not limit or evict durable history, and explain the resulting persistent-storage requirement.
- **FR-014**: The system MUST remove `EXCHANGE_API_HISTORY_CACHE_TTL_SECS`; if this deprecated variable is set, startup MUST fail with a clear message directing the operator to `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS`.
- **FR-015**: The background worker MUST retry network failures, timeouts, HTTP 408, 425, 429, HTTP 5xx, and invalid or incomplete responses using exponential backoff starting at one second and continuing until a valid full refresh succeeds. Other HTTP 4xx responses MUST be deferred until the next normal full-refresh schedule. Retry delays MUST NOT exceed the configured maximum.
- **FR-016**: The system MUST allow operators to configure the retry backoff cap using the positive-integer environment variable `EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS`, defaulting to 900 seconds. A successful full refresh MUST reset that symbol's retry state.
- **FR-017**: The system MUST validate the complete full-refresh result before atomically merging it. Any failed request/page, malformed or unusable record, invalid date, or duplicate provider/symbol/date identity MUST leave the existing collection and last-successful-refresh metadata unchanged.

### Key Entities *(include if feature involves data)*

- **History record**: A provider and symbol's dated market or currency data, including the existing six response fields and a stable source identity for matching records across refreshes.
- **History collection**: The complete retained set of records for a provider and symbol, updated from successful source responses and ordered according to the provider contract.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of successfully stored history remains available after service restart and after an arbitrary elapsed time, absent explicit operator deletion.
- **SC-002**: 100% of successful history requests fetch source records newer than the latest cached record.
- **SC-003**: A newly available record appears in the next successful history response for that symbol.
- **SC-004**: The worker attempts a full background refresh for every symbol with cached history no later than one configured interval after its previous successful full refresh while the application is operating; failed attempts follow the retry policy, and the default interval is seven days.
- **SC-005**: Repeating a successful unchanged history request produces no duplicate records and preserves the complete history response shape.
- **SC-006**: Failed user-request source or history-store operations return the documented error response, while failed background refreshes preserve the last successfully stored history.
- **SC-007**: Startup rejects configurations that set the removed TTL variable and identifies the replacement full-refresh interval setting.
- **SC-008**: After a transient upstream error or invalid/incomplete full response, the background worker retains the prior collection and retries until a complete valid refresh succeeds; retry delays grow exponentially and never exceed the configured cap.
- **SC-009**: Changing `EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS` changes the maximum retry delay, and the default cap is 900 seconds.

## Assumptions

- “Lives forever” means no automatic age-based expiration or eviction; deliberate operator deletion or loss of the underlying storage is outside this behavior.
- User-request refreshes fetch records newer than the latest cached record; the scheduled background refresh reconciles the full available history, including older revisions, at the configured cadence.
- The configurable full-refresh interval is expressed in seconds, may be any positive value, and defaults to 604800 seconds (seven days).
- Retryable background failures use exponential backoff from one second up to the configured maximum, defaulting to 900 seconds; failure count and next retry time are persisted so retries survive application restarts.
- TTL-based history expiration and its `EXCHANGE_API_HISTORY_CACHE_TTL_SECS` setting are removed; deployments must migrate to the full-refresh interval setting.
- The application tracks symbols with cached history so scheduled background refreshes cover them without requiring users to request those symbols again. If a refresh fails, retained history remains available while retries follow the configured backoff.
- Existing provider-specific error behavior remains authoritative when a refresh fails.
