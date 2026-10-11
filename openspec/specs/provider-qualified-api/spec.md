# provider-qualified-api Specification

## Purpose

Defines the public provider-qualified history and quote API, giving clients a single route convention across MOEX, SPBEX, and CBR.

## Requirements

### Requirement: Serve history and quotes through provider-qualified v2 routes
The service MUST provide history and quote operations through provider-qualified v2 routes for each supported provider, preserving provider-specific normalization, validation, and response formats.

#### Scenario: Request provider history
- **WHEN** a caller sends `GET /v2/history/{provider}/{symbol}` for `moex`, `spbex`, or `cbr`
- **THEN** the service returns that provider's history using its existing refresh behavior

#### Scenario: Request provider quote
- **WHEN** a caller sends `GET /v2/quote/{provider}/{symbol}` for `moex`, `spbex`, or `cbr`
- **THEN** the service returns that provider's quote using its existing behavior

### Requirement: Do not retain history responses in the in-memory fallback cache
When no persistent history store is configured, the service MUST fetch history directly from the provider and MUST NOT retain the response in an in-memory cache.

#### Scenario: Request history without a persistent store
- **WHEN** a caller requests history through a v2 route while no persistent store is configured
- **THEN** the service fetches and returns provider history without retaining a response cache for a later request

### Requirement: Discard legacy response-cache data
The service MUST NOT import records from the legacy response-cache table into durable history collections.

#### Scenario: Open a database containing legacy response-cache rows
- **WHEN** the persistent store opens a database containing the legacy response-cache table
- **THEN** the legacy response-cache data and its migration marker are discarded while existing durable history collections and records are retained

### Requirement: Remove the v1 provider-specific history and quote routes
The service MUST NOT expose provider-specific v1 history or quote routes after this change.

#### Scenario: Caller requests a removed v1 route
- **WHEN** a caller sends a request to any former `/v1/{provider}/{symbol}` or `/v1/{provider}/{symbol}/quote` route
- **THEN** the service returns HTTP 404 and performs no history or quote operation
