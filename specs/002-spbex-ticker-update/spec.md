# Feature Specification: SPBEX Ticker History and Quote API

**Feature Branch**: `002-spbex-ticker-update`

**Created**: 2026-10-08

**Status**: Draft

**Input**: User description: "implement SPBEX ticker update from SYMBOL from /v1/sbpex/<SYMBOL> REST API, which should be reused from https://github.com/kiberdruzhinnik/go-exchange-api/blob/main/api/spbex.go and return something like JSON: [{\"date\":\"2013-03-25T00:00:00Z\",\"close\":73.35,\"high\":75.05,\"low\":73.21,\"volume\":120300,\"facevalue\":1},...]. make sure to fetch latest quote via /v1/spbex/<SYMBOL>/quote like in MOEX"

## Clarifications

### Session 2026-10-08

- Q: Should `/v1/spbex/{SYMBOL}/quote` return the latest daily candle, matching `GetCurrentPrice` in the Go reference, or the most recent executed trade, matching MOEX quote semantics? → A: Return the latest available daily candle, matching `GetCurrentPrice` in the Go reference.
- Q: When the SPBEX chart feed provides no volume, should the API return `null` or `0` to preserve the referenced Go response? → A: Return `null` to indicate that volume is unavailable.
- Q: How should the API distinguish an unsupported SPBEX symbol from a valid symbol whose chart feed is empty? → A: Treat a successful empty feed as valid empty data; return HTTP 400 only for malformed symbols or explicit upstream rejection.
- Q: If the production load test misses the one-second p95 target for successful responses, should the feature remain incomplete until it passes? → A: Yes. The p95 target is a hard acceptance gate; optimize the request/source path and repeat measurement until it passes.
- Q: What should the history route return when the persistent history store cannot be read or written? → A: Return HTTP 503 with the standard history-store error, distinct from an SPBEX failure.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Retrieve an SPBEX ticker's daily history (Priority: P1)

A client requests the history of an SPBEX instrument by symbol and receives dated market records in the same six-field JSON format used by the MOEX API.

**Why this priority**: Historical price retrieval is the primary purpose of the feature and is required independently of current-quote requests.

**Independent Test**: Request a known SPBEX symbol against a controlled SPBEX chart-feed response and verify the returned records' fields, values, UTC dates, and ascending order.

**Acceptance Scenarios**:

1. **Given** a syntactically valid symbol accepted by SPBEX with available daily candles, **When** a client requests `/v1/spbex/{SYMBOL}`, **Then** the service returns a JSON array of records containing `date`, `close`, `high`, `low`, `volume`, and `facevalue`.
2. **Given** source candles are out of chronological order, **When** the client requests history, **Then** the records are returned in ascending date order and each source timestamp is represented in UTC.
3. **Given** a syntactically valid symbol with a successful empty feed, **When** the client requests history, **Then** the service returns an empty JSON array.

### User Story 2 - Retrieve the latest SPBEX quote (Priority: P2)

A client requests the latest available SPBEX quote for a symbol using a dedicated route and receives a one-record array with the same fields as a history record.

**Why this priority**: Clients need an explicit way to refresh the current value without parsing the full history response.

**Independent Test**: Request a quote using a controlled upstream response and verify it reflects the latest available value, follows the agreed one-record response shape, and is fetched again on a subsequent request.

**Acceptance Scenarios**:

1. **Given** a syntactically valid symbol with current SPBEX data, **When** a client requests `/v1/spbex/{SYMBOL}/quote`, **Then** the service returns a one-element array using the same six-field record shape as history.
2. **Given** the upstream quote data changes between requests, **When** the client requests the quote twice, **Then** the second response reflects a new upstream fetch rather than a previously cached quote.
3. **Given** a syntactically valid symbol with no quote data available, **When** the client requests the quote, **Then** the service returns the documented no-quote result rather than treating a successful empty response as an upstream outage.

### User Story 3 - Distinguish invalid symbols from SPBEX failures (Priority: P2)

A client receives a documented error when its symbol is malformed or explicitly rejected by SPBEX, and a dependency error when SPBEX cannot provide usable data.

**Why this priority**: Consumers need to distinguish a request problem from an exchange dependency failure.

**Independent Test**: Submit malformed symbols and simulate unsuccessful, malformed, and unusable SPBEX responses; verify each outcome matches the documented error contract.

**Acceptance Scenarios**:

1. **Given** a malformed symbol or an explicit upstream rejection of the symbol, **When** a client requests history or quote, **Then** the service returns HTTP `400` in the standard error format.
2. **Given** a syntactically valid symbol and a successful empty chart feed, **When** a client requests history or quote, **Then** the service returns the documented empty-history or no-quote result.
3. **Given** SPBEX is unavailable or returns unsuccessful or malformed data, **When** a client requests history or quote, **Then** the service returns the documented dependency error and does not present the failure as empty market data.
4. **Given** the persistent history store cannot be read or written, **When** a client requests history, **Then** the service returns HTTP `503` in the standard history-store error format.

### Edge Cases

- A symbol has whitespace or lowercase letters; the symbol is normalized consistently with the referenced SPBEX behavior.
- SPBEX returns no candles for a valid symbol.
- A candle has an invalid or duplicate timestamp, missing or non-finite prices, or inconsistent high, low, and close values.
- The SPBEX response is too large, truncated, malformed, or unsuccessful.
- The persistent history store is unavailable while serving or saving history.
- The chart feed omits trading volume; the API MUST return `null` for `volume` to indicate unavailable data.
- The quote is the latest available daily candle, not a most-recent executed trade.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST expose daily SPBEX history at `GET /v1/spbex/{SYMBOL}`. The canonical path uses `spbex` consistently with the requested quote path; the `sbpex` spelling in the request is treated as a typo.
- **FR-002**: The system MUST obtain SPBEX daily price data from the public chart feed behavior represented by [`api/spbex.go`](https://github.com/kiberdruzhinnik/go-exchange-api/blob/main/api/spbex.go), without exchange subscriber credentials.
- **FR-003**: A successful history response MUST be a JSON array of records with exactly the fields `date`, `close`, `high`, `low`, `volume`, and `facevalue`.
- **FR-004**: `date` MUST preserve the candle's source timestamp and be serialized as an ISO 8601 UTC timestamp. Records MUST be sorted from oldest to newest.
- **FR-005**: `close`, `high`, and `low` MUST preserve the source candle prices. `facevalue` MUST be `1`, matching the referenced SPBEX adapter's response behavior. Because the chart feed does not provide volume, `volume` MUST be `null` to indicate unavailable data.
- **FR-006**: A syntactically valid symbol with a successful feed response and no available history MUST return an empty JSON array.
- **FR-007**: The system MUST expose the latest SPBEX quote at `GET /v1/spbex/{SYMBOL}/quote`. It MUST return a one-element JSON array containing the latest available daily candle, with the same six fields as a history record.
- **FR-008**: The quote endpoint MUST fetch current SPBEX data on every request and MUST NOT reuse a previously fetched quote.
- **FR-009**: When SPBEX successfully reports no current quote, the endpoint MUST return a one-element record with nullable market fields rather than an upstream dependency error.
- **FR-010**: The system MUST return malformed symbols and symbols explicitly rejected by the upstream source as HTTP `400` in the standard JSON error format. A successful empty chart-feed response MUST be treated as valid empty data, not as an unsupported symbol.
- **FR-011**: The system MUST distinguish unsuccessful, malformed, oversized, or unusable SPBEX responses from valid empty data and return HTTP `502` in the standard dependency error format.
- **FR-012**: Under the project's normal workload of 10 concurrent clients issuing 10 requests per second total, at least 95% of successful responses from each SPBEX route MUST complete in under one second end to end. Meeting this target is a hard acceptance gate; if a measurement misses it, the request/source path MUST be optimized and measured again before the feature is complete.
- **FR-013**: Retrieved history MUST survive application restarts for its applicable freshness period. Expired history MUST be refreshed before being returned; the storage mechanism is not prescribed.
- **FR-014**: API documentation MUST describe both SPBEX routes, symbol normalization, response fields and types, ordering, empty-history and no-quote behavior, and client and dependency errors.
- **FR-015**: If the history response cannot be read from or persisted to the history store, the history endpoint MUST return HTTP `503` in the standard JSON error format with a history-store error distinct from the SPBEX dependency error.

### Key Entities

- **SPBEX ticker**: An instrument identified by its SPBEX symbol.
- **Daily candle**: One SPBEX trading-day record with timestamp, close, high, low, volume, and facevalue fields.
- **Latest quote record**: A single history-shaped record returned for the latest SPBEX value, fetched on each quote request.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A client can retrieve SPBEX daily history from the documented route as a six-field JSON array, with source candles sorted oldest to newest and timestamps represented in UTC.
- **SC-002**: At least 95% of successful history and quote requests complete in under one second under the project's 10-concurrent-client, 10-requests-per-second workload. Both routes MUST meet this target for the feature to pass acceptance; any miss requires optimization and repeat measurement.
- **SC-003**: Two consecutive quote requests each trigger a fresh upstream read; a change in the upstream latest value is visible in the subsequent response.
- **SC-004**: Unexpired history remains available after an application restart, while expired history is refreshed before it is returned.
- **SC-005**: Invalid symbols, successful empty data, SPBEX failures, and history-store failures produce distinguishable outcomes matching the documented API contract.

## Assumptions

- `SPBEX` is the exchange identifier; `/v1/sbpex/{SYMBOL}` in the initial description is a typo for `/v1/spbex/{SYMBOL}`, confirmed by the quote path in the same request.
- SPBEX symbols are normalized by trimming surrounding whitespace and converting letters to uppercase, matching the referenced Go adapter.
- The chart feed is not an instrument catalog. A successful empty feed is indistinguishable from valid empty history and MUST use the empty-history/no-quote behavior; HTTP `400` for an otherwise well-formed symbol is returned only when the upstream explicitly rejects it.
- Daily chart history is based on the referenced SPBEX adapter's public chart feed behavior, which requests daily candles from the beginning of available data through the current time.
- The referenced adapter uses the most recent available daily candle for its current-price method and assigns `facevalue` as `1`; the quote route follows this behavior. The feed omits volume, so the API represents it as `null` rather than the Go struct's default `0`.
- History freshness and restart persistence follow the existing project's ticker-history expectations; exact storage choices remain outside this specification.
- A valid no-quote response follows the existing MOEX API convention of returning a one-element history-shaped array with unavailable quote fields as null.
- SPBEX data is available to the service without subscription credentials.
- The service is intended for a private network; application-level authentication is not required for this capability.
