# Data Model: Permanent History Cache Refresh

## History Collection

Represents retained daily history for one normalized symbol at one provider.

| Field | Meaning | Rules |
|---|---|---|
| provider | Provider namespace (`moex`, `spbex`, or `cbr`) | Required; part of the identity |
| symbol | Provider-normalized symbol | Required; part of the identity |
| records | Retained daily source records | Zero or more, unique by date, ordered by provider contract |
| latest_record_date | Newest date among retained records | Empty when there are no records; incremental request starts after this date |
| last_full_refresh_at | Completion time of the last successful full source refresh | Empty until the first successful full refresh; used to determine whether the collection is due |
| consecutive_refresh_failures | Consecutive retryable full-refresh failures | Incremented for retryable failures; reset to zero after a successful validated full refresh |
| next_refresh_attempt_at | Earliest time the collection may be attempted again | Persisted due time; retryable errors use capped backoff, other HTTP 4xx failures use one configured interval after the failed attempt |
| updated_at | Time the collection was last successfully merged | Updated only after a successful atomic merge |

Collection identity: `(provider, symbol)`. The implementation must migrate legacy composite keys without dropping their stored response bodies: `MOEX:{symbol}` and `CBR:{symbol}` map to their provider and symbol, while all `SPBEX:{symbol}:{UTC-date}` rows map to one SPBEX collection per normalized symbol. Migration must merge same-date records by the normal date identity. SPBEX's current date suffix is a response-boundary detail and must not create separate durable collections for the same symbol.

## History Record

Represents one provider's daily history row.

| Field | Meaning | Rules |
|---|---|---|
| date | Source record date/time under the existing provider contract | Required for identity and ordering |
| close | Provider-mapped close/rate value | Nullable as defined by provider contract |
| high | Provider-mapped high value | Nullable as defined by provider contract |
| low | Provider-mapped low value | Nullable as defined by provider contract |
| volume | Provider-mapped volume | Nullable as defined by provider contract |
| facevalue | Existing mapped facevalue/lot-size/nominal value | Nullable as defined by provider contract |

Record identity: `(provider, symbol, date)`. A successful source result replaces a retained record with the same identity and adds unseen identities. No failed request or failed background refresh changes committed records.

## Refresh Lifecycle

1. A first request has no collection and fetches full source history; successful results create the collection.
2. A subsequent user request reads its latest date, fetches only later source records, merges them, and responds with the complete retained collection.
3. The background worker enumerates all collections and full-refreshes those due according to the configured positive interval, defaulting to seven days. Newly created collections are eligible immediately. A pending `next_refresh_attempt_at` takes precedence over the normal interval; the worker wakes for the nearest due attempt.
4. A full response is completely fetched and validated before any record or success metadata is committed. MOEX must provide every cursor-indicated page with the expected number of rows; CBR and SPBEX date-range responses must be fully consumed, parseable, and mappable. Any duplicate identity, including identical duplicate values, invalidates the whole result. A successful full refresh reconciles all dates, including revisions to existing records. It does not delete retained records simply because they are absent from a partial/empty response; missing-record deletion is not specified.
5. A retryable transport/status/validation failure preserves the collection and last-success time, increments the persisted failure count, and schedules an exponential-backoff retry. Other HTTP 4xx failures preserve data and defer the next attempt until one configured interval after the failed attempt. Success resets retry state and restarts the normal cadence from the successful refresh time.
6. User-request source errors retain existing route-specific HTTP 502 behavior. Background source failures leave prior data intact. Store failures preserve committed rows and surface the existing HTTP 503 on request paths.

## Configuration

`EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS` is a positive integer in seconds and defaults to `604800` (seven days). Any positive value, including values longer than seven days, is honored across all providers. Invalid or zero values fail application startup with a clear configuration error.

`EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS` is a positive integer in seconds and defaults to `900`. Background retries start at one second, double after each retryable failure, and stop growing at this configured cap.
