# Canonical System Specification

**Status:** Current system requirements, consolidated from the MOEX, SPBEX, and CBR feature specifications.  
**Scope:** The six provider history and quote routes and shared service behavior.

The three feature task lists are fully checked, and their feature specs are sealed. The historical feature specs are preserved unchanged.

## Shared REST Contract

- **SHARED-FR-001** *(CBR FR-020, FR-021)*: The API MUST expose `GET /v1/{provider}/{SYMBOL}` and `GET /v1/{provider}/{SYMBOL}/quote` for `moex`, `spbex`, and `cbr`. All success records MUST use exactly `date`, `close`, `high`, `low`, `volume`, and `facevalue` in JSON arrays.
- **SHARED-FR-002** *(CBR FR-022)*: Providers MUST trim surrounding whitespace and uppercase symbols before validation. Malformed or unsupported symbols MUST return HTTP 400 with `{"error":{"code":"invalid_symbol","message":"..."}}`.
- **SHARED-FR-003** *(CBR FR-022, FR-024)*: Upstream failures or unusable upstream data MUST return HTTP 502 in the shared error envelope, retaining the existing route-specific codes `moex_unavailable`, `spbex_unavailable`, or `cbr_unavailable`. History-store failures MUST return HTTP 503 with `history_store_unavailable` in the same envelope.
- **SHARED-FR-004** *(MOEX FR-007; SPBEX FR-006; CBR FR-008)*: Successful empty history MUST return `[]`. A successful quote lookup with no quote MUST return HTTP 200 and a one-element array whose six fields are all `null`.
- **SHARED-FR-005** *(MOEX FR-010; SPBEX FR-014; CBR FR-016)*: API documentation MUST describe routes, symbols, provider field mappings, empty/no-quote outcomes, and error behavior consistently.

## Provider Data Requirements

### MOEX

- **MOEX-FR-001** *(MOEX FR-001–FR-006, FR-011)*: `GET /v1/moex/{SYMBOL}` MUST return all daily history available from public MOEX ISS without subscriber credentials, ordered oldest to newest. Each row MUST come from the board primary for that trading date. History dates MUST preserve the MOEX trading date as midnight UTC. Unavailable market values MUST be `null`; a missing or invalid trading date makes upstream data unusable.
- **MOEX-FR-002** *(MOEX FR-005; clarified SBER correction)*: `close`, `high`, `low`, and `volume` MUST map from the daily history fields. The JSON field remains named `facevalue` but MUST carry the current `LOTSIZE` for the board primary on that record’s date, or `null` if unavailable. It MUST NOT use MOEX `FACEVALUE`; the expected SBER value for 2026-10-06 is `1`.
- **MOEX-FR-003** *(MOEX FR-014)*: Each `/quote` request MUST fetch the latest executed trade from MOEX ISS without reusing a cached quote. Return one history-shaped record mapping UTC trade time to `date`, trade price to `close`, and traded size to `volume`; set `high`, `low`, and `facevalue` to `null`. A valid no-trade response is the shared all-null record.

### SPBEX

- **SPBEX-FR-001** *(SPBEX FR-001–FR-006, FR-016)*: `GET /v1/spbex/{SYMBOL}` MUST return public chart-feed candles in ascending order, preserving source timestamps normalized to UTC. The history route MUST return only candles dated before the current UTC calendar date; current-date data is reserved for `/quote`. A successful empty feed is valid empty history. Return HTTP 400 for malformed symbols or explicit upstream rejection, not merely an empty feed.
- **SPBEX-FR-002** *(SPBEX FR-005)*: Map source `close`, `high`, and `low`; return `volume: null` because the feed does not supply volume, and `facevalue: 1`.
- **SPBEX-FR-003** *(SPBEX FR-007–FR-009)*: Each `/quote` request MUST fetch the latest available daily candle from the feed without reusing a prior quote. Return it as one six-field record with the same SPBEX mappings. A successful empty quote lookup returns the shared all-null record.

### Central Bank of Russia (Bank of Russia)

- **CBR-FR-001** *(CBR FR-001–FR-008)*: `GET /v1/cbr/{SYMBOL}` MUST return all source-available daily rates from the earliest available date through the latest published date, oldest first. Symbols MUST be present in the Bank of Russia’s supported-currency list.
- **CBR-FR-002** *(CBR FR-006, FR-007, FR-007a)*: Serialize effective dates at midnight UTC. `close` MUST be Russian rubles per one currency unit (`Value / Nominal`); if either input is absent, `close` is `null`. `facevalue` MUST contain source nominal when available. CBR does not provide `high`, `low`, or `volume`, so these are `null`. A missing or invalid date or otherwise unusable record is an upstream failure.
- **CBR-FR-003** *(CBR FR-009–FR-011)*: Each `/quote` request MUST fetch a fresh official rate. Return the latest published rate and its effective date, even when that date precedes today. If a successful source response has no rate, return the shared all-null quote record.

## Shared Freshness, Performance, and Operations

- **SHARED-FR-006** *(MOEX FR-013; SPBEX FR-013; CBR FR-014)*: Unexpired history MUST survive application restarts. Expired history MUST be refreshed before it is returned; the specification does not prescribe a storage mechanism or freshness duration.
- **SHARED-FR-007** *(MOEX FR-014; SPBEX FR-008; CBR FR-010)*: Quote responses MUST be fetched for each request and MUST NOT be served from a prior quote response.
- **SHARED-FR-008** *(MOEX FR-012; SPBEX FR-012; CBR FR-017)*: Under 10 concurrent clients issuing 10 requests per second total, at least 95% of successful responses on each of the six routes MUST complete end to end in under one second. This is a hard acceptance gate; a miss requires optimization and another measurement.
- **SHARED-FR-009** *(MOEX FR-015; SPBEX FR-014; CBR FR-018)*: The release MUST successfully build a Linux amd64 container image. AMD64 acceptance requires build success only, not runtime or route testing.
- **SHARED-FR-010** *(MOEX FR-016; CBR FR-019)*: On Ctrl+C (SIGINT), the service MUST stop accepting new requests, allow in-flight work to finish, and exit within 30 seconds. Remaining work MUST be cancelled when the deadline expires.
- **SHARED-FR-011** *(CBR FR-023, FR-025)*: Every application configuration environment variable MUST begin with `EXCHANGE_API_`. Shared settings MUST apply consistently across providers; source and response-size settings MAY be provider-specific. Current names are `EXCHANGE_API_LISTEN_ADDR`, `EXCHANGE_API_REQUEST_TIMEOUT_SECS`, `EXCHANGE_API_HISTORY_CACHE_TTL_SECS`, `EXCHANGE_API_HISTORY_CACHE_MAX_BYTES`, `EXCHANGE_API_HISTORY_CACHE_DB_PATH`, `EXCHANGE_API_MOEX_ISS_BASE_URL`, `EXCHANGE_API_MOEX_MAX_ISS_RESPONSE_BYTES`, `EXCHANGE_API_MOEX_MAX_HISTORY_BYTES`, `EXCHANGE_API_SPBEX_API_BASE_URL`, `EXCHANGE_API_SPBEX_MAX_RESPONSE_BYTES`, `EXCHANGE_API_CBR_API_BASE_URL`, and `EXCHANGE_API_CBR_MAX_RESPONSE_BYTES`.
- **SHARED-FR-012** *(project implementation behavior)*: `GET /health/live` reports process liveness. `GET /health/ready` reports readiness and returns HTTP 503 when the history store cannot be queried.

## Implementation Comparison and Open Contradictions

Source review found no behavioral mismatch between the current implementation and the newest applicable feature requirements. It has all six routes, a shared provider interface and record shape, normalized symbols, the specified status/error mappings, persistent history caching, uncached quote fetches, provider-specific field mappings, `EXCHANGE_API_*` configuration, and bounded SIGINT shutdown. Source also exposes the two health routes described above. No implementation changes or tests were run for this consolidation.

The feature quickstarts record final validation on 2026-10-09, including Linux amd64 build success and all six route p95 values below one second under the specified workload: MOEX history/quote p95 were 0.002227/0.398614 seconds; SPBEX 0.003834/0.751137 seconds; CBR 0.003090/0.042939 seconds. Combined-profile route p95 values were also below one second. These are recorded results, not measurements repeated during this consolidation.

1. **SPBEX date boundary:** SPBEX FR-016 says “current calendar date” without a timezone. The OpenAPI contract and implementation use the current UTC date. If “calendar date” means the exchange-local date, the specification and behavior may differ; the intended timezone is not explicit.
Historical feature specifications and their plans, tasks, and contracts remain unchanged.
