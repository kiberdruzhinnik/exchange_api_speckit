# Feature Specification: CBR Currency Rates API

**Feature Branch**: `main`

**Created**: 2026-10-09

**Status**: Draft

**Input**: User description: "make another route /v1/cbr/<SYMBOL> which fetches history of currency from central bank of russia and /v1/cbr/<SYMBOL/quote which fetches current rate for today. <SYMBOL> must be one of supported currencies from central bank of russia, example: usd, cny, eur and etc."

## Clarifications

### Session 2026-10-09

- Q: What response fields and fallback values should the CBR routes use? → A: Return `date`, `close`, `high`, `low`, `volume`, and `facevalue`; use `null` for fields unavailable from the source.
- Q: How should rates quoted for multiple currency units be represented? → A: Normalize every rate to Russian rubles per one currency unit; for example, 100 units = 5 rubles becomes `close: 0.05`.
- Q: If a CBR history row omits the source value or nominal needed to calculate `close`, should `close` be `null` while a missing or invalid date makes the upstream response an error? → A: Yes. `close` is null when `Value` or `Nominal` is absent; a missing or invalid effective date makes the source response unusable and returns HTTP 502.
- Q: How should the CBR history request choose its date range to return all available history? → A: Request the currency's entire source-available date range through the latest published date.
- Q: How broadly should CBR be aligned with MOEX and SPBEX? → A: Align shared API and service requirements, including the response and error contracts, durable history, fresh quotes, performance target, documentation, Linux amd64 image build, and SIGINT shutdown; preserve CBR-specific latest-official-rate quote semantics.
- Q: Should provider unification cover the public REST contract and common provider behavior, while retaining each provider’s quote meaning? → A: Unify the public REST contract and provider behavior, while retaining provider-specific quote semantics.
- Q: For the shared REST contract, should all providers use the same symbol normalization, HTTP status categories, empty-history response, and no-quote response? → A: Yes. Trim and uppercase symbols; return 400 for invalid or unsupported symbols, 502 for upstream failures, and 503 for history-store failures; return `[]` for valid empty history and one all-null record for a successful no-quote result.
- Q: Should shared runtime settings use one common naming scheme and remove the former MOEX-specific names rather than retain aliases? → A: Use a common naming scheme for shared settings and remove the former names without aliases.
- Q: How should existing provider-specific upstream error codes be handled while standardizing the public error contract? → A: Preserve each provider’s existing upstream error code on its current `/v1` routes; standardize the JSON error envelope and HTTP status behavior so existing clients are not broken.
- Q: Should the one-second p95 acceptance target apply to CBR routes only or to history and quote routes for all three providers? → A: The hard p95 target applies to all six MOEX, SPBEX, and CBR history and quote routes.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Retrieve a supported currency's rate history (Priority: P1)

A client requests the history for a currency code supported by the Bank of Russia and receives the official daily ruble exchange rates in chronological order.

**Why this priority**: Historical rates are the primary capability requested and let clients analyze currency changes over time.

**Independent Test**: Request history for a supported code such as USD, CNY, or EUR and compare its dates and normalized rates with the Bank of Russia's published history.

**Acceptance Scenarios**:

1. **Given** a supported currency with published rate history, **When** a client requests `/v1/cbr/{SYMBOL}`, **Then** the service returns an array of six-field rate records ordered from oldest to newest.
2. **Given** the official source expresses a rate for multiple units of a currency, **When** history is returned, **Then** each rate is normalized to Russian rubles for one unit.
3. **Given** a supported currency has no history in the requested source data, **When** a client requests history, **Then** the service returns an empty JSON array.

### User Story 2 - Retrieve the current official currency rate (Priority: P2)

A client requests the current Bank of Russia rate for a supported currency and receives the latest official rate available for the current date.

**Why this priority**: Clients need a direct current-rate route without fetching and scanning the full history.

**Independent Test**: Request a quote for a supported currency and verify the returned date and rate match the latest official Bank of Russia rate available at request time.

**Acceptance Scenarios**:

1. **Given** the Bank of Russia has published a rate for the current date, **When** a client requests `/v1/cbr/{SYMBOL}/quote`, **Then** the service returns a one-element array with that date and normalized rate in the six-field record shape.
2. **Given** no rate has been published for the current calendar date, **When** a client requests the quote, **Then** the service returns the latest published rate and its effective date.
3. **Given** a supported currency has no available rate in a successful source response, **When** a client requests the quote, **Then** the service returns one record with all six fields set to null.

### User Story 3 - Handle unsupported currencies and source failures (Priority: P2)

A client can distinguish invalid input from temporary source failures, and all provider routes share consistent contracts and configuration.

**Why this priority**: Clear errors prevent clients from treating invalid input or unavailable rates as valid market data. Shared provider contracts and configuration also make the service predictable across exchanges.

**Independent Test**: Request malformed and unsupported codes and simulate failed or unusable source responses; verify each has its documented outcome.

**Acceptance Scenarios**:

1. **Given** a malformed code or a code absent from the Bank of Russia's supported-currency list, **When** a client requests either route, **Then** the service returns HTTP `400` in the standard JSON error format.
2. **Given** the Bank of Russia source is unavailable or returns malformed or unusable data, **When** a client requests either route, **Then** the service returns HTTP `502` in the standard JSON error format.
3. **Given** a history-store operation fails while serving currency history, **When** a client requests history, **Then** the service returns HTTP `503` in the standard JSON error format.
4. **Given** a client sends a lowercase symbol with surrounding whitespace to any provider route, **When** the request is processed, **Then** the provider receives the trimmed uppercase symbol; malformed or unsupported symbols receive the shared HTTP `400` outcome.
5. **Given** any provider returns successful empty history or no quote, **When** the corresponding route is requested, **Then** history returns `[]` or quote returns one all-null record, while each provider retains its documented quote meaning.
6. **Given** all three providers run with shared configuration, **When** the common runtime settings are configured, **Then** the listener, upstream timeout, history-cache TTL, capacity, and database path are shared; provider-specific source URL and response-size settings remain provider-specific.
7. **Given** a provider handles history or quote operations, **When** it is used by the service, **Then** it supports the common history and quote behavior and returns the shared normalized records and error outcomes.

### Edge Cases

- Currency codes are case-insensitive and normalized to uppercase; surrounding whitespace is trimmed.
- The Bank of Russia publishes no new rate on a calendar date; quote returns the latest published rate with its actual effective date.
- A source rate is quoted for multiple units (for example, 100 units = 5 rubles); the API returns the per-unit value (0.05 rubles).
- A history row lacks `Value` or `Nominal`; `close` is null, but a missing or invalid effective date makes the source response unusable.
- CBR does not supply high, low, or volume; these fields are returned as `null`.
- CBR's nominal quantity is returned as `facevalue` when supplied; if unavailable, `facevalue` is `null`.
- A valid source response has no history rows or no current rate for the currency.
- The currency code is syntactically valid but is not currently supported by the Bank of Russia.
- The source response is unavailable, malformed, oversized, or contains invalid dates or non-positive rates.
- Providers receive the same normalized symbol and return the same validation, dependency, history-store, empty-history, and no-quote outcomes.
- Persistent history cannot be read or saved.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST expose currency rate history at `GET /v1/cbr/{SYMBOL}`.
- **FR-002**: The system MUST expose the latest official currency rate at `GET /v1/cbr/{SYMBOL}/quote`.
- **FR-003**: `SYMBOL` MUST identify a currency in the Bank of Russia's supported-currency list; codes MUST be accepted case-insensitively and normalized to uppercase.
- **FR-004**: The system MUST retrieve official rates and supported currency identifiers from the Bank of Russia's public currency-rate service. The Bank of Russia documents daily rates and historical rate dynamics through its [official XML interface](https://www.cbr.ru/development/SXML/).
- **FR-005**: History MUST return all daily rates available from the CBR source for the supported currency, from its earliest source-available date through the latest published date, ordered from oldest to newest, as an array of records with exactly `date`, `close`, `high`, `low`, `volume`, and `facevalue` fields.
- **FR-006**: `date` MUST represent the source rate's effective date as an ISO 8601 UTC timestamp at midnight; a missing or invalid effective date MUST make the source response unusable and return HTTP `502`. `close` MUST be the normalized numeric value in Russian rubles per one unit of the requested currency when the source provides the values needed to calculate it; otherwise it MUST be `null`. `high`, `low`, `volume`, and `facevalue` MUST be numeric or `null`.
- **FR-007**: When the source expresses a rate as a value for `Nominal` currency units, `close` MUST equal the source value divided by `Nominal` so every returned close is Russian rubles per one currency unit. For example, a source value of 5 rubles for 100 units MUST produce `close: 0.05`. `facevalue` MUST equal the source nominal quantity when present and MUST otherwise be `null`.
- **FR-007a**: If the source does not provide a value for a response field other than the required effective date, that field MUST be serialized as JSON `null`; unavailable CBR high, low, and volume values MUST be `null`.
- **FR-008**: A supported currency with successful empty history MUST return an empty JSON array.
- **FR-009**: The quote route MUST return a one-element array with the same six-field record shape as history, using the latest official rate available at request time. If no rate is published for today's calendar date, its actual effective date MAY be earlier than today.
- **FR-010**: Every quote request MUST obtain a fresh rate from the Bank of Russia source and MUST NOT reuse a prior quote response.
- **FR-011**: If a successful source response has no rate for a supported currency, the quote route MUST return one record with all six fields set to `null`.
- **FR-012**: Malformed or unsupported currency codes MUST return HTTP `400` using the standard JSON error format.
- **FR-013**: Source transport failures, unsuccessful responses, oversized or malformed payloads, and unusable rate records MUST return HTTP `502` using the standard JSON dependency-error format.
- **FR-014**: Currency history MUST survive application restarts for its applicable freshness period; expired history MUST be refreshed before it is returned.
- **FR-015**: If the history response cannot be read from or saved to the persistent history store, the history route MUST return HTTP `503` in the standard JSON error format, distinct from a Bank of Russia dependency error.
- **FR-016**: API documentation MUST describe both routes, supported-currency validation, normalization, six-field response shape and units, null values for unavailable source fields, effective-date behavior, empty results, and client, dependency, and history-store errors, consistent with the shared MOEX and SPBEX API conventions.
- **FR-017**: Under a workload of 10 concurrent clients issuing 10 requests per second total, at least 95% of successful responses from each of the six MOEX, SPBEX, and CBR history and quote routes MUST complete in under one second end to end. This is a hard acceptance gate; if any route misses it, the request/source path MUST be optimized and measured again before the feature is complete.
- **FR-018**: The release process MUST build a Linux amd64 container image. AMD64 acceptance MUST require successful image build only; starting the image or exercising service routes on amd64 is not required.
- **FR-019**: When the service receives Ctrl+C (SIGINT), it MUST stop accepting new requests, allow in-flight work to finish, and terminate within 30 seconds. Any work still in flight at the deadline MUST be canceled.
- **FR-020**: MOEX, SPBEX, and CBR MUST provide the same history and quote capabilities using normalized records and shared error outcomes; each provider MUST preserve its documented source mapping and quote meaning.
- **FR-021**: The public history and quote routes for MOEX, SPBEX, and CBR MUST follow one shared REST contract for route pattern, six-field JSON record shape, array behavior, and JSON error envelope; provider-specific upstream mappings and quote semantics MUST remain documented.
- **FR-022**: All providers MUST trim and uppercase symbols, return HTTP 400 for malformed or unsupported symbols, HTTP 502 for upstream failures, and HTTP 503 for history-store failures. Valid empty history MUST return `[]`; a successful quote lookup without a quote MUST return one record with all six fields null. Each provider MUST preserve its established upstream error code on its current `/v1` routes while using the shared JSON error envelope and status categories.
- **FR-023**: Shared runtime settings for the listener, upstream timeout, history-cache freshness, capacity, and storage location MUST apply consistently to all providers. Provider-specific upstream source and response-size settings MAY vary by provider.
- **FR-024**: Standardizing provider error behavior MUST NOT rename existing upstream error codes on current `/v1` routes; provider-specific codes MUST map to the common HTTP 502 upstream-failure category and standard JSON error envelope.

### Key Entities *(include if data involved)*

- **Supported currency**: A currency identified by its Bank of Russia character code and present in the official supported-currency list.
- **Daily currency rate**: The official effective date and normalized Russian-ruble close for one unit of a supported currency, with high, low, volume, and facevalue values when supplied.
- **Latest currency quote**: A one-record response containing the newest official rate available when the quote request is made, using the same fields as history.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Clients can retrieve all available daily rates for USD, CNY, EUR, and any other supported code through the documented history route.
- **SC-002**: Every returned numeric `close` is expressed in Russian rubles per one unit of the requested currency, regardless of the source nominal quantity; unavailable fields are represented as JSON `null`.
- **SC-003**: Quote responses reflect the latest official rate available at request time and include the rate's effective date.
- **SC-004**: At least 95% of successful responses from each of the six MOEX, SPBEX, and CBR history and quote routes complete in under one second under the 10-client, 10-request-per-second workload.
- **SC-005**: History remains available across application restarts for its configured freshness period and is refreshed after expiration.
- **SC-006**: Unsupported codes, valid empty history, absent quote data, source failures, and history-store failures produce distinguishable documented outcomes.
- **SC-007**: A Linux amd64 container image builds successfully; amd64 runtime startup and route checks are outside acceptance scope.
- **SC-008**: After Ctrl+C is sent to a running service, it stops accepting new requests and exits within 30 seconds, completing in-flight work where possible and canceling work remaining at the deadline.
- **SC-009**: MOEX, SPBEX, and CBR expose the same documented history and quote response structure, symbol handling, and error outcomes, while each quote continues to represent its provider-specific data.
- **SC-010**: Shared runtime settings affect all provider routes consistently; provider-specific source settings can be configured independently.

## Assumptions

- `CBR` refers to the Central Bank of Russia, officially named the Bank of Russia.
- The canonical paths use `{SYMBOL}` without angle brackets; the quote path is `/v1/cbr/{SYMBOL}/quote`.
- The user intends the `close` value to mean Russian rubles per one unit of foreign currency. Source rates expressed per multiple units are normalized to one unit, and the source nominal is exposed as `facevalue` when present.
- The rate's source effective date is returned. On weekends, holidays, or before a new rate is published, the quote uses the latest available official rate and does not invent a rate for today's date.
- Currency support is determined from the Bank of Russia's current public currency list rather than a hard-coded list, so changes to supported currencies are reflected.
- Shared route behavior follows the MOEX and SPBEX conventions: the six-field history-shaped JSON records, standard JSON error envelope, durable history, fresh uncached quotes, and common performance and service acceptance requirements. Provider-specific data mapping remains distinct: CBR quotes return the latest official rate, MOEX quotes return the latest executed trade, and SPBEX quotes return the latest available daily candle.
- The existing application is intended for a private network and does not require application-level authentication for these routes.
