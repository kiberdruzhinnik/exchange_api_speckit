# Feature Specification: MOEX Benchmark and Currency Instrument Support

**Feature Branch**: `007-moex-instrument-coverage`

**Created**: 2026-10-10

**Status**: Completed

**Input**: User description: "looks like current implementation of moex does not work with market benchmarks, e.g. i cannot fetch IMOEX benchmark ticker. also it does not work with currencies, e.g. i cannot fetch GLDRUB_TOM ticker"

## Clarifications

### Session 2026-10-10

- Q: When validation starts accepting IMOEX, GLDRUB_TOM, and similar MOEX symbols, when should their existing cached history be migrated? → A: Migrate a symbol's existing cached history the first time its request passes validation.
- Q: Which underscore pattern should the MOEX symbol validator accept after case and whitespace normalization? → A: For newly supported benchmark and currency symbols, underscores separate alphanumeric parts; leading, trailing, and repeated underscores are invalid. Existing supported symbols retain their current validation behavior.
- Q: Should a newly supported ticker’s cached history migrate only after MOEX metadata confirms the ticker is recognized, or as soon as its syntax passes validation? → A: Confirm the ticker through MOEX metadata first; an unknown ticker returns HTTP 400 without migration, and a recognized ticker is migrated before history retrieval. A metadata request failure returns the standard MOEX dependency error without migration.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Retrieve benchmark history and current value (Priority: P1)

A client can request a MOEX market benchmark such as IMOEX through the existing MOEX history and quote routes and receive useful benchmark data in the established market-record format.

**Why this priority**: Benchmark data is a core market instrument type that clients currently cannot retrieve through the MOEX integration.

**Independent Test**: Request IMOEX history and quote and compare returned dates and values with the corresponding MOEX published benchmark data.

**Acceptance Scenarios**:

1. **Given** IMOEX has published daily history, **When** a client requests `/v1/moex/IMOEX`, **Then** the service returns its daily records in the established six-field JSON array format, ordered oldest to newest.
2. **Given** MOEX has a current published IMOEX value, **When** a client requests `/v1/moex/IMOEX/quote`, **Then** the service returns one record representing the latest available benchmark value using the established quote response format.
3. **Given** a valid benchmark has no available history or current value, **When** the corresponding route is requested, **Then** the service returns the established successful empty-history or no-quote response.
4. **Given** a client uses the provider-qualified v2 routes for MOEX, **When** it requests IMOEX history or quote, **Then** it receives the same supported benchmark data and record contract as the corresponding v1 route.
5. **Given** cached history exists for a newly supported symbol such as IMOEX or GLDRUB_TOM, **When** MOEX metadata confirms the symbol is recognized, **Then** the service migrates that history before requesting history and returns migrated records combined with fetched records according to the existing refresh behavior; if migration fails, it returns the history-store error and preserves the legacy cache row. A syntactically valid but unrecognized symbol returns HTTP 400 without migration, and a metadata request failure returns the standard MOEX dependency error without migration.

### User Story 2 - Retrieve currency instrument history and quote (Priority: P1)

A client can request MOEX-traded currency instruments, such as GLDRUB_TOM, through the existing MOEX routes and receive their available market history and current quote.

**Why this priority**: MOEX-traded currencies are distinct from official central-bank rates and must be available to clients seeking exchange-traded market data.

**Independent Test**: Request GLDRUB_TOM history and quote and compare returned dates, prices, and trade values with the corresponding MOEX market data.

**Acceptance Scenarios**:

1. **Given** GLDRUB_TOM has daily history, **When** a client requests `/v1/moex/GLDRUB_TOM`, **Then** the service returns the available daily records in the established six-field JSON array format, ordered oldest to newest.
2. **Given** GLDRUB_TOM has a latest executed trade, **When** a client requests `/v1/moex/GLDRUB_TOM/quote`, **Then** the service returns one record representing that trade using the established MOEX quote mapping.
3. **Given** another supported MOEX currency instrument is identified by a symbol containing an underscore, **When** a client requests its history or quote, **Then** the symbol is accepted and handled as the requested instrument.
4. **Given** a client uses the provider-qualified v2 routes for MOEX, **When** it requests GLDRUB_TOM history or quote, **Then** it receives the same supported currency data and record contract as the corresponding v1 route.

### User Story 3 - Preserve existing MOEX behavior for other instruments (Priority: P2)

Clients continue to retrieve equity and other already supported MOEX instruments with the same routes, record fields, and error behavior while benchmark and currency support is added.

**Why this priority**: Extending instrument coverage must not make existing MOEX integrations less reliable or change their response contract.

**Independent Test**: Request a previously supported equity such as SBER and verify its history and quote outcomes remain consistent with the current documented contract.

**Acceptance Scenarios**:

1. **Given** a previously supported MOEX equity, **When** a client requests its history or quote, **Then** existing routes and response contracts remain available.
2. **Given** any requested MOEX instrument is malformed or unsupported, **When** a client requests history or quote, **Then** the service returns the standard documented invalid-symbol response.
3. **Given** MOEX ISS is unavailable or returns unusable data for any supported instrument category, **When** a client requests history or quote, **Then** the service returns the standard MOEX dependency error rather than valid empty data.

### Edge Cases

- A newly supported ticker may contain single underscores between alphanumeric parts, as in GLDRUB_TOM; leading, trailing, or consecutive underscores are invalid. Existing supported symbols retain their current validation behavior.
- A benchmark provides a published index level rather than an executed trade; quote output must represent its latest available published value in the established quote record shape.
- A currency instrument or benchmark has no available record for a requested data kind; history and quote use the established empty/no-quote outcomes.
- Source data omits values that do not apply to an instrument category; unavailable fields are represented as `null`.
- MOEX ISS reports a recognized instrument but returns unsuccessful, malformed, or unusable market data.
- A previously fetched symbol has cached history when its request passes syntax validation and MOEX metadata confirms it is recognized; a syntactically valid but unrecognized symbol must not trigger migration.
- Migration of existing cached history fails; the service preserves the original cached data and reports the documented history-store failure.
- Existing equity requests continue to use the same board-selection and field-mapping behavior.
- Symbol case and surrounding whitespace are normalized according to the existing MOEX API contract.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST support MOEX equities, market benchmarks including IMOEX, and MOEX-traded currency instruments including GLDRUB_TOM through the existing MOEX history route `GET /v1/moex/{SYMBOL}`.
- **FR-002**: The system MUST support benchmark and currency instruments through the existing MOEX quote route `GET /v1/moex/{SYMBOL}/quote`.
- **FR-003**: After existing case and whitespace normalization, the system MUST accept newly supported benchmark and currency symbols made of one or more alphanumeric parts separated by single underscores, including `IMOEX` and `GLDRUB_TOM`. These symbols MUST NOT have a leading or trailing underscore or consecutive underscores. Existing supported symbols MUST retain their current validation behavior.
- **FR-004**: Successful history responses MUST retain the existing JSON array contract with exactly `date`, `close`, `high`, `low`, `volume`, and `facevalue` fields and records ordered oldest to newest.
- **FR-005**: History values MUST reflect MOEX's published daily data for the requested instrument. A field that is unavailable or inapplicable to that instrument MUST be represented as JSON `null`.
- **FR-006**: Successful quote responses MUST retain the existing one-element history-shaped JSON array contract. For a traded instrument, the record MUST represent the latest executed trade as defined by the current MOEX quote contract. For a benchmark without executed trades, the record MUST represent the latest available published benchmark value, with unavailable fields set to `null`.
- **FR-007**: A valid instrument with no available history MUST return an empty JSON array. A valid instrument with no available quote or published current value MUST return HTTP 200 with one record whose six fields are all `null`.
- **FR-008**: The system MUST distinguish malformed or unsupported symbols from valid instruments with no data, using HTTP 400 and the standard documented error representation for invalid symbols.
- **FR-009**: The system MUST return the standard MOEX dependency error for unsuccessful or unusable upstream data and MUST NOT represent such failures as empty history or a no-quote result.
- **FR-010**: The addition of benchmark and currency support MUST preserve the documented routes and response behavior for existing MOEX equity clients.
- **FR-011**: API documentation MUST describe supported MOEX instrument categories, symbol examples including `IMOEX` and `GLDRUB_TOM`, response fields, empty/no-quote behavior, and client and dependency errors.
- **FR-012**: The feature MUST remain compatible with the shared provider-qualified MOEX history and quote routes and their existing response contract.
- **FR-013**: For a history request for a newly supported benchmark or currency symbol, the system MUST resolve MOEX metadata after symbol syntax validation and before cache migration. If metadata confirms the symbol is recognized, the system MUST migrate any existing cached history for that MOEX symbol into the current persistent history collection before requesting history. The response MUST retain and combine migrated records with newly fetched records according to the existing history refresh behavior. If metadata reports an unrecognized symbol, the system MUST return HTTP 400 without migrating its cache. If metadata retrieval fails or returns unusable data, the system MUST return the standard MOEX dependency error without migrating its cache. If cache migration fails after recognition, the system MUST preserve the source cache data and return the documented history-store error.

### Key Entities *(include if data involved)*

- **MOEX instrument**: An exchange-listed or exchange-published instrument identified by its MOEX symbol and belonging to an equity, benchmark, or currency category.
- **Daily market record**: A dated MOEX record containing available price, range, volume, and applicable instrument metadata values.
- **Current value record**: A one-record response representing the latest executed trade for traded instruments or latest published value for benchmarks.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Clients can retrieve non-empty history for IMOEX and GLDRUB_TOM when MOEX publishes history for those instruments, using the existing MOEX history route.
- **SC-002**: Clients can retrieve a current value for IMOEX and a latest trade for GLDRUB_TOM when MOEX publishes those values, using the existing quote route.
- **SC-003**: For IMOEX and GLDRUB_TOM, all returned records use the documented six fields, preserve available source values, represent unavailable values as `null`, and order history oldest to newest.
- **SC-004**: Existing MOEX equity history and quote requests continue to meet their documented response and error outcomes.
- **SC-005**: Malformed symbols, valid empty data, and upstream failures remain distinguishable by their documented HTTP and response outcomes for all supported instrument categories.
- **SC-006**: Existing cached history for every newly supported benchmark or currency symbol is retained in the current history store when its first history request passes validation.

## Assumptions

- MOEX benchmarks and MOEX-traded currency instruments are MOEX instruments and belong under the existing `moex` provider; the CBR provider continues to represent official Bank of Russia rates.
- The scope covers the existing MOEX v1 routes and corresponding provider-qualified v2 routes, without adding new public routes.
- The established six-field market record remains the shared response contract; fields not supplied or not applicable for an instrument category are `null`.
- Currency symbols with underscores are valid MOEX symbols; underscore support must not broaden acceptance of otherwise malformed symbols.
- MOEX's latest published benchmark value is the meaningful equivalent of a current quote for a benchmark that has no executed trades.
- Existing MOEX normalization, primary-board selection where applicable, persistence, error, performance, and operational requirements continue to apply.
