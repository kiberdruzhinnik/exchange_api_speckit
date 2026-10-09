# Feature Specification: MOEX Ticker History API

**Feature Branch**: `001-moex-ticker-update`

**Created**: 2026-10-07

**Status**: Sealed

**Input**: User description: "implement MOEX ticker update from SYMBOL from /v1/moex/<SYMBOL> REST API, which should be fetched from iss.moex.com and return something like JSON: [{\"date\":\"2013-03-25T00:00:00Z\",\"close\":73.35,\"high\":75.05,\"low\":73.21,\"volume\":120300,\"facevalue\":1},...]"

## Clarifications

### Session 2026-10-07

- Q: If MOEX ISS has a daily record with a missing or null value for one of the requested fields, how should the endpoint handle that record? → A: Include the record and return `null` for missing values.
- Q: If MOEX ISS has daily data for the same ticker on multiple boards, which data should `/v1/moex/<SYMBOL>` return? → A: Return data from the primary board for the ticker.
- Q: Should `facevalue` show the value MOEX currently reports for the ticker on every history record, or the value that applied on each record’s trading date? → A: Use the current board-specific `LOTSIZE` from the primary board applicable to each record’s trading date.
- Q: If a ticker’s primary board has changed, should the history use the board that is primary now for all dates, or the primary board for each trading date? → A: Use the primary board that applied on each trading date.
- Q: Should the feature assume the deployment can access MOEX’s full historical data, including any required subscription credentials? → A: Limit results to history available without a subscription.

### Session 2026-10-08

- Q: Should the `<1 sec` limit apply to every request, including uncached requests that fetch data from MOEX, or should it be a percentile target measured under normal operating conditions? → A: At least 95% of successful requests must complete in under one second under normal operating conditions.
- Q: How should the SBER `FACEVALUE` discrepancy be resolved when the current generic reference says `3` but the expected value is `1`? → A: Source the API `facevalue` value from MOEX `LOTSIZE` on the selected primary board; SBER's value is `1`.
- Q: Should the primary-board lot size be historical for the trading date or use the current lot size for that selected board? → A: Use the current `LOTSIZE` for the primary board selected on each trading date.
- Q: Which upstream and executable should final live-data and latency validation use? → A: Validate against production `https://iss.moex.com/iss` and run measurements with the locally compiled application binary.
- Q: What latency and freshness behavior must ticker requests provide? → A: At least 95% of successful requests must complete in under one second under normal operating conditions; cached history must persist across application restarts; fetch the latest quote from MOEX ISS on every request and never cache it.
- Q: How should clients receive the uncached latest quote alongside the existing daily-history API? → A: Add `GET /v1/moex/{SYMBOL}/quote` as a separate endpoint and preserve the existing history-array response contract.
- Q: Should the latest quote represent the most recent executed trade, the current best bid and ask, or both? → A: Return the most recent executed trade with its price, time, and traded size.
- Q: Where should daily-history data be persisted so it survives application restarts? → A: The specification requires unexpired history to survive application restarts but does not prescribe a storage mechanism.
- Q: What should `/v1/moex/{SYMBOL}/quote` return when MOEX ISS has no latest trade data for a recognized ticker? → A: Return `200` with the trade fields null; under the later history-shaped response decision, return one record with all six fields null.
- Q: What should `/v1/moex/{SYMBOL}/quote` return if MOEX ISS is unavailable or returns malformed trade data? → A: Return the documented dependency error; reserve `200` with null trade fields for a valid upstream response with no trade.
- Q: What request workload should define “normal operating conditions” for measuring the one-second p95 target? → A: 10 concurrent clients, 10 requests per second.
- Q: Should the specification prescribe a storage mechanism for history persistence? → A: No; require history persistence across application restarts without specifying the storage mechanism.
- Q: What response should `/v1/moex/{SYMBOL}/quote` use to follow the history endpoint’s array and record shape while preserving the chosen latest-executed-trade meaning? → A: Return a one-element array using the six history fields; map trade time to `date`, price to `close`, and size to `volume`, with `high`, `low`, and `facevalue` set to null.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Retrieve a ticker's daily history (Priority: P1)

A client requests `/v1/moex/<SYMBOL>` and receives daily market records for that MOEX ticker in a consistent JSON format, sourced from MOEX ISS.

**Why this priority**: Returning the requested ticker history is the feature's primary value.

**Independent Test**: Request a known ticker and verify the response is a JSON array of dated records with the documented fields and values corresponding to MOEX ISS.

**Acceptance Scenarios**:

1. **Given** a recognized ticker with available daily history, **When** a client requests `/v1/moex/<SYMBOL>`, **Then** the service returns a successful response containing a JSON array of records with `date`, `close`, `high`, `low`, `volume`, and `facevalue` fields.
2. **Given** a ticker with daily records on multiple boards or a primary-board change over time, **When** the client requests its history, **Then** each record comes from the board that was primary on that trading date, and records are ordered from oldest to newest with each `date` an ISO 8601 UTC timestamp.
3. **Given** a recognized ticker with no available history, **When** the client requests it, **Then** the service returns an empty JSON array.

4. **Given** a recognized ticker with a latest executed trade, **When** a client requests `/v1/moex/<SYMBOL>/quote`, **Then** the service returns a one-element JSON array with the same six fields as a history record, mapping the trade's UTC execution time to `date`, trade price to `close`, and traded size to `volume`, with `high`, `low`, and `facevalue` set to `null`; the trade is fetched from MOEX ISS for that request without reusing a prior quote.
5. **Given** a recognized ticker with no latest trade data, **When** a client requests `/v1/moex/<SYMBOL>/quote`, **Then** the service returns `200` with a one-element array whose six fields are all `null`.

### User Story 2 - Handle invalid or unavailable ticker data (Priority: P2)

A client receives a clear, documented response when the symbol is invalid or MOEX ISS cannot provide data, so it can distinguish a bad request from a temporary upstream failure.

**Why this priority**: Clear failure behavior makes the endpoint dependable for API consumers.

**Independent Test**: Request an invalid symbol and simulate an unavailable MOEX ISS response; verify documented client and upstream error responses.

**Acceptance Scenarios**:

1. **Given** a malformed or unsupported symbol, **When** a client requests the endpoint, **Then** the service returns a client error with the standard documented error representation.
2. **Given** MOEX ISS is unavailable or returns an unsuccessful response, **When** a client requests ticker history, **Then** the service returns a server-side dependency error using the standard documented error representation and does not present the failure as an empty history.
3. **Given** MOEX ISS is unavailable or returns malformed trade data, **When** a client requests `/v1/moex/<SYMBOL>/quote`, **Then** the service returns the documented server-side dependency error and does not present the failure as a valid no-trade response.

### User Story 3 - Stop the service with Ctrl+C (Priority: P2)

An operator can interrupt the running service and have it stop accepting new work and exit cleanly.

**Why this priority**: Operators need a predictable way to stop the service during local use and deployment shutdown.

**Independent Test**: Start the service, send Ctrl+C (SIGINT), and verify it stops accepting new requests, allows in-flight work to finish, and exits within 30 seconds.

**Acceptance Scenarios**:

1. **Given** the service is running and handling requests, **When** an operator sends Ctrl+C, **Then** the service stops accepting new requests, allows in-flight work to complete, and terminates within 30 seconds.
2. **Given** an in-flight request has not completed before the 30-second shutdown deadline, **When** the deadline expires, **Then** the service cancels the remaining work and terminates.

### Edge Cases

- Symbol is empty, malformed, or contains characters outside the supported ticker format.
- A valid symbol has no available daily records.
- MOEX ISS times out, is unreachable, or returns malformed or incomplete data.
- An upstream record has a missing or null market value; include the record and emit `null` for the missing value without mislabeling another field's value. A record without a trading date is unusable upstream data.
- The upstream returns records across a date boundary; emitted timestamps remain normalized to UTC.
- MOEX ISS has no latest trade data for a recognized ticker; return a successful one-element quote array with all six fields null rather than treating it as an upstream failure.
- MOEX ISS is unavailable or returns malformed latest-trade data; return the documented dependency error rather than a null-filled quote array.
- Ctrl+C arrives while requests are active; the service stops accepting new requests and terminates within the bounded shutdown period, canceling unfinished work at the deadline.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST expose ticker history at `GET /v1/moex/{SYMBOL}`.
- **FR-002**: System MUST obtain the requested ticker's daily market data from MOEX ISS at `iss.moex.com` without using subscription credentials.
- **FR-003**: A successful response MUST be a JSON array whose records contain `date`, `close`, `high`, `low`, `volume`, and `facevalue`; unavailable market values MUST be represented as JSON `null`.
- **FR-004**: `date` MUST represent the market record's date as an ISO 8601 UTC timestamp. The API MUST preserve the corresponding MOEX trading date.
- **FR-005**: `close`, `high`, `low`, and `volume` MUST be represented as JSON numbers when available from MOEX ISS history and as JSON `null` when unavailable. The JSON property `facevalue` MUST contain the current numeric `LOTSIZE` from the primary board selected for that record’s trading date, or JSON `null` when unavailable. The property name remains `facevalue` to preserve the requested response shape.
- **FR-006**: Records MUST be returned in ascending date order. The default requested history MUST include all daily records MOEX ISS makes available without subscription credentials for the symbol and its primary board on each trading date.
- **FR-007**: A recognized symbol with no available records MUST return a successful empty JSON array.
- **FR-008**: System MUST validate `SYMBOL` and return a client error for malformed or unsupported symbols using the standard documented error representation.
- **FR-009**: System MUST distinguish upstream failure or unusable upstream data from a valid empty history and return a server-side dependency error using the standard documented error representation.
- **FR-010**: API documentation MUST describe both routes, symbol format, successful response fields and types, history ordering and empty-history behavior, the quote no-trade behavior, and documented client and dependency error behavior.
- **FR-011**: If the ticker's primary board changes over time or daily records exist on multiple boards, System MUST return each record from the board that was primary on that trading date. A source record without a trading date MUST be treated as unusable upstream data.
- **FR-012**: Under normal operating conditions, defined as 10 concurrent clients issuing 10 requests per second in total, at least 95% of successful requests to `/v1/moex/{SYMBOL}` and `/v1/moex/{SYMBOL}/quote` MUST return the complete response in under one second, measured from receipt of the request to completion of the response.
- **FR-013**: Previously retrieved ticker history MUST survive application restarts for its applicable freshness period. When history is no longer fresh, System MUST retrieve fresh history from MOEX ISS before returning it and MUST NOT return expired history as current.
- **FR-014**: `GET /v1/moex/{SYMBOL}/quote` MUST fetch the most recent executed trade for the requested ticker from MOEX ISS on every request and MUST NOT reuse previously fetched quote data. It MUST return a one-element JSON array with the same six fields as a history record: `date` (ISO 8601 UTC timestamp or null), `close` (number or null), `high` (number or null), `low` (number or null), `volume` (number or null), and `facevalue` (number or null). For a trade, `date` MUST contain the UTC execution time, `close` the trade price, and `volume` the traded size; `high`, `low`, and `facevalue` MUST be null. If MOEX ISS has no latest trade data for a recognized ticker, the endpoint MUST return `200` with one array record whose six fields are all null. If MOEX ISS is unavailable, returns an unsuccessful response, or provides malformed trade data, the endpoint MUST return the documented server-side dependency error.
- **FR-015**: The release process MUST build a Linux amd64 container image. AMD64 architecture acceptance MUST be satisfied by a successful image build and MUST NOT require starting the image or exercising service routes on amd64.
- **FR-016**: When the service receives Ctrl+C (SIGINT), it MUST stop accepting new requests, allow in-flight work to finish, and terminate within 30 seconds. Any work still in flight at the deadline MUST be canceled.

### Key Entities *(include if feature involves data)*

- **Ticker**: A MOEX-traded instrument identified by its symbol.
- **Daily market record**: One trading-date record for a ticker, containing the date and closing, high, low, volume, and face-value values.
- **Latest trade quote**: The most recent executed trade for a ticker, represented using one history-shaped array record: execution time in `date`, price in `close`, and traded size in `volume`; `high`, `low`, and `facevalue` are null.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For the known test ticker `SBER` on MOEX trading date `2026-10-06`, the returned record contains all six documented fields with the correct JSON types, has `facevalue` equal to `1`, and allows `null` for unavailable market values.
- **SC-002**: For known tickers, `date`, `close`, `high`, `low`, and `volume` match the MOEX ISS history available without subscription credentials for the board that was primary on each trading date; `facevalue` matches the current `LOTSIZE` for that selected board, and records are in ascending date order.
- **SC-003**: All specified cases (available history, empty history, invalid symbol, and upstream failure) produce their defined response outcome in acceptance testing.
- **SC-004**: A client can retrieve a ticker's history using only the documented route and response contract, without needing to interpret raw MOEX ISS field positions.
- **SC-005**: With 10 concurrent clients issuing 10 requests per second in total, at least 95% of successful requests to the history and quote endpoints complete in under one second, measured end to end from request receipt through delivery of the complete response.
- **SC-006**: After restarting an application instance, unexpired history remains available without a full-history refetch, while each request to `/v1/moex/{SYMBOL}/quote` fetches fresh quote data from MOEX ISS rather than returning a cached quote.
- **SC-007**: The quote endpoint returns a one-element history-shaped array containing the latest trade's UTC execution time, price, and size in `date`, `close`, and `volume`, with `high`, `low`, and `facevalue` null; if no trade is available, it returns one record with all six fields null.
- **SC-008**: A Linux amd64 container image builds successfully. AMD64 acceptance requires build success only; runtime startup and endpoint checks on amd64 are not required.
- **SC-009**: After Ctrl+C is sent to a running service, it stops accepting new requests and exits within 30 seconds, completing in-flight work where possible and canceling any work left at the deadline.

## Assumptions

- `SYMBOL` is the MOEX instrument ticker used by MOEX ISS; no exchange board or instrument class is supplied by the caller.
- When board assignments change, MOEX ISS listing history identifies which board was primary for each trading date.
- The response property `facevalue` is sourced from the current board-specific MOEX `LOTSIZE` reference for the primary board selected on each record’s trading date. It is not sourced from the generic `FACEVALUE` field or a historical lot-size series. If the selected board's current `LOTSIZE` is unavailable, return `null`. For the known `SBER` record dated `2026-10-06`, the expected value is `1`.
- The initial response covers all daily history available without subscription credentials for that symbol; date-range and pagination parameters are not part of this feature.
- The service does not use a MOEX subscriber account or credentials; if MOEX ISS denies a request or limits the returned history, the response must reflect the upstream result according to the documented success/error behavior.
- Dates represent daily trading dates and are serialized at midnight UTC, as in the user's example.
- The project-wide standard documented error representation applies to invalid symbols and upstream failures.
- AMD64 compatibility is verified by successfully building the Linux amd64 container image; running or testing that image on amd64 is outside acceptance scope.
- The endpoint is intended for the project's private-network deployment described in its constitution.
- Ctrl+C requests a graceful shutdown with a 30-second window for in-flight work; work remaining at the deadline is canceled.
