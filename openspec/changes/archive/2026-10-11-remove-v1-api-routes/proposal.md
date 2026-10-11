# Proposal

## Why

The provider-specific `/v1` routes and their response-cache compatibility layer are superseded by the provider-qualified `/v2` API and durable history collections. Removing both leaves one public route family and eliminates cache code, migration paths, tests, and configuration that no longer serve the intended model.

## What Changes

- **BREAKING** Remove all six `/v1/{provider}/{symbol}` history and `/v1/{provider}/{symbol}/quote` routes and their route-only handlers. Clients use the equivalent `/v2/history/{provider}/{symbol}` and `/v2/quote/{provider}/{symbol}` routes.
- Remove the in-memory history response cache and the legacy SQLite `history_cache` response table and importer. On store startup, discard the old response-cache table and migration marker; retain existing durable history collections and records.
- For v2 history requests without a configured persistent store, fetch from the provider directly and do not retain a response cache.
- Remove configuration and tests used only by the removed response-cache feature. Delete tests dedicated to the v1 aliases and migrate provider-behavior coverage to v2.
- Update API contracts, related specs and documentation, and the admin cache-clear requirement to describe the v2 routes and durable collections only.

## Capabilities

### New Capabilities
- `provider-qualified-api`: Defines the supported provider-qualified history and quote routes and excludes the removed v1 aliases.

### Modified Capabilities
- `admin-cache-management`: Cache clearing now targets durable history collections only; in-memory and legacy response-cache layers are removed.

## Impact

Affected code includes `src/http/routes.rs`, `src/cache.rs`, `src/cache_store.rs`, `src/lib.rs`, `src/config.rs`, and `src/main.rs`. API and project contract documents, route/provider tests, cache-store tests, and configuration tests need cleanup. Existing durable history data remains; rows in the old response-cache table are discarded when the store opens.
