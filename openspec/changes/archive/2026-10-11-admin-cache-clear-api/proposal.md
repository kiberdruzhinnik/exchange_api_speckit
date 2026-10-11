# Proposal

## Why

Operators need a controlled way to remove one provider's cached history for a single symbol when its stored data must be rebuilt. The current service has no administrative cache operation, so operators otherwise need direct access to the persistent store.

## What Changes

- Add an authenticated `DELETE /v2/admin/cache/clear/{provider}/{symbol}` endpoint.
- Require a configured static API key and validate it from the `X-API-Key` header.
- Clear the selected provider/symbol history collection and any matching legacy cache row without affecting other symbols or providers.
- Document authentication, provider and symbol validation, success behavior, and errors.

## Capabilities

### New Capabilities

- `admin-cache-management`: Authenticated operator actions for clearing a provider/symbol history cache.

### Modified Capabilities

- None.

## Impact

- API routing and shared HTTP error handling.
- Startup configuration for the admin API key.
- SQLite cache store deletion behavior and OpenAPI contract.
- Operator configuration documentation and API tests.
