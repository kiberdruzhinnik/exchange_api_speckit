# Design

## Context

See `proposal.md` for motivation and `specs/admin-cache-management/spec.md` for the observable contract. The HTTP router selects providers in `src/http/routes.rs`. Durable history is stored in SQLite as provider/symbol collections and dated rows; MOEX legacy entries remain in a separate response table. The v1 response cache also keeps entries in Moka memory. The refresh worker enumerates durable collections, so deleting a collection also removes it from future scheduled refreshes.

## Goals / Non-Goals

**Goals:**
- Authenticate the administrative operation before validating or mutating its target.
- Clear one normalized provider/symbol pair across the persistent and in-process history stores.
- Keep existing service deployments functional when no admin key is configured while failing closed for the admin route.

**Non-Goals:**
- Add user accounts, per-operator keys, key rotation, or a general authorization framework.
- Clear quote data; provider quote routes fetch directly from their source and do not use the history cache.
- Guarantee that a history request already in flight cannot repopulate a pair after a clear completes.

## Decisions

1. **Use `DELETE /v2/admin/cache/clear/{provider}/{symbol}` and return 204.** The operation removes a targeted resource, so DELETE communicates the effect and is naturally repeatable. Return 204 whether or not a cache entry existed, avoiding cache-presence disclosure and making retries safe. A POST action endpoint was considered but adds no needed behavior.

2. **Read one static key from `EXCHANGE_API_ADMIN_API_KEY` and accept it only in `X-API-Key`.** Keep the secret in startup configuration and application state; do not put it in URLs or log it. Missing and incorrect keys are rejected before provider selection. An unset or empty key disables this endpoint with 503 rather than preventing the rest of the service from starting. Requiring the key at startup was considered, but would break existing deployments that do not use administrative operations.

3. **Reuse provider selection and normalization, without upstream preflight.** Accept only the established lowercase provider path values and normalize the symbol through that provider's existing rules. Do not contact upstream metadata: clearing a local pair should remain possible during source outages and should not create cache entries. Invalid provider and symbol errors use the existing shared 400 envelopes.

4. **Delete persistent rows transactionally, then invalidate memory.** Add a cache-store operation that deletes the provider/symbol collection (letting the foreign-key cascade remove records) and the matching exact legacy response key in the same transaction. After commit, invalidate the v1 in-memory key `NAMESPACE:SYMBOL`. If the store is unavailable, return the shared 503 store error and do not report success. When the application is constructed without a persistent store, invalidate the in-memory entry directly. The removal of a persistent collection also removes it from the scheduled refresh inventory.

5. **Do not coordinate already-running history fetches in this change.** A request that began before the clear may finish its source fetch and write the pair afterward. Adding per-key coordination across HTTP refreshes and background refreshes would alter those workflows; the clear operation is a point-in-time removal, and a later history request is allowed to repopulate the cache.

## Risks / Trade-offs

- [A static key is shared by all operators and grants the same authority to anyone holding it] -> Keep it in deployment secret configuration, require the dedicated header, reject missing/incorrect values, and never log it.
- [An in-flight history fetch can repopulate a just-cleared pair] -> Document point-in-time semantics; operators can retry after active requests settle if they need the cache empty.
- [Removing durable data forces a future history request to fetch a larger source range] -> Restrict deletion to one provider/symbol and use existing provider bounds and error handling.

## Migration Plan

Deploy with `EXCHANGE_API_ADMIN_API_KEY` set to a non-empty secret to enable the route. Deployments without the setting continue to start and receive 503 from this route. Rollback requires no database migration; the endpoint and configuration can be removed with the application version.
