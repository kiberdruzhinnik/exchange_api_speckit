# Implementation Plan: SPBEX Ticker History and Quote API

**Branch**: `002-spbex-ticker-update` | **Date**: 2026-10-08 | **Spec**:
[spec.md](spec.md)

**Input**: Feature specification from `specs/002-spbex-ticker-update/spec.md`

## Summary

Add SPBEX daily history and latest-candle quote endpoints to the existing
Rust/Axum service. Use the public SPBEX chart feed, map its daily OHLC values
into the existing six-field record contract (`volume: null`, `facevalue: 1`),
persist successful history responses through the existing durable history cache,
and fetch uncached quote data on every request. Document the routes and errors
in an SPBEX OpenAPI contract and quickstart.

## Technical Context

**Language/Version**: Rust 2024, MSRV 1.85; repository toolchain 1.89.

**Primary Dependencies**: Existing Axum/Tokio HTTP stack, reqwest with Rustls,
Serde/serde_json, chrono, Moka, SQLx SQLite, tracing, and wiremock for tests. No
new runtime dependency is expected. The SPBEX reqwest client adds the official
Russian Trusted Root CA to its own verifier to validate the SPBEX TLS chain.

**Storage**: Reuse `HistoryCache` and `CacheStore` with exchange-qualified keys
such as `SPBEX:SBER`; use the existing history TTL and byte limits. Quote
results bypass both cache layers.

**Testing**: Cargo unit and integration tests, wiremock-based upstream fixtures,
Axum route tests, OpenAPI schema validation, restart-persistence checks, Docker
image build, and live latency acceptance with the local release binary and
production SPBEX source.

**Target Platform**: Existing Linux container service, port 8080; Docker Buildx
image must build for `linux/amd64` and retain `linux/arm64` support. Runtime
uses the existing distroless nonroot image.

**Project Type**: Existing single Rust web service.

**Performance Goals**: At 10 concurrent clients and 10 total requests per
second, p95 end-to-end latency for successful responses from each SPBEX route
must be under one second. This is a hard acceptance gate: history-only,
quote-only, and combined runs must each sustain the full request rate and meet
the p95 target. An under-rate run is invalid. If a valid run misses the target,
optimize the request/source path and repeat measurement; do not mark the feature
complete until it passes. Measure history cache hits, cold misses and expiry;
quotes always contact SPBEX.

**Constraints**: Use the public chart feed behavior in the referenced Go adapter
with daily resolution `1440` and current Unix `to`. History requests start at
`from=0`, but the response and cached payload exclude candles dated on the
current UTC calendar date; the current-date-qualified history cache key ensures
a prior day's filtered payload is not reused after UTC date rollover. Quote
requests start with a bounded 1-day lookback and double the lookback on empty
windows until a candle is found or the range reaches the Unix epoch, so
current-date candles remain available only through quote. The SPBEX client
limits concurrent upstream chart-feed requests to sixteen, which passed the
full-rate production latency gate and reduces source contention while keeping
quote p95 below one second. This retains the latest-available-candle semantics
for inactive symbols without downloading full history for every active-symbol
quote. Validate response size, JSON, timestamps, OHLC values, and duplicate
dates; sort history ascending. A successful empty feed remains valid empty
history/no-quote data; only malformed symbols or explicit source rejection
return HTTP 400. The feed omits volume, so return null. Quote requests are
always fresh and bypass both caches. Cache complete successful history responses
only; history reads the cache and contacts SPBEX on miss or expiry. Keep MOEX
routes and errors compatible. Do not add application authentication for this
private-network service. On final build, run Semgrep and Trivy; resolve all
Semgrep findings and fixable High/Critical Trivy findings.

**Scale/Scope**: One symbol per request. History uses one upstream chart-feed
response on a cache miss or expiry; quote requests perform one or more fresh
chart-feed reads, with bounded-lookback expansion when the recent window is
empty. SPBEX is added to the existing service; no new deployable service or
shared external database is required.

## Constitution Check

- **REST API Contracts First**: Pass after design. `contracts/openapi.yaml`
    defines both routes, six-field array shapes, status codes, empty/no-quote
    behavior, and errors.
- **Documentation Is Part of Delivery**: Pass after design. The contract and
    `quickstart.md` describe source behavior, route examples, configuration,
    local validation, persistence, latency measurement, and container use.
- **Clear Microservice Boundaries**: Pass. SPBEX support remains in the existing
    market-data service and has a documented public upstream dependency.
- **Compatibility and Change Management**: Pass. Only versioned SPBEX routes are
    added; MOEX routes retain their existing behavior and error codes.
- **Practical Quality and Operability**: Pass after design. Plan includes
    structured request outcomes, bounded upstream reads, failure mapping,
    fixture-backed tests, persistent history validation, latency measurement,
    amd64 image build, and final Semgrep/Trivy gates.
- **Private-network security posture**: Pass. No application authentication is
    added; upstream requests use the existing verified TLS client configuration.

**Gate**: PASS. No constitution violation or unresolved spec clarification
blocks design.

**Post-Design Gate**: PASS. The design defines versioned route contracts, keeps
SPBEX within the existing service boundary, distinguishes empty data from
explicit symbol rejection, uses adaptive quote lookback without caching, and
makes the production p95 target a hard acceptance gate with a required
optimization/retest loop. It also includes Docker build and final scanner
validation.

## Project Structure

### Documentation (this feature)

```text
specs/002-spbex-ticker-update/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/openapi.yaml
└── tasks.md                 # generated by $speckit-tasks
```

### Source Code (repository root)

```text
src/
├── config.rs                # SPBEX source URL and response-size configuration
├── domain.rs                # shared daily record and nullable quote record
├── lib.rs                   # application state with SPBEX client
├── main.rs                  # SPBEX client construction
├── cache.rs                 # existing shared history cache
├── cache_store.rs           # existing SQLite durable history store
├── http/
│   ├── routes.rs            # SPBEX history and quote handlers
│   └── errors.rs            # SPBEX dependency error mapping
└── spbex/
    ├── mod.rs
    ├── client.rs            # bounded public chart-feed HTTP client
    ├── models.rs            # source candle response model
    ├── mapping.rs           # validation and history/quote mapping
    └── validation.rs        # symbol syntax validation

tests/
├── api_spbex.rs
├── spbex_client.rs
├── spbex_mapping.rs
├── api_contract_schema.rs  # include SPBEX route/schema assertions
└── fixtures/spbex/
    ├── history.json
    ├── empty.json
    └── malformed.json
```

**Structure Decision**: Keep one deployable Rust service and parallel the
existing `src/moex/` separation for SPBEX transport, parsing, mapping, and
symbol checks. Reuse the shared cache and response shape. Prefix history cache
keys with the exchange to prevent collisions for symbols listed on both
exchanges. Add no separate persistence service or shared database.

## Complexity Tracking

No constitution violations or additional services are required. SPBEX uses the
existing cache boundary and adds only one upstream adapter and two versioned
routes.
