# Canonical System Specification

> **Revision**: 2026-10-09 — Archived permanent history retention and scheduled refresh behavior; consolidated matching API and configuration requirements.
> **Revision**: 2026-10-09 — Archived MOEX history and quote behavior, mappings, validation, and shutdown requirements; kept the indefinite date-keyed history model after resolving the legacy storage conflict in favor of the current plan.
> **Revision**: 2026-10-09 — Archived CBR history and latest-official-rate behavior, XML source mapping, shared-provider contract, and configuration requirements; retained the established indefinite durable-history model.
> **Revision**: 2026-10-10 — Archived provider-qualified v2 history and quote routes, retained v1 compatibility, and the cache-populated latency acceptance criterion.
> **Revision**: 2026-10-10 — Added v2 availability with default refresh settings and the corresponding unset-settings acceptance scenarios.
> **Revision**: 2026-10-10 — Archived configurable process-wide log coloring, its startup configuration contract, and output acceptance criteria.
> **Revision**: 2026-10-10 — Archived MOEX benchmark and currency support, category-specific quote mappings, segmented underscore symbols, and metadata-gated legacy cache migration.
> **Revision**: 2026-10-10 — Added MOEX primary-board interval validation and refreshed the existing 007 archive entry; the traded quote-volume conflict remains unresolved and its conflicting feature requirements were withheld.

**Status:** Current system requirements, consolidated from the MOEX, SPBEX, CBR, and log-coloring feature specifications.
**Scope:** The six provider-specific v1 routes, six provider-qualified v2 routes, and shared service behavior.

The archived feature task lists are complete, and feature specs are marked Completed. Their content is preserved; archival may finalize Draft status metadata.

## Shared REST Contract

- **SHARED-FR-001** *(MOEX FR-001, FR-003; SPBEX FR-001, FR-003, FR-007; CBR FR-020, FR-021)*: The API MUST expose `GET /v1/{provider}/{SYMBOL}` and `GET /v1/{provider}/{SYMBOL}/quote` for `moex`, `spbex`, and `cbr`, and the additive `GET /v2/history/{PROVIDER}/{SYMBOL}` and `GET /v2/quote/{PROVIDER}/{SYMBOL}` routes for those same providers. All success records MUST use exactly `date`, `close`, `high`, `low`, `volume`, and `facevalue` in JSON arrays. History responses MUST contain the complete retained history, including newly fetched records, ordered according to the provider contract. [Source: specs/004-history-cache-refresh/spec.md -> FR-006] [Source: specs/001-moex-ticker-update/spec.md -> FR-001, FR-003] [Source: specs/002-spbex-ticker-update/spec.md -> FR-001, FR-003, FR-007] [Source: specs/003-cbr-currency-rates/spec.md -> FR-020, FR-021] [Source: specs/005-v2-history-quote-api/spec.md -> FR-001, FR-002, FR-004, FR-005] [Source: specs/007-moex-instrument-coverage/spec.md -> FR-004, FR-012]
- **SHARED-FR-025**: V2 requests MUST use the provider path segment, exactly `moex`, `spbex`, or `cbr`, to select that provider's behavior; unsupported provider values MUST return HTTP 400 with the shared `invalid_provider` error envelope and MUST NOT fall through to another provider. [Source: specs/005-v2-history-quote-api/spec.md -> FR-003] [Source: specs/005-v2-history-quote-api/data-model.md -> Provider Selector]
- **SHARED-FR-026**: Existing provider-specific v1 history and quote routes MUST remain available with their existing behavior alongside the v2 routes, including after additional instruments are supported by a provider. [Source: specs/005-v2-history-quote-api/spec.md -> FR-007] [Source: specs/007-moex-instrument-coverage/spec.md -> FR-010]
- **SHARED-FR-028**: V2 history and quote routes MUST remain registered when `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS` and `EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS` are unset; those settings default to 604800 seconds and 900 seconds, respectively, and MUST NOT control route availability. [Source: specs/005-v2-history-quote-api/spec.md -> FR-008] [Source: specs/005-v2-history-quote-api/plan.md -> "Constraints"]
- **SHARED-FR-002** *(MOEX FR-008; SPBEX FR-010; CBR FR-003, FR-012, FR-022)*: Providers MUST trim surrounding whitespace and uppercase symbols before validation. Malformed or unsupported symbols MUST return HTTP 400 with `{"error":{"code":"invalid_symbol","message":"..."}}`. Newly supported MOEX benchmarks and currency instruments MAY contain alphanumeric parts separated by single underscores; leading, trailing, or repeated underscores are invalid, while existing MOEX symbols retain their prior validation behavior. The MOEX history flow MUST confirm recognized metadata before migrating a newly supported symbol's legacy cache; an unrecognized symbol is an invalid-symbol response. SPBEX successful empty feeds MUST NOT be treated as unsupported symbols; an explicit upstream rejection may return HTTP 400. CBR symbols MUST be validated against its currently supported-currency directory. [Source: specs/001-moex-ticker-update/spec.md -> FR-008] [Source: specs/002-spbex-ticker-update/spec.md -> FR-010] [Source: specs/003-cbr-currency-rates/spec.md -> FR-003, FR-012] [Source: specs/007-moex-instrument-coverage/spec.md -> FR-003, FR-008, FR-013]
- **SHARED-FR-003** *(MOEX FR-009; SPBEX FR-011, FR-015; CBR FR-013, FR-015, FR-022, FR-024)*: Upstream failures or unusable upstream data MUST return HTTP 502 in the shared error envelope, retaining the existing route-specific codes `moex_unavailable`, `spbex_unavailable`, or `cbr_unavailable`. History-store failures MUST return HTTP 503 with `history_store_unavailable` in the same envelope. Failed user-request refreshes MUST NOT report stale cached data as a successful refresh. [Source: specs/004-history-cache-refresh/spec.md -> FR-008] [Source: specs/004-history-cache-refresh/spec.md -> FR-010] [Source: specs/001-moex-ticker-update/spec.md -> FR-009] [Source: specs/002-spbex-ticker-update/spec.md -> FR-011, FR-015] [Source: specs/003-cbr-currency-rates/spec.md -> FR-013, FR-015, FR-022, FR-024] [Source: specs/007-moex-instrument-coverage/spec.md -> FR-009]
- **SHARED-FR-004** *(MOEX FR-007; SPBEX FR-006, FR-009; CBR FR-008, FR-011)*: Successful empty history MUST return `[]`. A successful quote lookup with no quote MUST return HTTP 200 and a one-element array whose six fields are all `null`. [Source: specs/002-spbex-ticker-update/spec.md -> FR-006, FR-009] [Source: specs/003-cbr-currency-rates/spec.md -> FR-008, FR-011] [Source: specs/007-moex-instrument-coverage/spec.md -> FR-007]
- **SHARED-FR-005** *(MOEX FR-010; SPBEX FR-014; CBR FR-016)*: API documentation MUST describe routes, symbols, provider field mappings, empty/no-quote outcomes, and error behavior consistently. It MUST also describe indefinite history retention, incremental request refresh, scheduled full refresh and its cadence configuration, retry behavior, and the persistent-storage requirement; `EXCHANGE_API_HISTORY_CACHE_MAX_BYTES` does not cap or evict durable history. CBR documentation MUST state its rate normalization, null mappings, and effective-date behavior. Application configuration documentation MUST describe `EXCHANGE_API_LOG_COLOR`, its enabled default, and its disabling values. [Source: specs/004-history-cache-refresh/spec.md -> FR-013] [Source: specs/002-spbex-ticker-update/spec.md -> FR-014] [Source: specs/003-cbr-currency-rates/spec.md -> FR-016] [Source: specs/006-log-coloring/spec.md -> FR-005] [Source: specs/007-moex-instrument-coverage/spec.md -> FR-011]

## Provider Data Requirements

### MOEX

- **MOEX-FR-001** *(MOEX FR-001–FR-006, FR-011)*: `GET /v1/moex/{SYMBOL}` MUST return all daily history available from public MOEX ISS without subscriber credentials for supported equities, market benchmarks including IMOEX, and MOEX-traded currencies including GLDRUB_TOM, ordered oldest to newest. Each row MUST come from the applicable primary board for that trading date after resolving the instrument's engine and market. Primary-board effective-date intervals MUST have valid dates, inclusive endpoints, and `primary_from` on or before `primary_through` when an end exists; open ends and gaps are allowed, but overlapping intervals make metadata unusable. History dates MUST preserve the MOEX trading date as midnight UTC. Unavailable or inapplicable market values MUST be `null`; a missing or invalid trading date makes upstream data unusable. [Source: specs/001-moex-ticker-update/spec.md -> FR-002, FR-004, FR-006, FR-011] [Source: specs/007-moex-instrument-coverage/spec.md -> FR-001, FR-005, FR-014]
- **MOEX-FR-002** *(MOEX FR-005; clarified SBER correction)*: `close`, `high`, `low`, and `volume` MUST map from the daily history fields. The JSON field remains named `facevalue` but MUST carry the current `LOTSIZE` for the board primary on that record’s date, or `null` if unavailable. It MUST NOT use MOEX `FACEVALUE`; the expected SBER value for 2026-10-06 is `1`. [Source: specs/001-moex-ticker-update/spec.md -> FR-005]
- **MOEX-FR-003** *(MOEX FR-014)*: Each `/quote` request MUST fetch a fresh current value from MOEX ISS without reusing a cached quote. For traded instruments, return the latest executed trade as one history-shaped record mapping UTC trade time to `date`, trade price to `close`, and traded size to `volume`; set `high`, `low`, and `facevalue` to `null`. For market benchmarks, return the latest available published benchmark value in that same record shape, mapping the latest non-null `CURRENTVALUE` or, when absent, `LASTVALUE` to `close`; unavailable fields are null. A valid lookup with no trade or published value is the shared all-null record. [Source: specs/001-moex-ticker-update/spec.md -> FR-014] [Source: specs/007-moex-instrument-coverage/spec.md -> FR-002, FR-006]
- **MOEX-FR-004**: For a history request for a newly supported benchmark or currency symbol, the system MUST validate syntax, resolve metadata, then migrate any exact legacy `MOEX:{SYMBOL}` history into the durable `moex` collection before collection lookup and history retrieval. A recognized instrument reuses the resolved metadata context for retrieval; an unknown symbol or metadata failure MUST NOT migrate cache data. Migration MUST be idempotent and atomic, retain the legacy row if parsing or persistence fails, and return the documented history-store error on migration failure. The first successful response combines migrated and newly fetched records under the existing refresh rules. [Source: specs/007-moex-instrument-coverage/spec.md -> FR-013] [Source: specs/007-moex-instrument-coverage/plan.md -> "Recognize a MOEX instrument before importing matching legacy history"]
- **MOEX-FR-005**: MOEX currency history and quotes MUST use the resolved currency-market primary board. Currency quote records MUST map the latest executed trade's `LAST`, `TIME`, and `QTY`; convert `QTY` lots to instrument units using the board's `LOTSIZE`, and normalize the trade timestamp to UTC. [Source: specs/007-moex-instrument-coverage/data-model.md -> "Current Value Record"] [Source: specs/007-moex-instrument-coverage/research.md -> "Select quote value according to instrument category"]

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

### User Story 8 - Fetch symbol history from v2 (Priority: P1)

As an API client, I want to fetch a symbol's complete history from a versioned v2 route so that history access follows the new public URL structure.

**Why this priority**: History is a primary service capability and the requested route migration.

**Independent Test**: Request a supported provider-symbol pair through `GET /v2/history/{PROVIDER}/{SYMBOL}` and verify the response contains its complete history in the established record format.

**Acceptance Scenarios**:
1. **Given** a supported provider and symbol, **When** its v2 history route is requested, **Then** the service returns that provider-symbol pair's complete retained history using the established six-field record shape and ordering.
2. **Given** an unsupported provider or malformed or unsupported symbol, **When** its v2 history route is requested, **Then** the service returns the established client error behavior.
3. **Given** the source or history store fails, **When** the v2 history route is requested, **Then** the service returns the established upstream or store error behavior.
4. **Given** a valid provider-symbol pair and the source successfully returns no history, **When** its v2 history route is requested, **Then** the service returns an empty array (`[]`).
5. **Given** `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS` and `EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS` are both unset, **When** a v2 history route is requested, **Then** the route is available and history refresh uses the existing defaults.

[Source: specs/005-v2-history-quote-api/spec.md -> "Fetch symbol history from v2"]

### User Story 9 - Fetch the current quote from v2 (Priority: P1)

As an API client, I want to fetch a symbol's current quote from a versioned v2 route so that quote access follows the new public URL structure.

**Why this priority**: Clients need a direct current quote route alongside the new history route.

**Independent Test**: Request a supported provider-symbol pair through `GET /v2/quote/{PROVIDER}/{SYMBOL}` and verify the response matches the existing current-quote format and freshness behavior.

**Acceptance Scenarios**:
1. **Given** a supported provider and symbol with current source data, **When** its v2 quote route is requested, **Then** the service returns that provider's current quote in the established one-record, six-field format.
2. **Given** a valid provider-symbol pair with no quote data, **When** its v2 quote route is requested, **Then** the service returns the established no-quote result.
3. **Given** the source is unavailable or returns unusable data, **When** its v2 quote route is requested, **Then** the service returns the established upstream error behavior.
4. **Given** `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS` and `EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS` are both unset, **When** a v2 quote route is requested, **Then** the route is available.

[Source: specs/005-v2-history-quote-api/spec.md -> "Fetch the current quote from v2"]

### User Story 10 - Disable log colors (Priority: P1)

A service operator running the application in a terminal or log collector that cannot display ANSI colors can disable colored log output through an environment setting. When disabled, log messages remain readable and contain no color control sequences.

**Why this priority**: Color control sequences can make logs difficult to read in colorless terminals and downstream log tools. Disabling them directly addresses that operational need.

**Independent Test**: Start the application with the color setting disabled, produce representative log messages, and verify that messages contain no color control sequences while retaining their normal text and severity information.

**Acceptance Scenarios**:
1. **Given** the color setting is disabled, **When** the application writes log messages, **Then** the messages contain no color control sequences.
2. **Given** the color setting is disabled, **When** the application writes log messages, **Then** message text, severity, and ordering remain available as usual.
3. **Given** `EXCHANGE_API_LOG_COLOR` is set to `false`, `0`, `no`, or `off` in any letter case, **When** the application writes log messages, **Then** coloring is disabled.

[Source: specs/006-log-coloring/spec.md -> "Disable log colors"]

### User Story 11 - Keep colored logs by default (Priority: P1)

An operator who does not configure the color setting receives the existing colored log output by default.

**Why this priority**: Existing deployments should retain the current log presentation without requiring configuration changes.

**Independent Test**: Start the application without the color setting and verify that representative log messages use the existing colored presentation.

**Acceptance Scenarios**:
1. **Given** the color setting is absent, **When** the application writes log messages, **Then** colored output is enabled.
2. **Given** the color setting is explicitly enabled, **When** the application writes log messages, **Then** colored output is enabled.
3. **Given** `EXCHANGE_API_LOG_COLOR` has any other value, **When** the application writes log messages, **Then** coloring remains enabled.

[Source: specs/006-log-coloring/spec.md -> "Keep colored logs by default"]

### User Story 12 - Retrieve MOEX benchmark history and current value (Priority: P1)

As a client, I want to request a MOEX benchmark such as IMOEX through the existing history and quote routes so that I can use its published market history and current index level.

**Why this priority**: Benchmark data is a core MOEX instrument type that clients could not retrieve through the integration.

**Independent Test**: Request IMOEX history and quote and compare returned dates and values with the corresponding MOEX published benchmark data.

**Acceptance Scenarios**:
1. **Given** IMOEX has published daily history, **When** `/v1/moex/IMOEX` is requested, **Then** the service returns records in the established six-field array format ordered oldest to newest.
2. **Given** MOEX has a current published IMOEX value, **When** `/v1/moex/IMOEX/quote` is requested, **Then** the service returns one record representing the latest available benchmark value.
3. **Given** a valid benchmark has no history or current value, **When** the corresponding route is requested, **Then** the established empty-history or no-quote result is returned.
4. **Given** the provider-qualified v2 routes are used, **When** IMOEX history or quote is requested, **Then** the same data and response contract are returned as for v1.
5. **Given** legacy cache history exists for a newly supported symbol, **When** metadata recognizes the symbol, **Then** the history is migrated before history retrieval and combined with fetched records; unknown symbols and metadata failures do not migrate it, and failed migration preserves the legacy row and returns the history-store error.

[Source: specs/007-moex-instrument-coverage/spec.md -> "Retrieve benchmark history and current value"]

### User Story 13 - Retrieve MOEX currency-instrument history and quote (Priority: P1)

As a client, I want to request MOEX-traded currency instruments such as GLDRUB_TOM through the existing MOEX routes so that I can use exchange-traded currency data separately from official central-bank rates.

**Why this priority**: MOEX-traded currencies are distinct from official central-bank rates and serve clients seeking exchange-traded market data.

**Independent Test**: Request GLDRUB_TOM history and quote and compare returned dates, prices, and trade values with corresponding MOEX market data.

**Acceptance Scenarios**:
1. **Given** GLDRUB_TOM has daily history, **When** `/v1/moex/GLDRUB_TOM` is requested, **Then** available records are returned in the established six-field array format ordered oldest to newest.
2. **Given** GLDRUB_TOM has a latest executed trade, **When** `/v1/moex/GLDRUB_TOM/quote` is requested, **Then** one record represents that trade using the established MOEX quote mapping.
3. **Given** another supported MOEX currency instrument has an underscore in its symbol, **When** its history or quote is requested, **Then** it is accepted and handled as that instrument.
4. **Given** provider-qualified v2 routes are used, **When** GLDRUB_TOM history or quote is requested, **Then** the same data and response contract are returned as for v1.

[Source: specs/007-moex-instrument-coverage/spec.md -> "Retrieve currency instrument history and quote"]

### User Story 14 - Preserve existing MOEX behavior for other instruments (Priority: P2)

As a client, I want existing MOEX equities and other supported instruments to retain their routes, response fields, and error behavior when additional instrument categories are added.

**Why this priority**: Expanding instrument coverage must not make existing MOEX integrations less reliable or change their response contract.

**Independent Test**: Request an existing equity such as SBER and verify its history and quote behavior remains consistent with the documented contract.

**Acceptance Scenarios**:
1. **Given** a previously supported MOEX equity, **When** its history or quote route is requested, **Then** existing routes and response contracts remain available.
2. **Given** a requested MOEX instrument is malformed or unsupported, **When** its history or quote is requested, **Then** the standard documented invalid-symbol response is returned.
3. **Given** MOEX ISS is unavailable or returns unusable data, **When** any supported instrument category is requested, **Then** the standard MOEX dependency error is returned rather than valid empty data.

[Source: specs/007-moex-instrument-coverage/spec.md -> "Preserve existing MOEX behavior for other instruments"]

## Shared Freshness, Performance, and Operations

- **SHARED-FR-006** *(MOEX FR-013; SPBEX FR-013; CBR FR-014)*: Unexpired history MUST survive application restarts. Expired history MUST be refreshed before it is returned; the specification does not prescribe a storage mechanism or freshness duration. [Source: specs/001-moex-ticker-update/spec.md -> FR-013] [Source: specs/002-spbex-ticker-update/spec.md -> FR-013] [Source: specs/003-cbr-currency-rates/spec.md -> FR-014]
- **SHARED-FR-007** *(MOEX FR-014; SPBEX FR-008; CBR FR-010)*: Quote responses MUST be fetched for each request and MUST NOT be served from a prior quote response. [Source: specs/002-spbex-ticker-update/spec.md -> FR-008] [Source: specs/003-cbr-currency-rates/spec.md -> FR-010]
- **SHARED-FR-008** *(MOEX FR-012; SPBEX FR-012; CBR FR-017)*: Under 10 concurrent clients issuing 10 requests per second total, at least 95% of successful responses on each of the six routes MUST complete end to end in under one second. This is a hard acceptance gate; a miss requires optimization and another measurement. [Source: specs/002-spbex-ticker-update/spec.md -> FR-012] [Source: specs/003-cbr-currency-rates/spec.md -> FR-017]
- **SHARED-FR-009** *(MOEX FR-015; SPBEX FR-014; CBR FR-018)*: The release MUST successfully build a Linux amd64 container image. AMD64 acceptance requires build success only, not runtime or route testing. [Source: specs/003-cbr-currency-rates/spec.md -> FR-018]
- **SHARED-FR-010** *(MOEX FR-016; CBR FR-019)*: On Ctrl+C (SIGINT), the service MUST stop accepting new requests, allow in-flight work to finish, and exit within 30 seconds. Remaining work MUST be cancelled when the deadline expires. [Source: specs/001-moex-ticker-update/spec.md -> FR-016] [Source: specs/003-cbr-currency-rates/spec.md -> FR-019]
- **SHARED-FR-011** *(CBR FR-023, FR-025)*: Every application configuration environment variable MUST begin with `EXCHANGE_API_`. Shared settings MUST apply consistently across providers; source and response-size settings MAY be provider-specific. Current names are `EXCHANGE_API_LISTEN_ADDR`, `EXCHANGE_API_REQUEST_TIMEOUT_SECS`, `EXCHANGE_API_HISTORY_CACHE_MAX_BYTES`, `EXCHANGE_API_HISTORY_CACHE_DB_PATH`, `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS`, `EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS`, `EXCHANGE_API_MOEX_ISS_BASE_URL`, `EXCHANGE_API_MOEX_MAX_ISS_RESPONSE_BYTES`, `EXCHANGE_API_MOEX_MAX_HISTORY_BYTES`, `EXCHANGE_API_SPBEX_API_BASE_URL`, `EXCHANGE_API_SPBEX_MAX_RESPONSE_BYTES`, `EXCHANGE_API_CBR_API_BASE_URL`, `EXCHANGE_API_CBR_MAX_RESPONSE_BYTES`, and `EXCHANGE_API_LOG_COLOR`. `EXCHANGE_API_HISTORY_CACHE_TTL_SECS` is removed; if set, startup MUST fail with a migration message directing operators to `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS`. [Source: specs/004-history-cache-refresh/spec.md -> FR-014] [Source: specs/003-cbr-currency-rates/spec.md -> FR-023, FR-025] [Source: specs/006-log-coloring/spec.md -> FR-001]
- **SHARED-FR-012** *(project implementation behavior)*: `GET /health/live` reports process liveness. `GET /health/ready` reports readiness and returns HTTP 503 when the history store cannot be queried.
- **SHARED-FR-029**: Operators MUST be able to configure process-wide log coloring through `EXCHANGE_API_LOG_COLOR`. [Source: specs/006-log-coloring/spec.md -> FR-001]
- **SHARED-FR-030**: Log coloring MUST be enabled when `EXCHANGE_API_LOG_COLOR` is absent. [Source: specs/006-log-coloring/spec.md -> FR-002]
- **SHARED-FR-031**: Values `false`, `0`, `no`, and `off` MUST disable log coloring regardless of letter case; all other values MUST leave it enabled. [Source: specs/006-log-coloring/spec.md -> FR-003]
- **SHARED-FR-032**: When log coloring is disabled, logs MUST contain no color control sequences while preserving message content and severity information. [Source: specs/006-log-coloring/spec.md -> FR-004]

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
- **SHARED-FR-027**: The provider selector is a path-level request entity that identifies exactly one existing provider; the symbol is normalized and validated by that selected provider. [Source: specs/005-v2-history-quote-api/data-model.md -> Provider-Symbol Request]

## Key Entities

### History Collection

Represents retained daily history for one normalized symbol at one provider. Its fields are provider and symbol identity, dated records, latest record date, last successful full-refresh time, consecutive refresh-failure count, next attempt time, and update time. Records are unique by date, ordered by provider contract, and remain available until explicitly removed by an operator. [Source: specs/004-history-cache-refresh/data-model.md -> "History Collection"]

### History Record

Represents one provider's daily history row with required date identity and nullable `close`, `high`, `low`, `volume`, and `facevalue` values. Its identity is `(provider, symbol, date)`; successful source data replaces a matching record and adds unseen dates. For MOEX, the trading date is UTC midnight, values come from the board primary on that date, `volume` maps instrument units rather than turnover, and `facevalue` contains current board `LOTSIZE` (not `FACEVALUE` or historical LOTSIZE). For SPBEX, `date` preserves the Unix-seconds source timestamp in UTC; `close`, `high`, and `low` must be finite positive prices with `low <= close <= high`; `volume` is null and `facevalue` is `1`. For CBR, `date` is the effective date at UTC midnight, `close` is source `Value / Nominal` in RUB per unit (or null if either is absent), `facevalue` is source nominal, and unavailable high/low/volume fields are null. Duplicate identities or unusable provider data reject the source result. [Source: specs/004-history-cache-refresh/data-model.md -> "History Record"] [Source: specs/001-moex-ticker-update/data-model.md -> "Daily market record"] [Source: specs/002-spbex-ticker-update/data-model.md -> "Daily Market Record"] [Source: specs/003-cbr-currency-rates/data-model.md -> "Daily Currency Rate"]

### Ticker

Identifies a normalized MOEX instrument resolved against security and board metadata. The resolved context includes instrument category, engine, market, board assignments, primary-board status, effective dates, and optional LOTSIZE. Its primary board is determined for each history date; primary-board intervals require valid inclusive ordered dates, may have gaps or open ends, and must not overlap. The response `facevalue` uses current board-specific `LOTSIZE`, or null when unavailable. [Source: specs/001-moex-ticker-update/data-model.md -> "Ticker"] [Source: specs/007-moex-instrument-coverage/data-model.md -> "MOEX Instrument Context"] [Source: specs/007-moex-instrument-coverage/spec.md -> FR-014]

### Latest Trade Quote Record

Represents the latest executed MOEX trade at request time as one history-shaped record. Trade date/time is converted from Europe/Moscow to UTC; price maps to `close`, quantity to `volume`, and `high`, `low`, and `facevalue` are null. For MOEX currency instruments, source trade quantity is converted from lots to instrument units using the resolved board LOTSIZE. This record is request-scoped and is not persisted or cached. [Source: specs/001-moex-ticker-update/data-model.md -> "Latest trade quote record"] [Source: specs/007-moex-instrument-coverage/data-model.md -> "Current Value Record"]

### MOEX Benchmark Current Value Record

Represents the latest published benchmark value at request time as one history-shaped quote record. Its `close` value is the latest non-null `CURRENTVALUE`, falling back to `LASTVALUE`; unavailable fields are null. It is request-scoped and is not persisted or cached. [Source: specs/007-moex-instrument-coverage/data-model.md -> "Current Value Record"] [Source: specs/007-moex-instrument-coverage/research.md -> "Select quote value according to instrument category"]

### SPBEX Ticker

Identifies a SPBEX chart-feed instrument. Normalize its symbol by trimming surrounding whitespace, uppercasing ASCII letters, and validating `[A-Za-z0-9._-]+`; use the normalized symbol for requests and exchange-qualified storage identity. [Source: specs/002-spbex-ticker-update/data-model.md -> "SPBEX Ticker"]

### Latest SPBEX Quote Record

Represents the latest available SPBEX daily candle fetched for a quote request. It is returned as a one-element history-shaped array, is not read from or written to history storage, and has null market fields when the full available chart range contains no candles. Unlike the MOEX quote entity, this is a daily candle, not an executed trade. [Source: specs/002-spbex-ticker-update/data-model.md -> "Latest Quote Record"]

### Supported Currency

Identifies a currency by its public three-letter code, normalized by trimming and uppercasing and validated against the Bank of Russia's current supported-currency directory. The source's internal identifier is used for history requests and is not exposed by the API. [Source: specs/003-cbr-currency-rates/data-model.md -> Supported Currency]

### Latest CBR Quote Record

Represents the latest official Bank of Russia rate fetched for a quote request. It is a one-element history-shaped response, carries the source's effective date and normalized RUB-per-unit rate, and is not read from or written to history storage. A successful lookup with no rate returns all six fields as null. [Source: specs/003-cbr-currency-rates/data-model.md -> Latest Quote]

### Log Color Setting

Represents the process-wide choice for ANSI formatting in the human-readable log formatter. It is read from `EXCHANGE_API_LOG_COLOR` at startup; absence enables coloring, and `false`, `0`, `no`, or `off` disables it case-insensitively. Other values enable coloring. The setting affects ANSI control sequences only and does not change log filtering, message content, severity, or persistence. [Source: specs/006-log-coloring/data-model.md -> Log Color Setting]

### Provider Selector

Identifies one existing data adapter selected by a v2 request: `moex`, `spbex`, or `cbr`. Other values are rejected with HTTP 400 and `invalid_provider`. [Source: specs/005-v2-history-quote-api/data-model.md -> Provider Selector]

### Provider-Symbol Request

Represents the provider selected by the v2 path and the caller-supplied symbol. The selected existing provider normalizes and validates the symbol; the same symbol text at another provider is a separate request target. [Source: specs/005-v2-history-quote-api/data-model.md -> Provider-Symbol Request]

## Edge Cases

- Disabling log colors removes ANSI color and formatting control sequences without suppressing or altering log messages. [Source: specs/006-log-coloring/spec.md -> Edge Cases]

- V2 provider values are limited to `moex`, `spbex`, and `cbr`; an unsupported value returns HTTP 400 with `invalid_provider` and never selects another provider. [Source: specs/005-v2-history-quote-api/spec.md -> Edge Cases]
- The same symbol may be valid at multiple providers; the v2 provider segment alone selects the intended provider's history or quote. [Source: specs/005-v2-history-quote-api/spec.md -> Edge Cases]
- Existing provider-specific v1 routes remain available during the v2 migration. [Source: specs/005-v2-history-quote-api/spec.md -> Edge Cases]
- A failed source fetch during a user request returns the documented provider-specific upstream error; cached data is not reported as a successful refresh. SPBEX oversized, truncated, malformed, unsuccessful, or unusable responses are not treated as valid empty data. [Source: specs/004-history-cache-refresh/spec.md -> "If the source request fails"] [Source: specs/002-spbex-ticker-update/spec.md -> FR-011] [Source: specs/002-spbex-ticker-update/spec.md -> "The SPBEX response is too large, truncated, malformed, or unsuccessful"]
- A history-store read or update failure returns the documented history-store error, including HTTP 503 for SPBEX history. A failed targeted MOEX cache migration retains its legacy row and returns the history-store error. [Source: specs/004-history-cache-refresh/spec.md -> "If the history store cannot be queried"] [Source: specs/002-spbex-ticker-update/spec.md -> FR-015] [Source: specs/007-moex-instrument-coverage/spec.md -> FR-013]
- Retryable background failures preserve last-good history and retry with configured backoff; other HTTP 4xx responses wait until the next normal schedule. [Source: specs/004-history-cache-refresh/spec.md -> "If a background full refresh receives a network failure"]
- A full refresh validates all pages and records before commit. Failed pages, missing or invalid dates, unusable records, or any duplicate identity reject the refresh without partial updates. [Source: specs/004-history-cache-refresh/spec.md -> "A full refresh MUST validate the complete response"]
- A successful empty source response for a valid MOEX equity, benchmark, or currency instrument returns the documented empty-history result; previously retained records remain available. For SPBEX, a successful empty chart feed does not establish that a syntactically valid symbol is unsupported. [Source: specs/004-history-cache-refresh/spec.md -> "If a successful source response contains no history"] [Source: specs/002-spbex-ticker-update/spec.md -> FR-006, FR-010] [Source: specs/007-moex-instrument-coverage/spec.md -> FR-007]
- Repeated requests do not create duplicate records for the same provider, symbol, and source identity. [Source: specs/004-history-cache-refresh/spec.md -> "Repeated requests must not create duplicate entries"]
- A malformed or unsupported MOEX symbol returns the standard client error; valid symbols with no history return `[]`. Newly supported MOEX benchmarks and currency instruments may use single underscores between alphanumeric parts, but not leading, trailing, or repeated underscores; existing symbol validation behavior is retained. SPBEX trims surrounding whitespace and uppercases symbols before syntax validation. [Source: specs/001-moex-ticker-update/spec.md -> FR-007, FR-008] [Source: specs/002-spbex-ticker-update/spec.md -> "A symbol has whitespace or lowercase letters"] [Source: specs/007-moex-instrument-coverage/spec.md -> FR-003]
- MOEX dates retain their trading date at UTC midnight; nullable market values remain null, but a missing/invalid date or unresolved primary board makes the response unusable. Primary-board intervals require valid ordered inclusive dates, may have gaps or open ends, and must not overlap. [Source: specs/001-moex-ticker-update/spec.md -> FR-004, FR-011] [Source: specs/007-moex-instrument-coverage/spec.md -> FR-014]
- A valid empty MOEX trades or benchmark-current-value response returns the all-null quote record; malformed or failed trade/current-value data returns a dependency error. Index quotes represent the latest available published value (`CURRENTVALUE`, falling back to `LASTVALUE`) rather than an executed trade; currency quotes represent the latest currency-market trade, with `QTY` converted using board LOTSIZE. Existing equity quote behavior remains unchanged. [Source: specs/001-moex-ticker-update/spec.md -> FR-014] [Source: specs/007-moex-instrument-coverage/spec.md -> FR-002, FR-006, FR-010] [Source: specs/007-moex-instrument-coverage/data-model.md -> "Current Value Record"]
- For a newly supported MOEX history symbol, metadata recognition occurs before legacy-history migration; unknown symbols and metadata failures leave that cache untouched. A failed migration retains the legacy row and reports a store error. [Source: specs/007-moex-instrument-coverage/spec.md -> "A previously fetched symbol has cached history when its request passes syntax validation and MOEX metadata confirms the symbol is recognized"]
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
- **SHARED-SC-010**: MOEX equity, index, and currency history returns all available public records from the primary board for each date, with correct UTC dates and the six-field mapping; SPBEX history returns six-field records sorted oldest to newest with source timestamps represented in UTC; CBR history returns source-available daily rates oldest first with correct UTC effective dates and normalized RUB-per-unit values. Valid empty history is `[]`. [Source: specs/001-moex-ticker-update/spec.md -> SC-002, SC-003] [Source: specs/002-spbex-ticker-update/spec.md -> SC-001] [Source: specs/003-cbr-currency-rates/spec.md -> SC-001] [Source: specs/007-moex-instrument-coverage/spec.md -> SC-001, SC-003]
- **SHARED-SC-011**: Invalid symbols, unavailable history, malformed/unavailable quote data, valid empty responses, and history-store failures produce distinguishable outcomes matching their documented contracts for every supported MOEX instrument category and the other providers. [Source: specs/001-moex-ticker-update/spec.md -> SC-004] [Source: specs/002-spbex-ticker-update/spec.md -> SC-005] [Source: specs/003-cbr-currency-rates/spec.md -> SC-006] [Source: specs/007-moex-instrument-coverage/spec.md -> SC-004, SC-005]
- **SHARED-SC-012**: A MOEX quote request fetches a fresh current value and maps it to the one-record response: an executed trade for traded instruments or the latest published value for an index. A valid lookup with no trade or current value returns the all-null record. A SPBEX quote request fetches current chart data again on a subsequent request, so an upstream change is reflected. CBR quote responses use the latest official rate available at request time and include its effective date. [Source: specs/001-moex-ticker-update/spec.md -> SC-006] [Source: specs/002-spbex-ticker-update/spec.md -> SC-003] [Source: specs/003-cbr-currency-rates/spec.md -> SC-003] [Source: specs/007-moex-instrument-coverage/spec.md -> SC-002, SC-003]
- **SHARED-SC-013**: At least 95% of successful responses from all six MOEX, SPBEX, and CBR history and quote routes complete end to end in under one second at 10 concurrent clients and 10 total requests per second. SPBEX history-only, quote-only, and combined profiles must each sustain the full rate and meet the gate. [Source: specs/002-spbex-ticker-update/spec.md -> SC-002] [Source: specs/003-cbr-currency-rates/spec.md -> SC-004]
- **SHARED-SC-014**: Every numeric CBR `close` is RUB per one currency unit, regardless of source nominal, and unavailable fields are JSON null. [Source: specs/003-cbr-currency-rates/spec.md -> SC-002]
- **SHARED-SC-015**: A Linux amd64 container image builds successfully; amd64 runtime startup and route checks are not required. [Source: specs/003-cbr-currency-rates/spec.md -> SC-007]
- **SHARED-SC-016**: After SIGINT, the service stops accepting requests and exits within 30 seconds, draining in-flight work where possible and cancelling what remains at the deadline. [Source: specs/003-cbr-currency-rates/spec.md -> SC-008]
- **SHARED-SC-017**: MOEX, SPBEX, and CBR expose the same documented history and quote response structure, symbol handling, and error outcomes while preserving provider-specific quote meanings: MOEX trades for equities and currencies, published values for indices, SPBEX daily candles, and CBR official rates. [Source: specs/003-cbr-currency-rates/spec.md -> SC-009] [Source: specs/007-moex-instrument-coverage/spec.md -> SC-002, SC-004]
- **SHARED-SC-018**: Shared runtime settings affect every provider consistently and provider-specific source settings remain independently configurable. [Source: specs/003-cbr-currency-rates/spec.md -> SC-010]
- **SHARED-SC-019**: Every documented and accepted application configuration environment variable begins with `EXCHANGE_API_`, including provider-specific settings. [Source: specs/003-cbr-currency-rates/spec.md -> SC-011]
- **SHARED-SC-020**: Each of the six v2 provider history and quote routes meets the requirement that at least 95% of successful responses finish in under one second at 10 total requests per second with 10 concurrent clients. MOEX history's initial uncached full fetch is reported separately; its gated profile runs after full history is cached. [Source: specs/005-v2-history-quote-api/spec.md -> SC-005]
- **SHARED-SC-021**: With both history refresh settings unset, all six v2 provider history and quote routes respond according to their contracts, using the seven-day full-refresh interval and 900-second retry-backoff defaults. [Source: specs/005-v2-history-quote-api/spec.md -> SC-006]
- **SHARED-SC-022**: With coloring disabled, all sampled log messages contain no color control sequences. [Source: specs/006-log-coloring/spec.md -> SC-001]
- **SHARED-SC-023**: With the log-color setting absent or enabled, colored output matches the current default behavior. [Source: specs/006-log-coloring/spec.md -> SC-002]
- **SHARED-SC-024**: Disabling coloring preserves all sampled log messages and their severity information. [Source: specs/006-log-coloring/spec.md -> SC-003]
- **SHARED-SC-025**: Operators can identify the log-color setting, its default, and disabling values in application configuration documentation. [Source: specs/006-log-coloring/spec.md -> SC-004]
- **SHARED-SC-027**: On the first history request for a newly supported MOEX benchmark or currency symbol, a recognized symbol's legacy records are retained and combined with fetched history; unknown symbols and metadata failures do not migrate data, and failed migration preserves the legacy row. [Source: specs/007-moex-instrument-coverage/spec.md -> SC-006, FR-013]

## Assumptions

- **AS-001**: Indefinite retention excludes deliberate operator deletion or loss of underlying storage. [Source: specs/004-history-cache-refresh/spec.md -> "Lives forever"]
- **AS-002**: User requests fetch records newer than the latest retained date; scheduled full refreshes reconcile older revisions. [Source: specs/004-history-cache-refresh/spec.md -> "User-request refreshes"]
- **AS-003**: The full-refresh interval accepts any positive number of seconds and defaults to 604800 seconds. [Source: specs/004-history-cache-refresh/spec.md -> "configurable full-refresh interval"] [Source: specs/005-v2-history-quote-api/spec.md -> "The full-refresh interval and retry-backoff settings default to seven days and 900 seconds"]
- **AS-004**: Retryable failures use persisted exponential backoff from one second to the configured cap, default 900 seconds. [Source: specs/004-history-cache-refresh/spec.md -> "Retryable background failures"] [Source: specs/005-v2-history-quote-api/spec.md -> "The full-refresh interval and retry-backoff settings default to seven days and 900 seconds"]
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
- **AS-023**: Providers share the six-field record and route behavior while retaining distinct quote meanings: MOEX trades for equities and currencies, published values for indices, SPBEX daily candles, and CBR official rates. [Source: specs/003-cbr-currency-rates/spec.md -> "Shared route behavior follows"] [Source: specs/007-moex-instrument-coverage/spec.md -> "MOEX benchmarks and MOEX-traded currency instruments are MOEX instruments and belong under the existing `moex` provider"]
- **AS-024**: Application configuration uses `EXCHANGE_API_`-prefixed environment variables, including provider-specific settings. [Source: specs/003-cbr-currency-rates/spec.md -> "All application configuration is supplied"]
- **AS-025**: V2 changes route organization only; provider mappings, history lifecycle, response records, and error contracts remain unchanged. The added MOEX categories use the existing v1/v2 routes and six-field response schema. [Source: specs/005-v2-history-quote-api/spec.md -> Assumptions] [Source: specs/007-moex-instrument-coverage/spec.md -> "Currency symbols with underscores are valid MOEX symbols; underscore support must not broaden acceptance of otherwise malformed symbols"]
- **AS-026**: Provider-specific v1 routes remain available during the v2 migration to preserve compatibility. [Source: specs/005-v2-history-quote-api/spec.md -> Assumptions]
- **AS-027**: V2 route templates are `/v2/history/{PROVIDER}/{SYMBOL}` and `/v2/quote/{PROVIDER}/{SYMBOL}`; braces mark path parameters. [Source: specs/005-v2-history-quote-api/spec.md -> Assumptions]
- **AS-028**: The log-color setting is process-wide and applied when the application starts. [Source: specs/006-log-coloring/spec.md -> "The setting is process-wide and is applied when the application starts"]
- **AS-029**: Colored log output is the compatible default for existing deployments. [Source: specs/006-log-coloring/spec.md -> "The application currently emits colored logs by default"]
- **AS-030**: Feature completion includes the final container build, Semgrep source analysis, and Trivy image scan; outcomes and unavailable scans or fixes are recorded in the quickstart. [Source: specs/006-log-coloring/spec.md -> "Feature completion includes the constitution-required final container build"]
- **AS-031**: MOEX benchmarks and MOEX-traded currency instruments belong to the existing MOEX provider; CBR remains the provider for official Bank of Russia rates. [Source: specs/007-moex-instrument-coverage/spec.md -> "MOEX benchmarks and MOEX-traded currency instruments are MOEX instruments and belong under the existing `moex` provider"]
- **AS-034**: For newly supported MOEX categories, underscores separate alphanumeric symbol parts and do not make leading, trailing, or repeated underscores valid; existing MOEX validation behavior remains. [Source: specs/007-moex-instrument-coverage/spec.md -> "Currency symbols with underscores are valid MOEX symbols; underscore support must not broaden acceptance of otherwise malformed symbols"]
- **AS-036**: Existing MOEX normalization, primary-board selection, persistence, error, performance, and operational requirements continue to apply to added instrument categories. [Source: specs/007-moex-instrument-coverage/spec.md -> "Existing MOEX normalization, primary-board selection where applicable, persistence, error, performance, and operational requirements continue to apply"]

## Implementation Comparison and Open Contradictions

Source review found no behavioral mismatch between the current implementation and the newest applicable feature requirements. It has all six routes, a shared provider interface and record shape, normalized symbols, the specified status/error mappings, persistent history caching, uncached quote fetches, provider-specific field mappings, `EXCHANGE_API_*` configuration, and bounded SIGINT shutdown. Source also exposes the two health routes described above. No implementation changes or tests were run for this consolidation.

The feature quickstarts record final validation on 2026-10-09, including Linux amd64 build success and all six route p95 values below one second under the specified workload: MOEX history/quote p95 were 0.002227/0.398614 seconds; SPBEX 0.003834/0.751137 seconds; CBR 0.003090/0.042939 seconds. Combined-profile route p95 values were also below one second. These are recorded results, not measurements repeated during this consolidation.

1. **SPBEX date boundary:** SPBEX FR-016 says “current calendar date” without a timezone. The OpenAPI contract and implementation use the current UTC date. If “calendar date” means the exchange-local date, the specification and behavior may differ; the intended timezone is not explicit.
Historical feature specifications and their plans, tasks, and contracts remain unchanged.
