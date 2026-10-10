# Implementation Plan: MOEX Benchmark and Currency Instrument Support

**Branch**: `007-moex-instrument-coverage` | **Date**: 2026-10-10 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification at `specs/007-moex-instrument-coverage/spec.md`

## Summary

Extend the existing MOEX provider to resolve an instrument's engine, market, and primary board, then use those values for history and current-value requests. Keep the existing six-field API response, board selection by trading date, and shared v1/v2 routes. Add explicit index quote mapping for IMOEX and currency-market quote mapping for GLDRUB_TOM. Before the first history refresh for a newly accepted symbol, ensure any pre-existing cached MOEX history is represented in the durable history collection; retain the legacy source row if migration fails.

## Technical Context

**Language/Version**: Rust 2024 edition; minimum Rust 1.85, repository toolchain 1.89.

**Primary Dependencies**: Existing Axum 0.8, Tokio 1.53, Reqwest 0.13 with Rustls, SQLx 0.8 with SQLite, Serde/serde_json, chrono/chrono-tz, tracing, and Wiremock for fixture-backed tests. No new dependency is expected.

**Storage**: Existing SQLite history store at `EXCHANGE_API_HISTORY_CACHE_DB_PATH`, with `(provider, symbol)` collections and date-keyed records. Existing legacy MOEX response rows use `MOEX:{symbol}` keys in `history_cache`; migration must be idempotent and preserve rows until their records have been committed to the current collection.

**Testing**: Rust unit, provider-client, API contract, and SQLite migration/integration coverage with Wiremock fixtures. Run `cargo test` during implementation; validate the v1 and v2 MOEX history and quote routes. Existing project release gates still require a Linux amd64 image build, Semgrep, and Trivy.

**Target Platform**: Existing Linux container service for amd64 and arm64.

**Project Type**: Single Rust REST microservice.

**Performance Goals**: Retain the shared MOEX route target: at least 95% of successful requests complete end to end in under one second at 10 concurrent clients and 10 requests per second total. Quote requests remain uncached. Report the first full history load separately from the established cached-history profile.

**Constraints**: Use public MOEX ISS data without subscriber credentials. Do not alter existing route paths or record fields. Resolve board and market before interpreting source columns; board identity must be retained when more than one board exists for a symbol. Primary-board effective-date intervals have valid dates, inclusive endpoints, allow gaps and open-ended ends, and must not overlap; reversed intervals or overlaps make metadata unusable and return the standard MOEX dependency error. A missing source value becomes `null`; incomplete or malformed source data remains a dependency error. History uses the primary board applicable on each trading date. Quote behavior stays uncached and provider-specific: trade date and price come from the latest trade for equities and currency instruments, while `volume` reports fetch-time `NUMTRADES`; index quotes retain the latest published-value mapping.

Newly supported benchmark and currency symbols use one or more alphanumeric segments separated by single underscores after the existing normalization rules; leading, trailing, and repeated underscores are invalid. Existing supported symbols keep their current validation behavior. For history, syntax validation is followed by MOEX metadata recognition: an unrecognized symbol returns HTTP 400, and metadata failure returns the standard MOEX dependency error; neither outcome migrates cache data. After recognition, migrate the symbol's legacy history before collection lookup or history retrieval, and reuse the resolved instrument context for the history request.

**Scale/Scope**: Add MOEX index-market and currency-market support to one existing provider. Support IMOEX and GLDRUB_TOM as required examples and other symbols resolved by MOEX metadata. Retain existing MOEX equity behavior, cache semantics, and shared route behavior. No new public route or service is required.

## Constitution Check

- **REST API Contracts First**: PASS. The established v1/v2 routes, six-field history records, quote record shape, and error behavior remain explicit in the contract.
- **Documentation Is Part of Delivery**: PASS. The OpenAPI contract describes the added instrument categories; quickstart records fixture and live-route validation expectations.
- **Clear Microservice Boundaries**: PASS. MOEX metadata discovery, source access, and mapping remain within the existing MOEX provider boundary.
- **Compatibility and Change Management**: PASS. Existing equities, routes, response fields, and provider-specific errors remain compatible. Existing cached history is preserved through migration.
- **Practical Quality and Operability**: PASS. The design includes malformed-source and storage-failure behavior, structured request logging through existing routes, fixture-backed tests, persistence validation, existing performance acceptance, and existing release security scans.
- **Private-network security posture**: PASS. No new authentication is introduced; the provider continues to make unauthenticated public ISS requests without subscriber credentials.

**Pre-Research Gate**: PASS. No constitution exceptions are identified.

## Project Structure

### Documentation (this feature)

```text
specs/007-moex-instrument-coverage/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── openapi.yaml
└── tasks.md                 # Generated by $speckit-tasks
```

### Source Code (repository root)

```text
src/
├── moex/
│   ├── client.rs            # Market-aware ISS history and current-value requests
│   ├── provider.rs          # Preflight and retain instrument context; choose history/quote mapping
│   ├── board.rs             # Primary board and market assignment by date
│   ├── mapping.rs           # Named, market-aware source field mapping
│   ├── index.rs             # Index history and published-value quote mapping
│   ├── currency.rs          # Currency history and last-trade quote mapping
│   ├── models.rs            # ISS metadata, history, and quote source shapes
│   └── validation.rs        # Existing validation plus segmented underscores for new categories
├── cache_store.rs           # Per-symbol legacy-history import into durable collections
└── http/routes.rs           # Syntax-check, preflight provider, migrate, refresh, respond

tests/
├── moex_client.rs           # Metadata, history, trades, and marketdata URLs and pagination
├── moex_mapping.rs          # Shared market context and nullable-field mapping
├── moex_index.rs            # Index source mapping and v1/v2 history/quote routes
├── moex_currency.rs         # Currency source mapping and v1/v2 history/quote routes
├── api_moex.rs              # Equity regressions and first-request migration route behavior
├── cache_store.rs           # Idempotent migration and failure preservation
└── fixtures/moex/           # Index and currency ISS response fixtures

specs/contracts/openapi.yaml # Canonical API descriptions for supported MOEX categories
```

**Structure Decision**: Extend the existing Rust service and test layout. Keep shared metadata resolution, client transport, board selection, and common mappings in their current modules; use `index.rs` and `currency.rs` for category-specific record mapping. For history, the common route first validates symbol syntax and asks the provider to preflight the MOEX instrument. The provider distinguishes unknown metadata from upstream failure and retains the resolved context for the subsequent history call. Only a recognized instrument proceeds to targeted legacy-cache import; the route then reads the collection, incrementally refreshes history with the retained context, atomically merges records, and responds. This avoids importing data for unsupported symbols and avoids a second metadata request. Route tests verify unknown symbols and metadata failures do not migrate cache, while recognized symbols return migrated records together with newly fetched records; a failed migration preserves the legacy source and returns the history-store error.

## Phase 0: Outline & Research

Research decisions and alternatives are recorded in [research.md](research.md). Verified MOEX ISS endpoints and field differences cover IMOEX and GLDRUB_TOM. No unresolved technical clarification remains.

## Phase 1: Design & Contracts

- [data-model.md](data-model.md) describes MOEX instrument context, segmented symbol validation for newly supported categories, board assignments, market records, quotes, and cached-history migration state.
- [contracts/openapi.yaml](contracts/openapi.yaml) references the canonical versioned API paths; the canonical `specs/contracts/openapi.yaml` descriptions cover supported instrument examples and fetch-time trade-count quote volume without changing response schemas.
- [quickstart.md](quickstart.md) describes fixture-backed and live validation scenarios for history, category-specific current-marketdata requests, cache migration and response merging, and backward compatibility.

### Constitution Re-check

- Versioned REST routes and existing response/error schemas remain documented: **PASS**.
- Source/field mapping and operator-facing validation guidance are documented: **PASS**.
- Existing provider ownership and service boundary are retained: **PASS**.
- Cached data is transactionally preserved through the existing durable history model: **PASS**.
- Required automated checks, latency target, amd64 build, and Semgrep/Trivy release checks remain applicable: **PASS**.

**Post-Design Gate**: PASS. This plan changes upstream source selection and migration timing while preserving public response compatibility and service ownership.

## Complexity Tracking

No constitution violations or additional services are required. Market-aware metadata and ISS paths extend the current provider. Targeted legacy-history import uses the current persistent store.
