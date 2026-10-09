# Implementation Plan: CBR Currency Rates API

**Branch**: `003-cbr-currency-rates` | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/003-cbr-currency-rates/spec.md`

## Summary

Add Bank of Russia currency history and latest-rate endpoints to the existing Rust/Axum service. Resolve supported ISO currency symbols from the official CBR currency directory; retrieve daily history and current/latest rates from CBR's public XML service. Normalize available closes to RUB per one currency unit (`Value / Nominal`), preserve `Nominal` as `facevalue`, and return null when rate inputs or optional source fields are unavailable. CBR does not supply high, low, or volume. Reuse the shared durable history cache for history and bypass it for fresh quotes. Keep the shared six-field history-shaped JSON schema, standard error envelope, durable-history/fresh-quote behavior, and operational acceptance consistent with MOEX and SPBEX. Preserve CBR-specific rate mapping and latest-official-rate quote semantics. Document both contracts, verify bounded SIGINT shutdown and a successful Linux amd64 image build, and validate behavior against fixtures and production CBR under the specified hard p95 workload.

## Technical Context

**Language/Version**: Rust 2024, MSRV 1.85; repository toolchain 1.89.

**Primary Dependencies**: Existing Axum/Tokio service, reqwest with Rustls, Serde, chrono, Moka, SQLx SQLite, tracing, and wiremock. CBR returns XML; add a focused XML parser dependency (prefer `quick-xml` with Serde support) unless an existing compatible parser is identified during implementation. No new runtime service is needed.

**Storage**: Reuse `HistoryCache` and SQLite `CacheStore`; use exchange-qualified keys such as `CBR:USD`. Keep the existing history TTL and response-size limits. Quote requests bypass both memory and persistent cache layers.

**Testing**: Fixture-backed CBR XML parsing, normalization, supported-code resolution, client and Axum route behavior; OpenAPI schema validation; durable-history restart and store-failure behavior; production latency using the local optimized binary and `www.cbr.ru`. Run the existing process-level `cargo test --test shutdown` acceptance to verify SIGINT stops accepting requests, drains controlled in-flight work, and cancels work at the 30-second deadline. The final feature validation builds the Linux amd64 Docker image (build success only is required for amd64), preserves arm64 support, and runs constitution-mandated Semgrep and Trivy scans.

**Target Platform**: Existing Linux container service on port 8080. A successful `linux/amd64` image build is required; amd64 runtime startup and route checks are outside acceptance. Preserve existing `linux/arm64` support and use the existing distroless nonroot runtime. Reuse the service-wide SIGINT shutdown behavior: stop accepting new requests, drain in-flight work, and exit within 30 seconds, canceling remaining work at the deadline.

**Project Type**: Existing single Rust web service.

**Performance Goals**: At 10 concurrent clients and 10 total requests per second, at least 95% of successful responses from each CBR route must complete in under one second end to end. This shared target is a hard acceptance gate. Measure history cache hits, cold misses, and expiration, plus uncached quotes and combined load. A run below the target request rate is invalid; if a valid run misses p95, optimize and repeat until it passes before completion.

**Constraints**: Match normalized `{date, close, high, low, volume, facevalue}` contract. History returns every source row from the earliest source-available date for the currency through the latest published date, oldest to newest; quote returns one latest available rate from a fresh source request, with its actual effective date. Validate the requested symbol against CBR's currently supported daily-currency directory rather than a hard-coded list; directory data is cached for 60 seconds, then refreshed. Normalize symbols by trimming and uppercasing. Treat malformed or unsupported symbols as HTTP 400, source failures or unusable XML/rates as HTTP 502, and history-store errors as HTTP 503. Keep the common API conventions consistent with MOEX and SPBEX: identical six-field response shape, the same standard JSON error envelope, durable history across restarts until expiry, uncached fresh quotes, and shared latency, amd64-build, and SIGINT requirements. Provider-specific quote meanings remain distinct: MOEX returns the latest executed trade, SPBEX the latest daily candle, and CBR the latest official rate. No application authentication is required for the private-network deployment. At final build, run Semgrep and Trivy; resolve all Semgrep findings and fixable High/Critical Trivy findings.

**Scale/Scope**: One currency per request. History is fetched from CBR on cache miss or expiry and persisted under an exchange-qualified key. Quotes fetch current/latest CBR data on every request and are never cached. CBR is added to the current service; no new deployable service or external database is introduced.

## Constitution Check

- **REST API Contracts First**: Pass after design. `contracts/openapi.yaml` specifies both versioned routes, six-field arrays, status codes, nullable fields, and the shared JSON error shape.
- **Documentation Is Part of Delivery**: Pass after design. OpenAPI and `quickstart.md` cover source behavior, rate normalization, configuration, local validation, persistence, latency measurement, and container build.
- **Clear Microservice Boundaries**: Pass. CBR market data is a capability of the existing exchange API service and its public upstream dependency is documented.
- **Compatibility and Change Management**: Pass. Additive CBR paths leave MOEX and SPBEX routes and response schemas unchanged.
- **Practical Quality and Operability**: Pass after design. Plan includes bounded XML reads, validation and shared error mapping, structured source diagnostics, fixture-backed tests, persistent-cache verification, production latency measurement, the existing 30-second SIGINT drain/cancel behavior, required amd64 image build, and final Semgrep/Trivy gates.
- **Private-network security posture**: Pass. Do not add application-level authentication; upstream TLS certificate and hostname verification remain enabled.

**Gate**: PASS. The specification and CBR source research resolve functional decisions needed to design the feature.

**Post-Design Gate**: PASS. The design specifies versioned route contracts, official supported-currency lookup, explicit rate normalization and missing-field behavior, shared cache and error conventions, CBR-specific latest-rate quote mapping, and hard production p95 measurement. It reuses the existing bounded SIGINT shutdown and container architecture, with successful Linux amd64 image build as the amd64 acceptance criterion.

## Project Structure

### Documentation (this feature)

```text
specs/003-cbr-currency-rates/
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
├── config.rs                # CBR base URL and response-size configuration
├── domain.rs                # shared six-field public response types
├── lib.rs                   # application state with CBR client
├── main.rs                  # CBR client construction and shared SIGINT wiring
├── shutdown.rs              # shared bounded graceful shutdown coordinator
├── cache.rs                 # existing shared history cache
├── cache_store.rs           # existing SQLite durable history store
├── http/
│   ├── routes.rs            # CBR history and quote handlers
│   └── errors.rs            # CBR dependency error mapping
└── cbr/
    ├── mod.rs
    ├── client.rs            # bounded CBR XML HTTP client
    ├── models.rs            # source directory and daily-rate models
    ├── mapping.rs           # source records to normalized market records
    └── validation.rs        # ISO symbol normalization and validation

tests/
├── api_cbr.rs
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

**Structure Decision**: Keep one deployable Rust service and add `src/cbr/` alongside the existing exchange adapters. Reuse shared `DailyMarketRecord`, `LatestTradeRecord`, `HistoryCache`, `CacheStore`, error envelope, router conventions, and bounded SIGINT shutdown. Prefix persistent history keys with `CBR:` to prevent collisions with identical symbols on other exchanges. Quotes bypass both rate-history cache layers.

## Complexity Tracking

No constitution violations or extra services are required. CBR adds one source adapter, route pair, and XML parsing dependency while using the existing storage and API infrastructure.
