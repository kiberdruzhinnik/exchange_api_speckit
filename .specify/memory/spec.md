# Canonical System Specification

> **Revision**: 2026-10-09 — Archived permanent history retention and scheduled refresh behavior; consolidated matching API and configuration requirements.
> **Revision**: 2026-10-09 — Archived MOEX history and quote behavior, mappings, validation, and shutdown requirements; kept the indefinite date-keyed history model after resolving the legacy storage conflict in favor of the current plan.
> **Revision**: 2026-10-09 — Archived CBR history and latest-official-rate behavior, XML source mapping, shared-provider contract, and configuration requirements; retained the established indefinite durable-history model.

**Status:** Current system requirements, consolidated from the MOEX, SPBEX, and CBR feature specifications.  
**Scope:** The six provider history and quote routes and shared service behavior.

The archived feature task lists are fully checked, and their feature specs are sealed. The historical feature specs are preserved unchanged.

## Shared REST Contract

- **SHARED-FR-001** *(MOEX FR-001, FR-003; SPBEX FR-001, FR-003, FR-007; CBR FR-020, FR-021)*: The API MUST expose `GET /v1/{provider}/{SYMBOL}` and `GET /v1/{provider}/{SYMBOL}/quote` for `moex`, `spbex`, and `cbr`. All success records MUST use exactly `date`, `close`, `high`, `low`, `volume`, and `facevalue` in JSON arrays. History responses MUST contain the complete retained history, including newly fetched records, ordered according to the provider contract. [Source: specs/004-history-cache-refresh/spec.md -> FR-006] [Source: specs/001-moex-ticker-update/spec.md -> FR-001, FR-003] [Source: specs/002-spbex-ticker-update/spec.md -> FR-001, FR-003, FR-007] [Source: specs/003-cbr-currency-rates/spec.md -> FR-020, FR-021]
- **SHARED-FR-002** *(MOEX FR-008; SPBEX FR-010; CBR FR-003, FR-012, FR-022)*: Providers MUST trim surrounding whitespace and uppercase symbols before validation. Malformed or unsupported symbols MUST return HTTP 400 with `{"error":{"code":"invalid_symbol","message":"..."}}`. SPBEX successful empty feeds MUST NOT be treated as unsupported symbols; an explicit upstream rejection may return HTTP 400. CBR symbols MUST be validated against its currently supported-currency directory. [Source: specs/001-moex-ticker-update/spec.md -> FR-008] [Source: specs/002-spbex-ticker-update/spec.md -> FR-010] [Source: specs/003-cbr-currency-rates/spec.md -> FR-003, FR-012]
- **SHARED-FR-003** *(MOEX FR-009; SPBEX FR-011, FR-015; CBR FR-013, FR-015, FR-022, FR-024)*: Upstream failures or unusable upstream data MUST return HTTP 502 in the shared error envelope, retaining the existing route-specific codes `moex_unavailable`, `spbex_unavailable`, or `cbr_unavailable`. History-store failures MUST return HTTP 503 with `history_store_unavailable` in the same envelope. Failed user-request refreshes MUST NOT report stale cached data as a successful refresh. [Source: specs/004-history-cache-refresh/spec.md -> FR-008] [Source: specs/004-history-cache-refresh/spec.md -> FR-010] [Source: specs/001-moex-ticker-update/spec.md -> FR-009] [Source: specs/002-spbex-ticker-update/spec.md -> FR-011, FR-015] [Source: specs/003-cbr-currency-rates/spec.md -> FR-013, FR-015, FR-022, FR-024]
- **SHARED-FR-004** *(MOEX FR-007; SPBEX FR-006, FR-009; CBR FR-008, FR-011)*: Successful empty history MUST return `[]`. A successful quote lookup with no quote MUST return HTTP 200 and a one-element array whose six fields are all `null`. [Source: specs/002-spbex-ticker-update/spec.md -> FR-006, FR-009] [Source: specs/003-cbr-currency-rates/spec.md -> FR-008, FR-011]
- **SHARED-FR-005** *(MOEX FR-010; SPBEX FR-014; CBR FR-016)*: API documentation MUST describe routes, symbols, provider field mappings, empty/no-quote outcomes, and error behavior consistently. It MUST also describe indefinite history retention, incremental request refresh, scheduled full refresh and its cadence configuration, retry behavior, and the persistent-storage requirement; `EXCHANGE_API_HISTORY_CACHE_MAX_BYTES` does not cap or evict durable history. CBR documentation MUST state its rate normalization, null mappings, and effective-date behavior. [Source: specs/004-history-cache-refresh/spec.md -> FR-013] [Source: specs/002-spbex-ticker-update/spec.md -> FR-014] [Source: specs/003-cbr-currency-rates/spec.md -> FR-016]

## Provider Data Requirements

### MOEX

- **MOEX-FR-001** *(MOEX FR-001–FR-006, FR-011)*: `GET /v1/moex/{SYMBOL}` MUST return all daily history available from public MOEX ISS without subscriber credentials, ordered oldest to newest. Each row MUST come from the board primary for that trading date. History dates MUST preserve the MOEX trading date as midnight UTC. Unavailable market values MUST be `null`; a missing or invalid trading date makes upstream data unusable. [Source: specs/001-moex-ticker-update/spec.md -> FR-002, FR-004, FR-006, FR-011]
- **MOEX-FR-002** *(MOEX FR-005; clarified SBER correction)*: `close`, `high`, `low`, and `volume` MUST map from the daily history fields. The JSON field remains named `facevalue` but MUST carry the current `LOTSIZE` for the board primary on that record’s date, or `null` if unavailable. It MUST NOT use MOEX `FACEVALUE`; the expected SBER value for 2026-10-06 is `1`. [Source: specs/001-moex-ticker-update/spec.md -> FR-005]
- **MOEX-FR-003** *(MOEX FR-014)*: Each `/quote` request MUST fetch the latest executed trade from MOEX ISS without reusing a cached quote. Return one history-shaped record mapping UTC trade time to `date`, trade price to `close`, and traded size to `volume`; set `high`, `low`, and `facevalue` to `null`. A valid no-trade response is the shared all-null record. [Source: specs/001-moex-ticker-update/spec.md -> FR-014]

### SPBEX

- **SPBEX-FR-001** *(SPBEX FR-001–FR-006, FR-010, FR-016)*: `GET /v1/spbex/{SYMBOL}` MUST return public chart-feed candles in ascending order, preserving source timestamps normalized to UTC. The history route MUST return only candles dated before the current UTC calendar date; current-date data is reserved for `/quote`. A successful empty feed is valid empty history. Return HTTP 400 for malformed symbols or explicit upstream rejection, not merely an empty feed. [Source: specs/002-spbex-ticker-update/spec.md -> FR-001, FR-002, FR-004, FR-006, FR-010, FR-016]
- **SPBEX-FR-002** *(SPBEX FR-005)*: Map source `close`, `high`, and `low`; return `volume: null` because the feed does not supply volume, and `facevalue: 1`. [Source: specs/002-spbex-ticker-update/spec.md -> FR-005]
- **SPBEX-FR-003** *(SPBEX FR-007–FR-009)*: Each `/quote` request MUST fetch the latest available daily candle from the feed without reusing a prior quote. Return it as one six-field record with the same SPBEX mappings. A successful empty quote lookup returns the shared all-null record. [Source: specs/002-spbex-ticker-update/spec.md -> FR-007, FR-008, FR-009]

### Central Bank of Russia (Bank of Russia)

- **CBR-FR-001** *(CBR FR-001, FR-003–FR-005, FR-008)*: `GET /v1/cbr/{SYMBOL}` MUST return all source-available daily rates from the earliest available date through the latest published date, oldest first. Symbols MUST be present in the Bank of Russia’s supported-currency list. Successful empty history returns `[]`. [Source: specs/003-cbr-currency-rates/spec.md -> FR-001, FR-003–FR-005, FR-008]
- **CBR-FR-002** *(CBR FR-006, FR-007, FR-007a)*: Serialize effective dates at midnight UTC. `close` MUST be Russian rubles per one currency unit (`Value / Nominal`); if either input is absent, `close` is `null`. `facevalue` MUST contain source nominal when available. CBR does not provide `high`, `low`, or `volume`, so these are `null`. A missing or invalid date or otherwise unusable record is an upstream failure. [Source: specs/003-cbr-currency-rates/spec.md -> FR-006, FR-007, FR-007a] [Source: specs/003-cbr-currency-rates/data-model.md -> Daily Currency Rate]
- **CBR-FR-003** *(CBR FR-009–FR-011)*: Each `/quote` request MUST fetch a fresh official rate. Return the latest published rate and its effective date, even when that date precedes today. If a successful source response has no rate, return the shared all-null quote record. [Source: specs/003-cbr-currency-rates/spec.md -> FR-002, FR-009–FR-011]

## User Stories

### User Story 1 - Keep cached history available (Priority: P1)

As an API consumer, I want previously fetched history to remain available indefinitely so that restarting the service or waiting between requests does not discard useful history.

**Why this priority**: Persistent history is the core requested behavior and prevents the cache from expiring merely because time passed.

**Independent Test**: Fetch history, restart the service, and request the same symbol; previously fetched records remain available.

**Acceptance Scenarios**:
1. **Given** history has been fetched and stored, **When** the service restarts, **Then** the stored records remain available.
2. **Given** history has been stored for any length of time, **When** a later request is made, **Then** the stored records have not been removed due to age.

[Source: specs/004-history-cache-refresh/spec.md -> "Keep cached history available"]

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

[Source: specs/004-history-cache-refresh/spec.md -> "Refresh history on each later request"]

### User Story 3 - Retrieve MOEX and SPBEX ticker daily history (Priority: P1)

As a client, I want to request `/v1/moex/<SYMBOL>` and receive daily market records for that MOEX ticker in a consistent JSON format sourced from MOEX ISS.

As a SPBEX client, I want to request `/v1/spbex/{SYMBOL}` and receive dated market records in the same six-field JSON format used by the MOEX API.

**Why this priority**: This is the primary value of the MOEX provider.

**Independent Test**: Request a known ticker and verify the JSON array contains dated records with documented fields and values corresponding to MOEX ISS.

**Acceptance Scenarios**:
1. **Given** a recognized ticker has history, **When** its history route is requested, **Then** records use the six-field response, per-date primary board, and ascending ISO UTC dates.
2. **Given** a recognized ticker has no history, **When** its history route is requested, **Then** the response is `[]`.
3. **Given** the quote route returns a latest trade, **When** the request succeeds, **Then** a one-element six-field array maps UTC execution time, price, and size while other fields are null.
4. **Given** a valid quote lookup has no trade, **When** the request succeeds, **Then** the one-element record has all six fields null.

[Source: specs/001-moex-ticker-update/spec.md -> "Retrieve a ticker’s daily history"]
**SPBEX priority rationale**: Historical price retrieval is the primary purpose of the SPBEX feature and is required independently of current-quote requests. (specs/002-spbex-ticker-update argued P1)

**SPBEX Independent Test**: Request a known SPBEX symbol against a controlled chart-feed response and verify record fields, values, UTC dates, and ascending order.

**SPBEX Acceptance Scenarios**:
5. **Given** a valid SPBEX symbol has daily candles, **When** `/v1/spbex/{SYMBOL}` is requested, **Then** the response contains the six documented fields.
6. **Given** SPBEX candles are out of chronological order, **When** history is requested, **Then** records are ascending and timestamps are UTC.
7. **Given** a valid SPBEX symbol has a successful empty feed, **When** history is requested, **Then** the response is `[]`.
8. **Given** a candle is dated on the current UTC calendar date, **When** SPBEX history is requested, **Then** it is excluded and available only via quote.

As a currency API client, I want to request history for a currency supported by the Bank of Russia and receive its official daily ruble rates in chronological order.

**CBR Why this priority**: Historical rates are the primary CBR capability and let clients analyze currency changes over time. (specs/003-cbr-currency-rates argued P1)

**CBR Independent Test**: Request a supported code such as USD, CNY, or EUR and compare dates and normalized rates with the Bank of Russia's published history.

**CBR Acceptance Scenarios**:
9. **Given** a supported currency has published rate history, **When** `/v1/cbr/{SYMBOL}` is requested, **Then** the service returns six-field records ordered oldest to newest.
10. **Given** the official source expresses a rate for multiple currency units, **When** history is returned, **Then** each rate is normalized to Russian rubles for one unit.
11. **Given** a supported currency has no history in the requested source data, **When** history is requested, **Then** the service returns `[]`.

[Source: specs/002-spbex-ticker-update/spec.md -> "Retrieve an SPBEX ticker's daily history"]
[Source: specs/003-cbr-currency-rates/spec.md -> "Retrieve a supported currency's rate history"]

### User Story 4 - Handle invalid or unavailable MOEX and SPBEX data (Priority: P2)

As a client, I want documented invalid-symbol and upstream errors so that I can distinguish bad requests from temporary MOEX ISS failures.

As a SPBEX client, I want a documented error when my symbol is malformed or explicitly rejected, and a dependency error when SPBEX cannot provide usable data.

**Why this priority**: Clients need to distinguish invalid input from dependency failures.

**Independent Test**: Request an invalid symbol and simulate unavailable or malformed ISS responses.

**Acceptance Scenarios**:
1. **Given** a symbol is malformed or unsupported, **When** it is requested, **Then** a client error uses the standard error representation.
2. **Given** ISS history is unavailable or unsuccessful, **When** history is requested, **Then** a dependency error is returned rather than empty history.
3. **Given** ISS trade data is unavailable or malformed, **When** a quote is requested, **Then** a dependency error is returned rather than a no-trade success.

[Source: specs/001-moex-ticker-update/spec.md -> "Handle invalid or unavailable ticker data"]
**SPBEX priority rationale**: Consumers need to distinguish a request problem from an exchange dependency failure. (specs/002-spbex-ticker-update argued P2)

**SPBEX Independent Test**: Submit malformed symbols and simulate unsuccessful, malformed, and unusable SPBEX responses; verify each outcome matches the error contract.

**SPBEX Acceptance Scenarios**:
4. **Given** a malformed symbol or explicit upstream rejection, **When** either SPBEX route is requested, **Then** HTTP 400 uses the standard error format.
5. **Given** a valid symbol returns a successful empty feed, **When** history or quote is requested, **Then** the documented empty-history or no-quote result is returned.
6. **Given** SPBEX is unavailable or returns unsuccessful, malformed, oversized, or unusable data, **When** either route is requested, **Then** HTTP 502 is returned rather than empty market data.
7. **Given** the persistent history store cannot be read or written, **When** SPBEX history is requested, **Then** HTTP 503 uses the standard store-error format.


[Source: specs/002-spbex-ticker-update/spec.md -> "Distinguish invalid symbols from SPBEX failures"]

### User Story 5 - Stop the service with Ctrl+C (Priority: P2)

As an operator, I want to interrupt the service and have it stop accepting requests and exit cleanly.

**Why this priority**: Predictable shutdown lets operators stop or replace the service safely.

**Independent Test**: Send SIGINT during active requests and verify graceful drain and the 30-second deadline.

**Acceptance Scenarios**:
1. **Given** the service receives SIGINT, **When** requests are in flight, **Then** it stops accepting work, drains requests, and exits within 30 seconds.
2. **Given** requests remain after the deadline, **When** the deadline expires, **Then** remaining work is cancelled.

[Source: specs/001-moex-ticker-update/spec.md -> "Stop the service with Ctrl+C"]

### User Story 6 - Retrieve the latest SPBEX quote (Priority: P2)

A client requests the latest available SPBEX quote for a symbol using a dedicated route and receives a one-record array with the same fields as a history record.

**Why this priority**: Clients need an explicit way to refresh the current value without parsing the full history response.

**Independent Test**: Request a quote using a controlled upstream response and verify it reflects the latest available value, follows the one-record response shape, and is fetched again on a subsequent request.

**Acceptance Scenarios**:
1. **Given** a syntactically valid symbol has current SPBEX data, **When** `/v1/spbex/{SYMBOL}/quote` is requested, **Then** a one-element array uses the six-field record shape.
2. **Given** upstream quote data changes between requests, **When** quote is requested twice, **Then** the second response reflects a fresh upstream fetch.
3. **Given** a valid symbol has no quote data across the available range, **When** quote is requested, **Then** the documented all-null result is returned.

As a currency client, I want the latest official Bank of Russia rate for a supported currency without fetching its entire history.

**CBR Why this priority**: Clients need a direct current-rate route without scanning full currency history. (specs/003-cbr-currency-rates argued P2)

**CBR Independent Test**: Request a supported currency quote and verify its date and normalized rate against the latest official Bank of Russia rate available at request time.

**CBR Acceptance Scenarios**:
4. **Given** a rate is published for the current date, **When** `/v1/cbr/{SYMBOL}/quote` is requested, **Then** a one-element array returns the current date and normalized rate using the six-field record shape.
5. **Given** no rate has been published for today's calendar date, **When** quote is requested, **Then** the latest published rate and its actual effective date are returned.
6. **Given** a successful source response has no rate for a supported currency, **When** quote is requested, **Then** one record has all six fields set to `null`.

[Source: specs/002-spbex-ticker-update/spec.md -> "Retrieve the latest SPBEX quote"]
[Source: specs/003-cbr-currency-rates/spec.md -> "Retrieve the current official currency rate"]

### User Story 7 - Unify provider contracts and shared configuration (Priority: P2)

A client can distinguish invalid input from temporary source failures, and all provider routes share consistent contracts and configuration.

**Why this priority**: Clear errors prevent clients from treating invalid input or unavailable rates as valid market data. Shared provider contracts and configuration also make the service predictable across exchanges.

**Independent Test**: Request malformed and unsupported codes and simulate failed or unusable source responses; verify each has its documented outcome.

**Acceptance Scenarios**:
1. **Given** a malformed code or a code absent from the Bank of Russia's supported-currency list, **When** a client requests either route, **Then** the service returns HTTP `400` in the standard JSON error format.
2. **Given** the Bank of Russia source is unavailable or returns malformed or unusable data, **When** a client requests either route, **Then** the service returns HTTP `502` in the standard JSON error format.
3. **Given** a history-store operation fails while serving currency history, **When** a client requests history, **Then** the service returns HTTP `503` in the standard JSON error format.
4. **Given** a client sends a lowercase symbol with surrounding whitespace to any provider route, **When** the request is processed, **Then** the provider receives the trimmed uppercase symbol; malformed or unsupported symbols receive the shared HTTP `400` outcome.
5. **Given** any provider returns successful empty history or no quote, **When** the corresponding route is requested, **Then** history returns `[]` or quote returns one all-null record, while each provider retains its documented quote meaning.
6. **Given** all three providers run with shared configuration, **When** application settings are configured, **Then** shared settings apply consistently and provider-specific settings remain independently configurable, and every application configuration environment variable begins with `EXCHANGE_API_`.
7. **Given** a provider handles history or quote operations, **When** it is used by the service, **Then** it supports the common history and quote behavior and returns the shared normalized records and error outcomes.

[Source: specs/003-cbr-currency-rates/spec.md -> "Handle unsupported currencies and source failures"]

## Shared Freshness, Performance, and Operations

- **SHARED-FR-006** *(MOEX FR-013; SPBEX FR-013; CBR FR-014)*: Unexpired history MUST survive application restarts. Expired history MUST be refreshed before it is returned; the specification does not prescribe a storage mechanism or freshness duration. [Source: specs/001-moex-ticker-update/spec.md -> FR-013] [Source: specs/002-spbex-ticker-update/spec.md -> FR-013] [Source: specs/003-cbr-currency-rates/spec.md -> FR-014]
- **SHARED-FR-007** *(MOEX FR-014; SPBEX FR-008; CBR FR-010)*: Quote responses MUST be fetched for each request and MUST NOT be served from a prior quote response. [Source: specs/002-spbex-ticker-update/spec.md -> FR-008] [Source: specs/003-cbr-currency-rates/spec.md -> FR-010]
- **SHARED-FR-008** *(MOEX FR-012; SPBEX FR-012; CBR FR-017)*: Under 10 concurrent clients issuing 10 requests per second total, at least 95% of successful responses on each of the six routes MUST complete end to end in under one second. This is a hard acceptance gate; a miss requires optimization and another measurement. [Source: specs/002-spbex-ticker-update/spec.md -> FR-012] [Source: specs/003-cbr-currency-rates/spec.md -> FR-017]
- **SHARED-FR-009** *(MOEX FR-015; SPBEX FR-014; CBR FR-018)*: The release MUST successfully build a Linux amd64 container image. AMD64 acceptance requires build success only, not runtime or route testing. [Source: specs/003-cbr-currency-rates/spec.md -> FR-018]
- **SHARED-FR-010** *(MOEX FR-016; CBR FR-019)*: On Ctrl+C (SIGINT), the service MUST stop accepting new requests, allow in-flight work to finish, and exit within 30 seconds. Remaining work MUST be cancelled when the deadline expires. [Source: specs/001-moex-ticker-update/spec.md -> FR-016] [Source: specs/003-cbr-currency-rates/spec.md -> FR-019]
- **SHARED-FR-011** *(CBR FR-023, FR-025)*: Every application configuration environment variable MUST begin with `EXCHANGE_API_`. Shared settings MUST apply consistently across providers; source and response-size settings MAY be provider-specific. Current names are `EXCHANGE_API_LISTEN_ADDR`, `EXCHANGE_API_REQUEST_TIMEOUT_SECS`, `EXCHANGE_API_HISTORY_CACHE_MAX_BYTES`, `EXCHANGE_API_HISTORY_CACHE_DB_PATH`, `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS`, `EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS`, `EXCHANGE_API_MOEX_ISS_BASE_URL`, `EXCHANGE_API_MOEX_MAX_ISS_RESPONSE_BYTES`, `EXCHANGE_API_MOEX_MAX_HISTORY_BYTES`, `EXCHANGE_API_SPBEX_API_BASE_URL`, `EXCHANGE_API_SPBEX_MAX_RESPONSE_BYTES`, `EXCHANGE_API_CBR_API_BASE_URL`, and `EXCHANGE_API_CBR_MAX_RESPONSE_BYTES`. `EXCHANGE_API_HISTORY_CACHE_TTL_SECS` is removed; if set, startup MUST fail with a migration message directing operators to `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS`. [Source: specs/004-history-cache-refresh/spec.md -> FR-014] [Source: specs/003-cbr-currency-rates/spec.md -> FR-023, FR-025]
- **SHARED-FR-012** *(project implementation behavior)*: `GET /health/live` reports process liveness. `GET /health/ready` reports readiness and returns HTTP 503 when the history store cannot be queried.

### History Retention and Refresh

- **SHARED-FR-013**: Successfully stored history records MUST be retained indefinitely unless an operator explicitly removes the history store. [Source: specs/004-history-cache-refresh/spec.md -> FR-001]
- **SHARED-FR-014**: History records MUST NOT expire or be evicted based solely on age or a TTL setting. [Source: specs/004-history-cache-refresh/spec.md -> FR-002]
- **SHARED-FR-015**: Each history request MUST fetch source records newer than the latest retained record, even when history is already stored. [Source: specs/004-history-cache-refresh/spec.md -> FR-003]
- **SHARED-FR-016**: The service MUST full-refresh every retained provider/symbol history collection in the background without user action, on a configurable positive interval defaulting to seven days. [Source: specs/004-history-cache-refresh/spec.md -> FR-004]
- **SHARED-FR-017**: Successful source fetches and background refreshes MUST merge records into persistent history, adding new dates and replacing values when the source revises a date. [Source: specs/004-history-cache-refresh/spec.md -> FR-005]
- **SHARED-FR-018**: Refreshing unchanged history MUST NOT create duplicate provider/symbol/date records. [Source: specs/004-history-cache-refresh/spec.md -> FR-007]
- **SHARED-FR-019**: Failed, invalid, or incomplete background refreshes MUST leave the retained collection and last-successful-refresh time unchanged. [Source: specs/004-history-cache-refresh/spec.md -> FR-009]
- **SHARED-FR-020**: Existing history and quote response schemas, symbol validation, and provider-specific mappings MUST remain unchanged. [Source: specs/004-history-cache-refresh/spec.md -> FR-011]
- **SHARED-FR-021**: Operators MUST be able to configure the full-refresh interval in seconds with positive-integer `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS`, defaulting to `604800`. [Source: specs/004-history-cache-refresh/spec.md -> FR-012]
- **SHARED-FR-022**: The worker MUST retry network failures, timeouts, HTTP 408, 425, 429, 5xx, and invalid/incomplete data with exponential backoff starting at one second until a valid refresh succeeds; other HTTP 4xx responses are deferred until the next normal schedule, and retry delays MUST not exceed the configured cap. [Source: specs/004-history-cache-refresh/spec.md -> FR-015]
- **SHARED-FR-023**: Operators MUST be able to configure a positive-integer retry cap in seconds using `EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS`, defaulting to `900`; successful refreshes MUST reset that collection's retry state. [Source: specs/004-history-cache-refresh/spec.md -> FR-016]
- **SHARED-FR-024**: The worker MUST validate the complete full-refresh result before atomically merging it. Failed pages, malformed or unusable records, invalid dates, and duplicate provider/symbol/date identities MUST preserve existing records and last-successful-refresh metadata. [Source: specs/004-history-cache-refresh/spec.md -> FR-017]

## Key Entities

### History Collection

Represents retained daily history for one normalized symbol at one provider. Its fields are provider and symbol identity, dated records, latest record date, last successful full-refresh time, consecutive refresh-failure count, next attempt time, and update time. Records are unique by date, ordered by provider contract, and remain available until explicitly removed by an operator. [Source: specs/004-history-cache-refresh/data-model.md -> "History Collection"]

### History Record

Represents one provider's daily history row with required date identity and nullable `close`, `high`, `low`, `volume`, and `facevalue` values. Its identity is `(provider, symbol, date)`; successful source data replaces a matching record and adds unseen dates. For MOEX, the trading date is UTC midnight, values come from the board primary on that date, `volume` maps instrument units rather than turnover, and `facevalue` contains current board `LOTSIZE` (not `FACEVALUE` or historical LOTSIZE). For SPBEX, `date` preserves the Unix-seconds source timestamp in UTC; `close`, `high`, and `low` must be finite positive prices with `low <= close <= high`; `volume` is null and `facevalue` is `1`. For CBR, `date` is the effective date at UTC midnight, `close` is source `Value / Nominal` in RUB per unit (or null if either is absent), `facevalue` is source nominal, and unavailable high/low/volume fields are null. Duplicate identities or unusable provider data reject the source result. [Source: specs/004-history-cache-refresh/data-model.md -> "History Record"] [Source: specs/001-moex-ticker-update/data-model.md -> "Daily market record"] [Source: specs/002-spbex-ticker-update/data-model.md -> "Daily Market Record"] [Source: specs/003-cbr-currency-rates/data-model.md -> "Daily Currency Rate"]

### Ticker

Identifies a normalized MOEX instrument resolved against security and board metadata. Its primary board is determined for each history date; the response `facevalue` uses current board-specific `LOTSIZE`, or null when unavailable. [Source: specs/001-moex-ticker-update/data-model.md -> "Ticker"]

### Latest Trade Quote Record

Represents the latest executed MOEX trade at request time as one history-shaped record. Trade date/time is converted from Europe/Moscow to UTC; price maps to `close`, quantity to `volume`, and `high`, `low`, and `facevalue` are null. This record is request-scoped and is not persisted or cached. [Source: specs/001-moex-ticker-update/data-model.md -> "Latest trade quote record"]

### SPBEX Ticker

Identifies a SPBEX chart-feed instrument. Normalize its symbol by trimming surrounding whitespace, uppercasing ASCII letters, and validating `[A-Za-z0-9._-]+`; use the normalized symbol for requests and exchange-qualified storage identity. [Source: specs/002-spbex-ticker-update/data-model.md -> "SPBEX Ticker"]

### Latest SPBEX Quote Record

Represents the latest available SPBEX daily candle fetched for a quote request. It is returned as a one-element history-shaped array, is not read from or written to history storage, and has null market fields when the full available chart range contains no candles. Unlike the MOEX quote entity, this is a daily candle, not an executed trade. [Source: specs/002-spbex-ticker-update/data-model.md -> "Latest Quote Record"]

### Supported Currency

Identifies a currency by its public three-letter code, normalized by trimming and uppercasing and validated against the Bank of Russia's current supported-currency directory. The source's internal identifier is used for history requests and is not exposed by the API. [Source: specs/003-cbr-currency-rates/data-model.md -> Supported Currency]

### Latest CBR Quote Record

Represents the latest official Bank of Russia rate fetched for a quote request. It is a one-element history-shaped response, carries the source's effective date and normalized RUB-per-unit rate, and is not read from or written to history storage. A successful lookup with no rate returns all six fields as null. [Source: specs/003-cbr-currency-rates/data-model.md -> Latest Quote]

## Edge Cases

- A failed source fetch during a user request returns the documented provider-specific upstream error; cached data is not reported as a successful refresh. SPBEX oversized, truncated, malformed, unsuccessful, or unusable responses are not treated as valid empty data. [Source: specs/004-history-cache-refresh/spec.md -> "If the source request fails"] [Source: specs/002-spbex-ticker-update/spec.md -> FR-011] [Source: specs/002-spbex-ticker-update/spec.md -> "The SPBEX response is too large, truncated, malformed, or unsuccessful"]
- A history-store read or update failure returns the documented history-store error, including HTTP 503 for SPBEX history. [Source: specs/004-history-cache-refresh/spec.md -> "If the history store cannot be queried"] [Source: specs/002-spbex-ticker-update/spec.md -> FR-015]
- Retryable background failures preserve last-good history and retry with configured backoff; other HTTP 4xx responses wait until the next normal schedule. [Source: specs/004-history-cache-refresh/spec.md -> "If a background full refresh receives a network failure"]
- A full refresh validates all pages and records before commit. Failed pages, missing or invalid dates, unusable records, or any duplicate identity reject the refresh without partial updates. [Source: specs/004-history-cache-refresh/spec.md -> "A full refresh MUST validate the complete response"]
- A successful empty source response returns the documented empty-history result; previously retained records remain available. For SPBEX, a successful empty chart feed does not establish that a syntactically valid symbol is unsupported. [Source: specs/004-history-cache-refresh/spec.md -> "If a successful source response contains no history"] [Source: specs/002-spbex-ticker-update/spec.md -> FR-006, FR-010]
- Repeated requests do not create duplicate records for the same provider, symbol, and source identity. [Source: specs/004-history-cache-refresh/spec.md -> "Repeated requests must not create duplicate entries"]
- A malformed or unsupported MOEX symbol returns the standard client error; valid symbols with no history return `[]`. SPBEX trims surrounding whitespace and uppercases symbols before syntax validation. [Source: specs/001-moex-ticker-update/spec.md -> FR-007, FR-008] [Source: specs/002-spbex-ticker-update/spec.md -> "A symbol has whitespace or lowercase letters"]
- MOEX dates retain their trading date at UTC midnight; nullable market values remain null, but a missing/invalid date or unresolved primary board makes the response unusable. [Source: specs/001-moex-ticker-update/spec.md -> FR-004, FR-011]
- A valid empty MOEX trades response returns the all-null quote record; malformed or failed trade data returns a dependency error. [Source: specs/001-moex-ticker-update/spec.md -> FR-014]
- SIGINT during active work drains within 30 seconds and cancels work remaining at the deadline. [Source: specs/001-moex-ticker-update/spec.md -> FR-016]
- SPBEX candles with missing, non-positive, or non-finite prices, inconsistent OHLC values, invalid/duplicate timestamps, or oversized/malformed response data are rejected as upstream failures rather than returned partially. [Source: specs/002-spbex-ticker-update/data-model.md -> "Daily Market Record"] [Source: specs/002-spbex-ticker-update/spec.md -> "A candle has an invalid or duplicate timestamp"]
- A SPBEX latest-candle quote searches an adaptive recent-to-older range; no quote is reported only after the search reaches the Unix epoch with no candles. [Source: specs/002-spbex-ticker-update/research.md -> "Quote freshness and latency"]
- CBR rates expressed for multiple currency units are normalized by dividing source value by nominal; an absent value or nominal yields null `close`, while an absent or invalid effective date makes the response unusable. [Source: specs/003-cbr-currency-rates/spec.md -> FR-006, FR-007]
- CBR does not supply high, low, or volume, so these are null; `facevalue` contains the source nominal when present and is null otherwise. [Source: specs/003-cbr-currency-rates/spec.md -> FR-006, FR-007a]
- CBR's valid empty history is `[]`; a successful quote lookup without a rate is the shared all-null quote record, and a non-publication day returns the latest available effective date. [Source: specs/003-cbr-currency-rates/spec.md -> FR-008, FR-009, FR-011]
- The supported-currency directory is validated from current public CBR data and cached for 60 seconds; rate history and quotes are separate from this metadata cache. [Source: specs/003-cbr-currency-rates/research.md -> "Official source and rate mapping"]
- Non-positive nominal or rate values, malformed or oversized XML, duplicate dates, or invalid effective dates are upstream failures; CBR does not supply high, low, or volume. [Source: specs/003-cbr-currency-rates/data-model.md -> Daily Currency Rate] [Source: specs/003-cbr-currency-rates/research.md -> "Official source and rate mapping"]
- CBR configuration uses the `EXCHANGE_API_` namespace for shared and provider-specific settings; provider source URLs and response-size limits remain independently configurable. [Source: specs/003-cbr-currency-rates/spec.md -> FR-025]

## Success Criteria

- **SHARED-SC-001**: All successfully stored history remains available after restart and arbitrary elapsed time unless explicitly deleted by an operator; previously applicable freshness rules are met by refreshing stale history before returning it. [Source: specs/004-history-cache-refresh/spec.md -> SC-001] [Source: specs/002-spbex-ticker-update/spec.md -> SC-004] [Source: specs/003-cbr-currency-rates/spec.md -> SC-005]
- **SHARED-SC-002**: Every successful history request fetches source records newer than the latest cached record. [Source: specs/004-history-cache-refresh/spec.md -> SC-002]
- **SHARED-SC-003**: Newly available source records appear in the next successful response for their symbol. [Source: specs/004-history-cache-refresh/spec.md -> SC-003]
- **SHARED-SC-004**: Every cached collection is attempted in the background no later than one configured interval after its previous successful full refresh while the service operates; the default interval is seven days and failed attempts follow the retry policy. [Source: specs/004-history-cache-refresh/spec.md -> SC-004]
- **SHARED-SC-005**: Repeated unchanged requests return the complete history response shape without duplicate records. [Source: specs/004-history-cache-refresh/spec.md -> SC-005]
- **SHARED-SC-006**: Failed user-request source/store operations return documented errors, while failed background refreshes preserve last-good history. [Source: specs/004-history-cache-refresh/spec.md -> SC-006]
- **SHARED-SC-007**: Startup rejects the removed TTL setting and names the replacement interval setting. [Source: specs/004-history-cache-refresh/spec.md -> SC-007]
- **SHARED-SC-008**: Transient upstream or invalid/incomplete full responses preserve prior data and retry with increasing delays bounded by the configured cap until a valid refresh succeeds. [Source: specs/004-history-cache-refresh/spec.md -> SC-008]
- **SHARED-SC-009**: The configured retry cap controls the maximum delay and defaults to 900 seconds. [Source: specs/004-history-cache-refresh/spec.md -> SC-009]
- **SHARED-SC-010**: MOEX history returns all available public records from the primary board for each date, with correct UTC dates and the six-field mapping; SPBEX history returns six-field records sorted oldest to newest with source timestamps represented in UTC; CBR history returns source-available daily rates oldest first with correct UTC effective dates and normalized RUB-per-unit values. Valid empty history is `[]`. [Source: specs/001-moex-ticker-update/spec.md -> SC-002, SC-003] [Source: specs/002-spbex-ticker-update/spec.md -> SC-001] [Source: specs/003-cbr-currency-rates/spec.md -> SC-001]
- **SHARED-SC-011**: Invalid symbols, unavailable history, malformed/unavailable quote data, valid empty responses, and history-store failures produce distinguishable outcomes matching their documented contracts. [Source: specs/001-moex-ticker-update/spec.md -> SC-004] [Source: specs/002-spbex-ticker-update/spec.md -> SC-005] [Source: specs/003-cbr-currency-rates/spec.md -> SC-006]
- **SHARED-SC-012**: A MOEX quote request fetches a fresh latest trade, maps it to the one-record response, and returns an all-null record only for a valid no-trade response. A SPBEX quote request fetches current chart data again on a subsequent request, so an upstream change is reflected. CBR quote responses use the latest official rate available at request time and include its effective date. [Source: specs/001-moex-ticker-update/spec.md -> SC-006] [Source: specs/002-spbex-ticker-update/spec.md -> SC-003] [Source: specs/003-cbr-currency-rates/spec.md -> SC-003]
- **SHARED-SC-013**: At least 95% of successful responses from all six MOEX, SPBEX, and CBR history and quote routes complete end to end in under one second at 10 concurrent clients and 10 total requests per second. SPBEX history-only, quote-only, and combined profiles must each sustain the full rate and meet the gate. [Source: specs/002-spbex-ticker-update/spec.md -> SC-002] [Source: specs/003-cbr-currency-rates/spec.md -> SC-004]
- **SHARED-SC-014**: Every numeric CBR `close` is RUB per one currency unit, regardless of source nominal, and unavailable fields are JSON null. [Source: specs/003-cbr-currency-rates/spec.md -> SC-002]
- **SHARED-SC-015**: A Linux amd64 container image builds successfully; amd64 runtime startup and route checks are not required. [Source: specs/003-cbr-currency-rates/spec.md -> SC-007]
- **SHARED-SC-016**: After SIGINT, the service stops accepting requests and exits within 30 seconds, draining in-flight work where possible and cancelling what remains at the deadline. [Source: specs/003-cbr-currency-rates/spec.md -> SC-008]
- **SHARED-SC-017**: MOEX, SPBEX, and CBR expose the same documented history and quote response structure, symbol handling, and error outcomes while preserving provider-specific quote meanings. [Source: specs/003-cbr-currency-rates/spec.md -> SC-009]
- **SHARED-SC-018**: Shared runtime settings affect every provider consistently and provider-specific source settings remain independently configurable. [Source: specs/003-cbr-currency-rates/spec.md -> SC-010]
- **SHARED-SC-019**: Every documented and accepted application configuration environment variable begins with `EXCHANGE_API_`, including provider-specific settings. [Source: specs/003-cbr-currency-rates/spec.md -> SC-011]

## Assumptions

- **AS-001**: Indefinite retention excludes deliberate operator deletion or loss of underlying storage. [Source: specs/004-history-cache-refresh/spec.md -> "Lives forever"]
- **AS-002**: User requests fetch records newer than the latest retained date; scheduled full refreshes reconcile older revisions. [Source: specs/004-history-cache-refresh/spec.md -> "User-request refreshes"]
- **AS-003**: The full-refresh interval accepts any positive number of seconds and defaults to 604800 seconds. [Source: specs/004-history-cache-refresh/spec.md -> "configurable full-refresh interval"]
- **AS-004**: Retryable failures use persisted exponential backoff from one second to the configured cap, default 900 seconds. [Source: specs/004-history-cache-refresh/spec.md -> "Retryable background failures"]
- **AS-005**: TTL-based history expiration and `EXCHANGE_API_HISTORY_CACHE_TTL_SECS` are removed; deployments must migrate to the interval setting. [Source: specs/004-history-cache-refresh/spec.md -> "TTL-based history expiration"]
- **AS-006**: Every provider/symbol collection already stored is tracked for background refresh, including symbols not requested again; failures preserve retained history. [Source: specs/004-history-cache-refresh/spec.md -> "The application tracks symbols"]
- **AS-007**: Existing provider-specific error behavior remains authoritative when refreshes fail. [Source: specs/004-history-cache-refresh/spec.md -> "Existing provider-specific error behavior"]
- **AS-008**: MOEX and SPBEX public data require no subscriber credentials; source availability and limits remain external dependencies. [Source: specs/001-moex-ticker-update/spec.md -> Assumptions] [Source: specs/002-spbex-ticker-update/spec.md -> "SPBEX data is available to the service without subscription credentials"]
- **AS-009**: MOEX history dates represent daily trading dates at UTC midnight; the public API has no date-range or pagination parameters. [Source: specs/001-moex-ticker-update/spec.md -> Assumptions]
- **AS-010**: MOEX `facevalue` means current primary-board LOTSIZE; quote timestamp is converted from Europe/Moscow to UTC. [Source: specs/001-moex-ticker-update/data-model.md -> "Ticker", "Latest trade quote record"]
- **AS-011**: `spbex` is the canonical provider path spelling; the initial `sbpex` path was a typo. [Source: specs/002-spbex-ticker-update/spec.md -> "SPBEX is the exchange identifier"]
- **AS-012**: SPBEX symbols are normalized by trimming outer whitespace and uppercasing letters, then validating ASCII letters, digits, period, underscore, and hyphen. [Source: specs/002-spbex-ticker-update/spec.md -> "SPBEX symbols are normalized by trimming"]
- **AS-013**: The SPBEX chart feed is not an instrument catalog; successful empty feed data is valid empty history/no quote, and only explicit source rejection can establish unsupported input. [Source: specs/002-spbex-ticker-update/spec.md -> "The chart feed is not an instrument catalog"]
- **AS-014**: SPBEX history excludes candles dated on the current UTC calendar date; the quote route can include current-date candle data. [Source: specs/002-spbex-ticker-update/spec.md -> "Daily chart history is based on the referenced SPBEX adapter's public chart feed"]
- **AS-015**: The SPBEX quote represents the latest available daily candle, with `facevalue: 1` and unavailable volume represented as null; it is not an executed trade. [Source: specs/002-spbex-ticker-update/spec.md -> "The referenced adapter uses the most recent available daily candle"]
- **AS-016**: The SPBEX feature left its exact storage mechanism outside its own specification; current project storage behavior is defined in the shared implementation plan. [Source: specs/002-spbex-ticker-update/spec.md -> "History freshness and restart persistence follow"] [Source: specs/004-history-cache-refresh/plan.md -> "Storage"]
- **AS-017**: The service is intended for a private network; this capability does not add application-level authentication. [Source: specs/002-spbex-ticker-update/spec.md -> "The service is intended for a private network"] [Source: specs/003-cbr-currency-rates/spec.md -> "The existing application is intended for a private network"]
- **AS-018**: CBR denotes the Central Bank of Russia, officially named the Bank of Russia. [Source: specs/003-cbr-currency-rates/spec.md -> "CBR refers to the Central Bank of Russia"]
- **AS-019**: CBR route paths use `{SYMBOL}` and the quote suffix `/quote`. [Source: specs/003-cbr-currency-rates/spec.md -> "The canonical paths use"]
- **AS-020**: CBR close values represent RUB per one unit of foreign currency; source nominal is returned as `facevalue` when available. [Source: specs/003-cbr-currency-rates/spec.md -> "The user intends the close value"]
- **AS-021**: CBR quote dates use the source effective date; on weekends, holidays, or before publication, the latest available rate is returned without inventing a current-date rate. [Source: specs/003-cbr-currency-rates/spec.md -> "The rate's source effective date"]
- **AS-022**: CBR currency support is determined from its current public supported-currency list, not a hard-coded list. [Source: specs/003-cbr-currency-rates/spec.md -> "Currency support is determined"]
- **AS-023**: Providers share record and route behavior while retaining distinct quote meanings: MOEX trade, SPBEX daily candle, and CBR official rate. [Source: specs/003-cbr-currency-rates/spec.md -> "Shared route behavior follows"]
- **AS-024**: Application configuration uses `EXCHANGE_API_`-prefixed environment variables, including provider-specific settings. [Source: specs/003-cbr-currency-rates/spec.md -> "All application configuration is supplied"]

## Implementation Comparison and Open Contradictions

Source review found no behavioral mismatch between the current implementation and the newest applicable feature requirements. It has all six routes, a shared provider interface and record shape, normalized symbols, the specified status/error mappings, persistent history caching, uncached quote fetches, provider-specific field mappings, `EXCHANGE_API_*` configuration, and bounded SIGINT shutdown. Source also exposes the two health routes described above. No implementation changes or tests were run for this consolidation.

The feature quickstarts record final validation on 2026-10-09, including Linux amd64 build success and all six route p95 values below one second under the specified workload: MOEX history/quote p95 were 0.002227/0.398614 seconds; SPBEX 0.003834/0.751137 seconds; CBR 0.003090/0.042939 seconds. Combined-profile route p95 values were also below one second. These are recorded results, not measurements repeated during this consolidation.

1. **SPBEX date boundary:** SPBEX FR-016 says “current calendar date” without a timezone. The OpenAPI contract and implementation use the current UTC date. If “calendar date” means the exchange-local date, the specification and behavior may differ; the intended timezone is not explicit.
Historical feature specifications and their plans, tasks, and contracts remain unchanged.
