# Quickstart: Permanent History Cache Refresh

## Prerequisites

- Rust toolchain matching `Cargo.toml` (Rust 1.85 or newer).
- Persistent writable storage for the SQLite database configured by `EXCHANGE_API_HISTORY_CACHE_DB_PATH`.
- Fixture server or Wiremock fixtures for MOEX, SPBEX, and CBR upstream responses.

## Configure and start

```sh
EXCHANGE_API_HISTORY_CACHE_DB_PATH=./data/history.sqlite3 \
EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS=604800 \
EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS=900 \
cargo run
```

`EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS` is shared by all providers, is measured in seconds, accepts any positive value, and defaults to `604800` (seven days). The service tracks every provider/symbol pair whose history is stored. When a pair is due, it fetches complete history in the background without a user request. `EXCHANGE_API_HISTORY_CACHE_MAX_BYTES` only bounds transient in-memory use; it does not cap or evict SQLite history. Ensure the persistent volume can grow and monitor available disk space.

Retryable full-refresh failures (network errors, timeouts, HTTP 408, 425, 429, 5xx such as 502, and invalid or incomplete responses) keep the last valid collection and retry with exponential backoff starting at one second. Other HTTP 4xx errors are deferred until one configured interval after the failed attempt. A pending attempt time takes precedence over the regular cadence. `EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS` sets the positive-integer maximum delay in seconds and defaults to `900` (15 minutes). Retry count and next attempt are persisted across restarts; a successful refresh resets them. Each full refresh is validated as a whole before records or success metadata are committed. Any duplicate record identity, even with identical values, invalidates the refresh.

Remove `EXCHANGE_API_HISTORY_CACHE_TTL_SECS` from deployment configuration. If it is still set, startup fails with a migration message; use `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS` to configure full-refresh cadence.

## Validate the history lifecycle

1. Point one provider at a fixture containing two daily records and request its history route. Expect the normalized two-record six-field array and persisted collection metadata.
2. Change the fixture to return one new record after the stored latest date. Request the route again. Expect an upstream incremental request, a three-record response, and no duplicates.
3. Restart the service using the same database. Verify the complete history and provider/symbol refresh metadata remain present.
4. Configure a short positive interval in a fixture environment, seed multiple symbols across providers, and verify the worker full-refreshes all due collections without incoming HTTP requests.
5. Configure an interval longer than seven days and use a controlled clock. Verify a collection is not due just before its configured interval and is attempted when that interval elapses.
6. Return a revised older record from a scheduled full refresh. Verify the matching date is updated while other dates remain available.
7. Fail an incremental user request and a scheduled full refresh separately. Verify the user request receives the established provider-specific 502. For scheduled refreshes, exercise HTTP 408, 425, 429, representative 5xx including 502, and representative other 4xx responses; verify retryable cases use exponential backoff and other 4xx cases defer one configured interval from the failed attempt.
8. Return malformed/incomplete full-refresh data and duplicate identities with both identical and conflicting values. Verify last-good records and success time remain unchanged; a later valid response is atomically committed and resets retry state. Repeat with a custom cap and across a service restart.
9. Reopen a legacy cache database. Verify migration preserves every existing response and maps provider/symbol keys without age-based deletion.
10. Configure zero or a malformed full-refresh interval or retry cap. Verify startup fails with a clear setting-specific error.

Run the existing and new fixture-backed checks with:

```sh
cargo test
```

## Performance and operational acceptance

Run the established 10-client / 10-request-per-second workload for each of the six routes. Include first fetches, incremental upstream refreshes, warmed response assembly, due background refreshes, and combined traffic. At least 95% of successful responses per route must complete end to end under one second. Record actual request rate, p95, upstream errors, and background refresh outcomes. Optimize or repeat measurements if the gate is missed.

Confirm `GET /health/ready` returns 503 when the persistent store is unavailable. Send SIGINT during a scheduled sweep and verify the worker stops or cancels within the service's 30-second shutdown deadline. Before release, run Semgrep source analysis and Trivy against the built deliverable as required by the project constitution.

## Measured acceptance (2026-10-09)

Ran each provider harness with 10 concurrent clients, a 10 requests/second arrival schedule, and 5 seconds per history, quote, and combined stage. Every stage issued 50/50 scheduled requests with no HTTP errors. The combined stage ran with persistent history populated and the background refresh worker active.

| Provider | History p95 | Quote p95 | Combined history p95 | Combined quote p95 |
|---|---:|---:|---:|---:|
| MOEX | 0.620250 s | 0.427227 s | 0.761879 s | 0.458093 s |
| SPBEX | 0.421445 s | 0.368655 s | 0.489799 s | 0.571844 s |
| CBR | 0.133308 s | 0.030456 s | 0.074855 s | 0.037579 s |

All measured route p95 values were below the one-second acceptance target. CBR's incremental lifecycle probe returned HTTP 200 on all three requests and logged one source fetch per request. The harness outputs reported 10 issued requests/second for every stage (CBR observed issue rate 9.99–10.00 requests/second; MOEX and SPBEX observed start rate 10.20–10.21 requests/second).

## Release checks (2026-10-09)

- `docker build --platform linux/amd64 -t exchange-api:history-refresh .`: passed.
- Semgrep `scan --config auto` on the repository and an explicit `src/` plus `tests/` scan: 0 findings.
- Trivy vulnerability scan of `exchange-api:history-refresh`: 0 High or Critical findings; no fixable findings were reported at those severities.
