# Feature Specification: V2 History and Quote Routes

**Feature Branch**: `[005-v2-history-quote-api]`

**Created**: 2026-10-09

**Status**: Completed

**Input**: User description: "move history fetch to /v2/history/<SYMBOL> and current quote to /v2/quote/<SYMBOL>"; clarified to include `<PROVIDER>` in each path and keep existing v1 routes.

## Clarifications

### Session 2026-10-09

- Q: Should each v2 route be measured separately at 10 total requests per second with 10 concurrent clients? → A: Run six separate route profiles, one for each v2 provider history and quote route, each at 10 total requests per second with 10 concurrent clients; at least 95% of successful responses in each profile must complete in under one second.
- Q: When a valid history request gets a successful response with no records, should v2 return `[]` and have an explicit acceptance scenario and API test for that result? → A: Return `[]` and explicitly test this outcome in the v2 history acceptance scenario and API tests.
- Q: Should SC-005 exclude the first uncached MOEX history request from its one-second target, while requiring subsequent requests served from the completed cache to meet the target? → A: Report the initial MOEX full-fetch latency separately; apply SC-005’s one-second target to subsequent cached MOEX history requests. Keep the existing target for the other five v2 routes.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Fetch symbol history from v2 (Priority: P1)

As an API client, I want to fetch a symbol's complete history from a versioned v2 route so that history access follows the new public URL structure.

**Why this priority**: History is a primary service capability and the requested route migration.

**Independent Test**: Request a supported provider-symbol pair through `GET /v2/history/{PROVIDER}/{SYMBOL}` and verify the response contains its complete history in the established record format.

**Acceptance Scenarios**:

1. **Given** a supported provider and symbol, **When** its v2 history route is requested, **Then** the service returns that provider-symbol pair's complete retained history using the established six-field record shape and ordering.
2. **Given** an unsupported provider or malformed or unsupported symbol, **When** its v2 history route is requested, **Then** the service returns the established client error behavior.
3. **Given** the source or history store fails, **When** the v2 history route is requested, **Then** the service returns the established upstream or store error behavior.
4. **Given** a valid provider-symbol pair and the source successfully returns no history, **When** its v2 history route is requested, **Then** the service returns an empty array (`[]`).

### User Story 2 - Fetch the current quote from v2 (Priority: P1)

As an API client, I want to fetch a symbol's current quote from a versioned v2 route so that quote access follows the new public URL structure.

**Why this priority**: Clients need a direct current quote route alongside the new history route.

**Independent Test**: Request a supported provider-symbol pair through `GET /v2/quote/{PROVIDER}/{SYMBOL}` and verify the response matches the existing current-quote format and freshness behavior.

**Acceptance Scenarios**:

1. **Given** a supported provider and symbol with current source data, **When** its v2 quote route is requested, **Then** the service returns that provider's current quote in the established one-record, six-field format.
2. **Given** a valid provider-symbol pair with no quote data, **When** its v2 quote route is requested, **Then** the service returns the established no-quote result.
3. **Given** the source is unavailable or returns unusable data, **When** the v2 quote route is requested, **Then** the service returns the established upstream error behavior.

### Edge Cases

- The provider segment must identify one of the supported providers: `moex`, `spbex`, or `cbr`.
- The same symbol may be accepted by more than one provider; the provider path segment must select the intended provider's history or quote.
- Existing v1 routes remain available during the v2 migration to preserve current consumers.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST expose history retrieval at `GET /v2/history/{PROVIDER}/{SYMBOL}`, where `{PROVIDER}` is `moex`, `spbex`, or `cbr`.
- **FR-002**: The system MUST expose current quote retrieval at `GET /v2/quote/{PROVIDER}/{SYMBOL}`, where `{PROVIDER}` is `moex`, `spbex`, or `cbr`.
- **FR-003**: The system MUST use the provider path segment to select the data source and provider-specific behavior for the requested symbol.
- **FR-004**: V2 history responses MUST preserve the established complete-history behavior, six-field record schema, and provider ordering.
- **FR-005**: V2 quote responses MUST preserve the established current-quote behavior and six-field record schema.
- **FR-006**: The v2 routes MUST preserve established symbol validation and actionable upstream, store, and no-data outcomes.
- **FR-007**: The existing v1 provider-specific history and quote routes MUST remain available while clients migrate to v2.

### Key Entities *(include if feature involves data)*

- **Symbol request**: A normalized symbol submitted to one v2 history or quote route and associated with the provider selected by the clarified rule.
- **Provider result**: The established provider-specific history or current quote represented through the shared six-field response format.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Each supported provider-symbol combination can retrieve history through `GET /v2/history/{PROVIDER}/{SYMBOL}` with the established response shape and ordering.
- **SC-002**: Each supported provider-symbol combination can retrieve a current quote through `GET /v2/quote/{PROVIDER}/{SYMBOL}` with the established response shape and freshness behavior.
- **SC-003**: All v1 history and quote routes continue to return their existing documented outcomes during the v2 migration.
- **SC-004**: Invalid, unsupported, ambiguous, and unavailable-symbol cases return documented outcomes without serving data from an unintended provider.
- **SC-005**: In a separate workload profile for each of the six v2 provider history and quote routes, at 10 total requests per second with 10 concurrent clients, at least 95% of successful responses complete in under one second. For MOEX history, measure the initial uncached full-history fetch separately and report its latency and outcome; apply the one-second criterion to a separate profile run after the full history has been populated in the cache, measuring subsequent cached history requests.

## Assumptions

- V2 changes route organization only; established provider mappings, cache lifecycle, response records, and error contracts remain unchanged.
- V1 provider-specific routes remain available to preserve compatibility unless a later migration decision changes that policy.
- The v2 route paths are exactly `/v2/history/{PROVIDER}/{SYMBOL}` and `/v2/quote/{PROVIDER}/{SYMBOL}`; braces denote path parameters and are not literal URL characters.
