# Implementation Plan: CBR Currency Rates API

**Branch**: `003-cbr-currency-rates` | **Date**: 2026-10-09 | **Spec**:
[spec.md](spec.md)

**Input**: Feature specification from `specs/003-cbr-currency-rates/spec.md`

## Summary

Add Bank of Russia currency history and latest-rate endpoints to the existing
Rust/Axum service. Resolve supported ISO currency symbols from the official CBR
currency directory; retrieve daily history and current/latest rates from CBR's
public XML service. Normalize available closes to RUB per one currency unit
(`Value / Nominal`), preserve `Nominal` as `facevalue`, and return null when
rate inputs or optional source fields are unavailable. CBR does not supply high,
low, or volume. Reuse the shared durable history cache for history and bypass it
for fresh quotes. Keep the shared six-field history-shaped JSON schema, standard
error envelope, durable-history/fresh-quote behavior, and operational acceptance
consistent with MOEX and SPBEX. Preserve CBR-specific rate mapping and
latest-official-rate quote semantics. Document both contracts, verify bounded
SIGINT shutdown and a successful Linux amd64 image build, and validate behavior
against fixtures and production CBR under the specified hard p95 workload.

## Technical Context

**Language/Version**: Rust 2024, MSRV 1.85; repository toolchain 1.89.

**Primary Dependencies**: Existing Axum/Tokio service, reqwest with Rustls,
Serde, chrono, Moka, SQLx SQLite, tracing, and wiremock. CBR returns XML; use
the existing Serde-enabled `quick-xml` dependency. Add `async-trait` to express
a shared async provider interface stored as `Arc<dyn ExchangeProvider>` in
application state. No new runtime service is needed. Route orchestration
consumes normalized history and quote records and common provider error
categories, while each adapter owns source parsing and provider-specific quote
meaning.

**Storage**: Reuse `HistoryCache` and SQLite `CacheStore`; use
exchange-qualified keys such as `CBR:USD`. Configure the shared history TTL,
capacity, and database path through application-wide settings. Keep
provider-specific upstream response-size limits. Quote requests bypass both
memory and persistent cache layers.

**Testing**: Fixture-backed source parsing, normalization, supported-symbol
resolution, client and Axum route behavior for all three providers; OpenAPI
schema validation; durable-history restart and store-failure behavior;
production latency using the local optimized binary and each provider's
production source. Run the existing process-level `cargo test --test shutdown`
acceptance to verify SIGINT stops accepting requests, drains controlled
in-flight work, and cancels work at the 30-second deadline. The final feature
validation builds the Linux amd64 Docker image (build success only is required
for amd64), preserves arm64 support, and runs constitution-mandated Semgrep and
Trivy scans.

**Target Platform**: Existing Linux container service on port 8080. A successful
`linux/amd64` image build is required; amd64 runtime startup and route checks
are outside acceptance. Preserve existing `linux/arm64` support and use the
existing distroless nonroot runtime. Reuse the service-wide SIGINT shutdown
behavior: stop accepting new requests, drain in-flight work, and exit within 30
seconds, canceling remaining work at the deadline.

**Project Type**: Existing single Rust web service.

**Performance Goals**: At 10 concurrent clients and 10 total requests per
second, at least 95% of successful responses from each of the six MOEX, SPBEX,
and CBR history and quote routes must complete in under one second end to end.
This is a hard acceptance gate for the complete feature. Measure cache hits,
cold misses, and expiration for history plus uncached quotes, with
provider-specific and combined profiles. A run below the target request rate or
with skipped arrivals is invalid; optimize and repeat any valid profile that
misses p95.

**Constraints**: Use one shared provider interface across the three adapters and
the normalized `{date, close, high, low, volume, facevalue}` contract. History
returns every source row from the earliest source-available date for the
currency through the latest published date, oldest to newest; quote returns one
latest available rate from a fresh source request, with its actual effective
date. Validate the requested symbol against CBR's currently supported
daily-currency directory rather than a hard-coded list; directory data is cached
for 60 seconds, then refreshed. Normalize symbols by trimming and uppercasing.
All providers use HTTP 400 for malformed or unsupported symbols, HTTP 502 for
upstream failures, and HTTP 503 for history-store failures, with one JSON error
envelope. Preserve existing provider-specific upstream `code` values on current
`/v1` routes (`moex_unavailable`, `spbex_unavailable`, and `cbr_unavailable`) to
avoid breaking clients; share `invalid_symbol` and `history_store_unavailable`.
Return `[]` for valid empty history and one all-null record for a successful
quote with no data. Keep durable history across restarts until expiry and fetch
quotes fresh. Provider-specific quote meanings remain distinct: MOEX latest
executed trade, SPBEX latest daily candle, and CBR latest official rate. No
application authentication is required for the private-network deployment. At
final build, run Semgrep and Trivy; resolve all Semgrep findings and fixable
High/Critical Trivy findings.

**Scale/Scope**: One currency per request. History is fetched from CBR on cache
miss or expiry and persisted under an exchange-qualified key. Quotes fetch
current/latest CBR data on every request and are never cached. CBR is added to
the current service; no new deployable service or external database is
introduced.

## Shared Provider Interface and Configuration

Implement `ExchangeProvider` with async `history(symbol)` and `quote(symbol)`
operations returning `DailyMarketRecord` and `LatestQuoteRecord`. Store `Arc<dyn
ExchangeProvider>` adapters in shared application state. Each adapter normalizes
source data and maps source failures to provider-neutral invalid-symbol or
upstream error categories; shared route/cache orchestration maps those
categories to HTTP responses and handles persistent-store failures. Use the
neutral quote record name throughout, replacing the existing MOEX-specific
`LatestTradeRecord`. Centralize route orchestration, symbol trim/uppercase
handling, status mapping, error envelope, and empty/no-quote behavior. Preserve
established upstream error-code strings (`moex_unavailable`,
`spbex_unavailable`, `cbr_unavailable`) in the common adapter error context and
HTTP response mapping. Keep upstream parsing, provider-specific symbol
syntax/support checks, record mapping, and quote interpretation inside each
adapter. MOEX quotes remain latest trades, SPBEX quotes remain latest daily
candles, and CBR quotes remain latest official currency rates.

The external REST contract is shared across all providers: matching
history/quote route patterns, the same six-field JSON records and array
behavior, common HTTP status categories, and one error envelope. Preserve
provider-specific upstream code values on `/v1` routes; `invalid_symbol` and
`history_store_unavailable` are common codes. Keep provider-specific routes and
quote descriptions in the canonical `specs/contracts/openapi.yaml`; each
feature-level OpenAPI document references only its own two path items from that
canonical contract. The canonical contract is generated as a Phase 1 design
artifact in this plan. Update contract checks so the three providers cannot
drift.

Use one application-wide namespace for shared settings and remove the old names
without aliases:

| Setting | Canonical environment variable | Default |
| --- | --- | --- |
| Listener address | `EXCHANGE_API_LISTEN_ADDR` | `0.0.0.0:8080` |
| Upstream timeout | `EXCHANGE_API_REQUEST_TIMEOUT_SECS` | `15` |
| History cache TTL | `EXCHANGE_API_HISTORY_CACHE_TTL_SECS` | `60` |
| History cache capacity | `EXCHANGE_API_HISTORY_CACHE_MAX_BYTES` | `67108864` |
| History database path | `EXCHANGE_API_HISTORY_CACHE_DB_PATH` | `/var/lib/exchange-api/history.sqlite3` |

All application configuration environment variables use the `EXCHANGE_API_`
prefix; do not support aliases. Provider-specific values remain independently
configurable inside this namespace:

| Setting | Canonical environment variable | Default |
| --- | --- | --- |
| MOEX ISS base URL | `EXCHANGE_API_MOEX_ISS_BASE_URL` | `https://iss.moex.com/iss/` |
| MOEX maximum ISS response bytes | `EXCHANGE_API_MOEX_MAX_ISS_RESPONSE_BYTES` | `4194304` |
| MOEX maximum accumulated history bytes | `EXCHANGE_API_MOEX_MAX_HISTORY_BYTES` | `67108864` |
| SPBEX API base URL | `EXCHANGE_API_SPBEX_API_BASE_URL` | `https://spbexchange.ru/api/` |
| SPBEX maximum response bytes | `EXCHANGE_API_SPBEX_MAX_RESPONSE_BYTES` | `16777216` |
| CBR API base URL | `EXCHANGE_API_CBR_API_BASE_URL` | `https://www.cbr.ru/` |
| CBR maximum response bytes | `EXCHANGE_API_CBR_MAX_RESPONSE_BYTES` | `16777216` |

The application MUST read only these canonical names and MUST NOT accept
provider-only or unprefixed aliases. Update `src/config.rs`, its configuration
tests, measurement scripts, `docker-compose.yml`, and all provider setup
documentation so every application configuration variable follows this rule.
Historical benchmark records remain valid evidence for the settings and binaries
used at the time; future runs use the canonical names.

## Constitution Check

- **REST API Contracts First**: Pass after design. `contracts/openapi.yaml`
    specifies both CBR routes against reusable schemas and errors in the
    canonical `specs/contracts/openapi.yaml`; the canonical file covers all
    provider routes and the common six-field arrays, statuses, nullable fields,
    and error envelope.
- **Documentation Is Part of Delivery**: Pass after design. OpenAPI and
    `quickstart.md` cover source behavior, rate normalization, configuration,
    local validation, persistence, latency measurement, and container build.
- **Clear Microservice Boundaries**: Pass. CBR market data is a capability of
    the existing exchange API service and its public upstream dependency is
    documented.
- **Compatibility and Change Management**: Pass. CBR paths are additive.
    Existing MOEX and SPBEX paths, market data fields, and upstream error code
    values are preserved; shared statuses and envelope clarify common behavior
    without breaking current `/v1` clients.
- **Practical Quality and Operability**: Pass after design. Plan includes
    bounded XML reads, validation and shared error mapping, structured source
    diagnostics, fixture-backed tests, persistent-cache verification, production
    latency measurement, the existing 30-second SIGINT drain/cancel behavior,
    required amd64 image build, and final Semgrep/Trivy gates.
- **Private-network security posture**: Pass. Do not add application-level
    authentication; upstream TLS certificate and hostname verification remain
    enabled.

**Gate**: PASS. The specification and CBR source research resolve functional
decisions needed to design the feature.

**Post-Design Gate**: PASS. The design specifies versioned route contracts,
official supported-currency lookup, explicit rate normalization and
missing-field behavior, one provider interface and canonical shared contract,
canonical `EXCHANGE_API_*` names for every application setting with no aliases,
preserved `/v1` error-code compatibility, CBR-specific latest-rate quote
mapping, and the hard p95 gate across all six provider routes. It reuses the
existing bounded SIGINT shutdown and container architecture, with successful
Linux amd64 image build as the amd64 acceptance criterion.

## Project Structure

### Documentation (this feature)

```text
specs/003-cbr-currency-rates/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/openapi.yaml      # CBR-focused references into the canonical contract
└── tasks.md                 # generated by $speckit-tasks
specs/contracts/openapi.yaml    # canonical contract and common schemas for all providers
```

### Source Code (repository root)

```text
src/
├── config.rs                # shared and EXCHANGE_API_-prefixed provider configuration
├── domain.rs                # shared six-field public response types, including LatestQuoteRecord
├── provider.rs              # shared async provider interface and provider errors
├── lib.rs                   # application state with CBR client
├── main.rs                  # CBR client construction and shared SIGINT wiring
├── shutdown.rs              # shared bounded graceful shutdown coordinator
├── cache.rs                 # existing shared history cache
├── cache_store.rs           # existing SQLite durable history store
├── http/
│   ├── routes.rs            # shared route orchestration and provider handlers
│   └── errors.rs            # common error envelope and status mapping
└── cbr/
    ├── mod.rs
    ├── client.rs            # bounded CBR XML HTTP client
    ├── models.rs            # source directory and daily-rate models
    ├── mapping.rs           # source records to normalized market records
    └── validation.rs        # ISO symbol normalization and validation

tests/
├── api_cbr.rs
├── provider_contract.rs    # shared history/quote and provider error expectations
├── cbr_client.rs
├── cbr_mapping.rs
├── api_contract_schema.rs  # add CBR OpenAPI checks
└── fixtures/cbr/
    ├── currencies.xml
    ├── history.xml
    ├── latest.xml
    ├── empty.xml
    └── malformed.xml
```

**Structure Decision**: Keep one deployable Rust service and add `src/cbr/`
alongside the existing exchange adapters. Consolidate shared provider
abstraction, neutral `DailyMarketRecord`/`LatestQuoteRecord`, route/error
orchestration, configuration, cache behavior, and router conventions; retain
adapters for MOEX, SPBEX, and CBR source behavior. Put the canonical shared
OpenAPI components and all provider paths in `specs/contracts/openapi.yaml`,
with feature contracts referring to it. Prefix persistent history keys with
`CBR:` to prevent collisions with identical symbols on other exchanges. Quotes
bypass both rate-history cache layers. All application configuration names,
including source URL and response-size controls, begin with `EXCHANGE_API_`.

## Complexity Tracking

No constitution violations or extra services are required. CBR adds one source
adapter, route pair, and XML parsing dependency while using the existing storage
and API infrastructure.
