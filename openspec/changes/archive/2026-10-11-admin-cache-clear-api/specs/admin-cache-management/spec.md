# Spec Delta

## Purpose

Provides authenticated operator controls for clearing one provider and symbol's retained history so a later request can rebuild it from the provider source.

## ADDED Requirements

### Requirement: Clear one provider and symbol history cache
The system MUST expose an authenticated operation that removes cached history for exactly one supported provider and normalized symbol from all cache layers.

#### Scenario: Clear a cached collection
- **WHEN** an authorized caller sends `DELETE /v2/admin/cache/clear/{provider}/{symbol}` for a supported provider and symbol with cached history
- **THEN** the service removes that provider/symbol's in-memory entry, durable history collection and records, and matching legacy cache entry, and returns `204 No Content`

#### Scenario: Clear an uncached pair
- **WHEN** an authorized caller clears a valid provider/symbol pair that has no cached history
- **THEN** the service returns `204 No Content` and leaves all other cache entries unchanged

#### Scenario: Subsequent history request repopulates cache
- **WHEN** a history request for a cleared provider/symbol completes successfully
- **THEN** the response is built using the existing provider refresh behavior and the resulting history is retained as usual

### Requirement: Protect cache clearing with a static API key
The system MUST authorize cache-clearing requests only when the `X-API-Key` header exactly matches the static key configured by `EXCHANGE_API_ADMIN_API_KEY`.

#### Scenario: Missing or incorrect API key
- **WHEN** a caller omits `X-API-Key` or supplies a value that does not match the configured key
- **THEN** the service returns `401 Unauthorized` and does not modify any cache data

#### Scenario: Admin key is not configured
- **WHEN** `EXCHANGE_API_ADMIN_API_KEY` is unset or empty
- **THEN** cache-clearing requests return `503 Service Unavailable` and do not modify any cache data

#### Scenario: Authorized caller
- **WHEN** a caller supplies the configured API key in `X-API-Key`
- **THEN** the service evaluates the requested provider/symbol and may perform the requested cache clear

### Requirement: Validate cache-clear target
The system MUST accept only the supported provider identifiers and validate symbols using the selected provider's existing normalization rules before clearing.

#### Scenario: Unsupported provider
- **WHEN** an authorized caller supplies a provider other than `moex`, `spbex`, or `cbr`
- **THEN** the service returns HTTP 400 with the shared `invalid_provider` error envelope and changes no cache data

#### Scenario: Invalid symbol
- **WHEN** an authorized caller supplies a symbol rejected by the selected provider's existing validation rules
- **THEN** the service returns HTTP 400 with the shared `invalid_symbol` error envelope and changes no cache data

### Requirement: Preserve cache isolation
The system MUST limit a cache-clear operation to the selected provider and normalized symbol.

#### Scenario: Other cache entries remain
- **WHEN** a cache-clear operation succeeds for one provider/symbol pair
- **THEN** cached history for every other provider or symbol remains available

### Requirement: Report cache-store failures
The system MUST report a cache-store failure without claiming that the requested clear succeeded.

#### Scenario: Persistent deletion fails
- **WHEN** the persistent store cannot complete the cache-clear operation
- **THEN** the service returns HTTP 503 with the shared `history_store_unavailable` error envelope
