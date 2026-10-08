# Feature Specification: MOEX Ticker History API

**Feature Branch**: `001-moex-ticker-update`

**Created**: 2026-10-07

**Status**: Draft

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

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Retrieve a ticker's daily history (Priority: P1)

A client requests `/v1/moex/<SYMBOL>` and receives daily market records for that MOEX ticker in a consistent JSON format, sourced from MOEX ISS.

**Why this priority**: Returning the requested ticker history is the feature's primary value.

**Independent Test**: Request a known ticker and verify the response is a JSON array of dated records with the documented fields and values corresponding to MOEX ISS.

**Acceptance Scenarios**:

1. **Given** a recognized ticker with available daily history, **When** a client requests `/v1/moex/<SYMBOL>`, **Then** the service returns a successful response containing a JSON array of records with `date`, `close`, `high`, `low`, `volume`, and `facevalue` fields.
2. **Given** a ticker with daily records on multiple boards or a primary-board change over time, **When** the client requests its history, **Then** each record comes from the board that was primary on that trading date, and records are ordered from oldest to newest with each `date` an ISO 8601 UTC timestamp.
3. **Given** a recognized ticker with no available history, **When** the client requests it, **Then** the service returns an empty JSON array.

### User Story 2 - Handle invalid or unavailable ticker data (Priority: P2)

A client receives a clear, documented response when the symbol is invalid or MOEX ISS cannot provide data, so it can distinguish a bad request from a temporary upstream failure.

**Why this priority**: Clear failure behavior makes the endpoint dependable for API consumers.

**Independent Test**: Request an invalid symbol and simulate an unavailable MOEX ISS response; verify documented client and upstream error responses.

**Acceptance Scenarios**:

1. **Given** a malformed or unsupported symbol, **When** a client requests the endpoint, **Then** the service returns a client error with the standard documented error representation.
2. **Given** MOEX ISS is unavailable or returns an unsuccessful response, **When** a client requests ticker history, **Then** the service returns a server-side dependency error using the standard documented error representation and does not present the failure as an empty history.

### Edge Cases

- Symbol is empty, malformed, or contains characters outside the supported ticker format.
- A valid symbol has no available daily records.
- MOEX ISS times out, is unreachable, or returns malformed or incomplete data.
- An upstream record has a missing or null market value; include the record and emit `null` for the missing value without mislabeling another field's value. A record without a trading date is unusable upstream data.
- The upstream returns records across a date boundary; emitted timestamps remain normalized to UTC.

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
- **FR-010**: API documentation MUST describe the route, symbol format, successful response fields and types, ordering, empty-history behavior, and error behavior.
- **FR-011**: If the ticker's primary board changes over time or daily records exist on multiple boards, System MUST return each record from the board that was primary on that trading date. A source record without a trading date MUST be treated as unusable upstream data.
- **FR-012**: Under normal operating conditions, at least 95% of successful `/v1/moex/{SYMBOL}` requests MUST return the complete response in under one second, measured from receipt of the request to completion of the response.

### Key Entities *(include if feature involves data)*

- **Ticker**: A MOEX-traded instrument identified by its symbol.
- **Daily market record**: One trading-date record for a ticker, containing the date and closing, high, low, volume, and face-value values.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For the known test ticker `SBER` on MOEX trading date `2026-10-06`, the returned record contains all six documented fields with the correct JSON types, has `facevalue` equal to `1`, and allows `null` for unavailable market values.
- **SC-002**: For known tickers, `date`, `close`, `high`, `low`, and `volume` match the MOEX ISS history available without subscription credentials for the board that was primary on each trading date; `facevalue` matches the current `LOTSIZE` for that selected board, and records are in ascending date order.
- **SC-003**: All specified cases (available history, empty history, invalid symbol, and upstream failure) produce their defined response outcome in acceptance testing.
- **SC-004**: A client can retrieve a ticker's history using only the documented route and response contract, without needing to interpret raw MOEX ISS field positions.
- **SC-005**: Under normal operating conditions, at least 95% of successful requests to `/v1/moex/{SYMBOL}` complete in under one second, measured end to end from request receipt through delivery of the complete response.

## Assumptions

- `SYMBOL` is the MOEX instrument ticker used by MOEX ISS; no exchange board or instrument class is supplied by the caller.
- When board assignments change, MOEX ISS listing history identifies which board was primary for each trading date.
- The response property `facevalue` is sourced from the current board-specific MOEX `LOTSIZE` reference for the primary board selected on each record’s trading date. It is not sourced from the generic `FACEVALUE` field or a historical lot-size series. If the selected board's current `LOTSIZE` is unavailable, return `null`. For the known `SBER` record dated `2026-10-06`, the expected value is `1`.
- The initial response covers all daily history available without subscription credentials for that symbol; date-range and pagination parameters are not part of this feature.
- The service does not use a MOEX subscriber account or credentials; if MOEX ISS denies a request or limits the returned history, the response must reflect the upstream result according to the documented success/error behavior.
- Dates represent daily trading dates and are serialized at midnight UTC, as in the user's example.
- The project-wide standard documented error representation applies to invalid symbols and upstream failures.
- The endpoint is intended for the project's private-network deployment described in its constitution.
