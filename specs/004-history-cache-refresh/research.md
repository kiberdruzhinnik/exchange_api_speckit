# Research: Permanent History Cache Refresh

## Decision 1: Persist records and refresh metadata per provider and symbol

- **Decision**: Give each provider/symbol pair a durable history collection and store the latest retained record date plus the last successful full-refresh time. Merge successful incremental and full source results by record date in a transaction.
- **Rationale**: The current store contains one expiring JSON response per composite symbol key. The worker must enumerate all previously cached symbols after restart; it cannot do that reliably from process memory. Atomic merge preserves older rows if a refresh fails midway.
- **Alternatives considered**: Keep one response blob with only a TTL change (does not refresh new data or support record-level merges); keep the symbol registry only in memory (misses symbols after restart); replace the whole history on each incremental fetch (loses old rows).
- **Evidence**: `src/cache_store.rs` currently stores `symbol`, `response_body`, `fetched_at`, `expires_at`, and `response_bytes`. `src/http/routes.rs` builds keys from provider namespace, normalized symbol, and for SPBEX the UTC date.

## Decision 2: Fetch only the missing date range during a user request

- **Decision**: Add provider-level history-since capability. For an uncached symbol, request full available history. For a cached symbol, request records after its latest retained date, then merge and return the complete retained history. Keep SPBEX's current UTC-date boundary for history responses.
- **Rationale**: This matches the clarified behavior while avoiding a complete source-history transfer on every request. Every history request still performs a source fetch, so it does not serve a TTL-fresh but out-of-date response.
- **Alternatives considered**: Fetch full history on every request (captures source revisions immediately but increases source traffic and payload size); serve the stored response until expiration (violates the feature requirement).
- **Evidence**: The SPBEX client already accepts Unix `from` and `to` bounds (`src/spbex/client.rs`). CBR's official XML interface accepts a date range (`date_req1`, `date_req2`, `VAL_NM_RQ`) in its [SXML documentation](https://www.cbr.ru/development/sxml/). MOEX's [ISS reference](https://iss.moex.com/iss/reference/) describes security history queries over date intervals; its current client uses paginated unbounded history requests (`src/moex/client.rs`).

## Decision 3: Run a durable, bounded-concurrency background full refresh

- **Decision**: Start a Tokio worker with the application. It enumerates all persisted provider/symbol pairs and refreshes each pair according to `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS` (positive seconds, default `604800`). A never-successfully-refreshed pair is due immediately. When a failed pair has a persisted `next_refresh_attempt_at`, that time overrides the regular cadence: retryable failures use exponential backoff, while other HTTP 4xx failures are deferred until one configured interval after that failed attempt. After success, clear retry state and resume the regular cadence from the successful refresh time. Wake the scheduler for the nearest due attempt, run work with bounded provider concurrency, and update success metadata only after a successful atomic merge. Stop and join the worker as part of graceful shutdown.
- **Rationale**: The user's examples require refreshing SBER, SIBN, and ROSN without a new user request. Persisted enumeration and due timestamps make this work across restarts; a concurrency bound protects public upstreams.
- **Alternatives considered**: Refresh only when a user asks for that symbol (does not satisfy unattended refresh); refresh every symbol at process startup regardless of due date (excessive calls on frequent restarts); make all calls unbounded concurrent (risks provider overload).
- **Evidence**: `src/main.rs` currently owns server and SIGINT lifecycles but has no worker. Existing shutdown is bounded to 30 seconds (`src/shutdown.rs`).

## Decision 4: Configure a positive full-refresh interval, defaulting to seven days

- **Decision**: Parse `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS` as a positive integer, default `604800`, and use the configured interval for persisted due checks.
- **Rationale**: This implements the clarified operator control while keeping seven days as the normal cadence. Persisting the last successful refresh timestamp makes changes to the setting effective across restarts.
- **Alternatives considered**: Hard-code seven days (ignores operator configuration); configure a timestamp schedule (more complex and timezone-dependent for no added value).
- **Evidence**: Existing shared application settings are parsed in `src/config.rs` and use the `EXCHANGE_API_*` prefix required by the canonical project specification.

## Decision 5: Preserve history indefinitely; remove TTL and stop size limits from deleting durable rows

- **Decision**: Remove expiry deletion and oldest-first/oversized-response deletion from the persistent store. Remove `EXCHANGE_API_HISTORY_CACHE_TTL_SECS` and fail startup with a clear message directing operators to `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS` when the old variable remains set. Keep `EXCHANGE_API_HISTORY_CACHE_MAX_BYTES` as a transient/in-memory budget only; it must not limit or evict durable rows. On disk exhaustion, return a store error and preserve previously committed rows.
- **Rationale**: Any automatic TTL, capacity eviction, or deletion of a single oversized response contradicts the explicit requirement that history remain indefinitely absent operator deletion. A failed insert should not delete older records. Rejecting the old TTL setting avoids deployments believing it still controls freshness.
- **Alternatives considered**: Keep eviction and relax indefinite retention (contradicts the feature spec); accept and ignore the TTL setting (rejected by clarification); silently ignore cache-size limits without documenting the configuration change (misleads operators); return stale data after store failure (violates the shared history-store error contract).
- **Operational tradeoff**: Durable storage grows with every tracked symbol and record. Operators must provision and monitor persistent disk and remove the store only when intentionally discarding history.

## Decision 6: Use date as the existing daily record identity and merge atomically

- **Decision**: Treat `(provider, normalized symbol, record date)` as the stable identity. Replace a matching date when a source refresh returns revised values, append unseen dates, sort according to provider contract, and commit records plus refresh metadata atomically. An empty incremental result leaves retained records unchanged; an empty initial full fetch returns an empty collection.
- **Rationale**: All six history routes expose daily history and a date field. A source refresh returning duplicate dates must not create duplicate JSON records, and background work must not partially replace known-good history.
- **Alternatives considered**: Use array position as identity (unstable when upstream inserts records); append without deduplication (creates duplicates); delete stored rows before parsing the full refresh (data loss on invalid source payload).

## Decision 7: Preserve route contracts and update operational documentation

- **Decision**: Keep route paths, six-field records, symbol validation, and existing provider/store error envelopes. Update OpenAPI descriptions and quickstarts to document incremental source refresh, indefinite retention, the full-refresh interval variable/default, and background behavior.
- **Rationale**: The feature changes freshness and persistent lifecycle, not the public response schema. Contract documentation still needs to state these observable behaviors and the new configuration interface.
- **Alternatives considered**: Introduce new endpoints or response fields (unnecessary compatibility change); leave documentation describing TTL expiration (contradicts implementation).

## Decision 8: Retry background full-refresh failures with a configurable cap

- **Decision**: Classify provider failures for background refreshes as retryable transport/timeouts, HTTP 408, 425, 429, all 5xx responses, and invalid/incomplete full-refresh data; retry these with exponential delays starting at one second. Defer other HTTP 4xx responses, including permanent symbol rejection, until one configured interval after the failed attempt. Cap retry delays with positive integer `EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS`, default `900`; retry attempt count is unbounded until a valid refresh succeeds, while each delay remains capped. Persist retry failure count and next-attempt time, and reset retry state on success. Provider-specific invalid-symbol errors must remain identifiable to the background worker while public route error envelopes stay unchanged.
- **Rationale**: Temporary outages such as HTTP 502 must not discard usable cached data or prevent eventual reconciliation. A configurable cap lets operators balance recovery speed against upstream load.
- **Alternatives considered**: Stop retrying until the next configured sweep (can leave data stale throughout an outage); retry without a delay cap (can overload upstream services); hard-code the cap (removes operator control).

## Decision 9: Validate a full response before committing any part of it

- **Decision**: Fetch the complete provider result and map all records, then validate required fields, usable dates, and uniqueness of provider/symbol/date identities before one atomic merge of records and success metadata. Reject any duplicate identity, including identical duplicate values. For MOEX, completion requires every offset implied by the cursor total/page size to succeed with consistent columns and the expected number of valid row arrays for that page; for date-range CBR and SPBEX responses, completion requires a successful fully consumed response body that parses and maps in full. On any retryable failure, preserve the last good collection and last-success timestamp.
- **Rationale**: Partial or malformed upstream responses must not create a mixed collection or mark an unsuccessful refresh as complete.
- **Alternatives considered**: Commit per page (exposes partial refreshes); replace old rows before validation (risks data loss); accept malformed dates or duplicates (breaks record identity and ordering).
