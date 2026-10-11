# Design

## Context

The service has six provider-specific v1 route aliases and v1-only wrappers. It also retains an in-memory response cache used by v2 when no persistent store is configured, plus a SQLite response-cache table and importer for older databases. The configured binary uses durable provider/symbol history collections; the response-cache mechanisms are no longer needed for that path. See proposal.md for the motivation and the specs for the externally visible behavior.

## Goals / Non-Goals

**Goals:**
- Keep provider history and quote behavior available through v2.
- Use durable history collections as the only retained history cache.
- Discard obsolete response-cache data without affecting durable collections.
- Remove alias-only tests while retaining provider behavior coverage against v2.

**Non-Goals:**
- Add a v1 deprecation period, redirects, or warning headers.
- Change provider mappings, durable record schemas, refresh policy, quote behavior, or error envelopes.
- Remove health or administrative routes, or remove the durable SQLite history store.

## Decisions

1. **Remove v1 registrations and their wrapper handlers.** V2 provider selection already dispatches history and quote requests to the same provider implementations. Remove v1-only route tests; migrate provider behavior tests to v2 instead of losing their coverage.
2. **Remove the response-cache layer, not the durable history store.** Delete the in-memory `HistoryCache`, its AppState wiring and builders, and the response-body `get`/`put` APIs on the store. For v2 history requests without a persistent store, fetch from the provider and return the result without retaining it. Configured-store requests continue using incremental refresh and durable collections.
3. **Discard old response-cache tables on store open.** Stop creating or importing `history_cache` and stop using `history_schema_migrations`. Drop either table when present so old response-cache rows and migration markers are discarded. Do not alter `history_collections` or `history_records`, which hold the current durable history.
4. **Remove response-cache-only settings and constructor parameters.** Remove the transient max-bytes setting and obsolete TTL rejection path, along with cache-only AppState parameters. Keep the database path setting because it still configures the durable history store.
5. **Clear durable history through the admin operation.** The admin endpoint continues to authorize, validate, isolate by provider/symbol, and return store failures as before; it deletes the durable collection and has no memory or legacy response-cache layer to clear.
6. **Align current contracts and project specs with the remaining behavior.** Remove v1 operations and claims about legacy response-cache import from the current API contracts and affected feature specs, plans, and quickstarts. Preserve archived change records as historical records.

## Risks / Trade-offs

- [Rows in old response-cache tables are permanently discarded on store open] -> This is the requested behavior; durable history collections are preserved. Back up the database before deployment only if recovery of the old response-cache rows is required.
- [Clients still using v1 receive 404, and no-store v2 callers fetch upstream on every request] -> Publish the v2 route mapping and the removal in release notes; configured service deployments continue retaining history in the durable store.
- [Provider tests currently rely on route aliases or old cache helpers] -> Move provider behavior coverage to v2 and replace obsolete response-cache tests with a focused test proving legacy tables are dropped while durable collections survive.

## Migration Plan

1. Deploy the route, cache, storage, contract, and test cleanup together.
2. Clients change history requests to `/v2/history/{provider}/{symbol}` and quotes to `/v2/quote/{provider}/{symbol}`.
3. Existing durable history remains available. The old response-cache table and migration marker are dropped when the new service opens the database. Rollback restores route and cache code but cannot restore discarded response-cache rows without a pre-deployment database backup.
