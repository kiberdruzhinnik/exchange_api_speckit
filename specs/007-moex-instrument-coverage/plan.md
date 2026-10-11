# Implementation Plan: MOEX Benchmark and Currency Instrument Support

**Branch**: `007-moex-instrument-coverage` | **Date**: 2026-10-10 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification at `specs/007-moex-instrument-coverage/spec.md`

## Summary

Extend the existing MOEX provider to resolve an instrument's engine, market, and primary board, then use those values for history and current-value requests. Keep the existing six-field API response, board selection by trading date, and provider-qualified v2 routes. Add explicit index quote mapping for IMOEX and currency-market quote mapping for GLDRUB_TOM. Legacy serialized response-cache rows are discarded during store upgrade; history is fetched from MOEX and retained in durable collections.

## Technical Context

**Language/Version**: Rust 2024 edition; minimum Rust 1.85, repository toolchain 1.89.

**Primary Dependencies**: Existing Axum 0.8, Tokio 1.53, Reqwest 0.13 with Rustls, SQLx 0.8 with SQLite, Serde/serde_json, chrono/chrono-tz, tracing, and Wiremock for fixture-backed tests. No new dependency is expected.

**Storage**: Existing SQLite history store at `EXCHANGE_API_HISTORY_CACHE_DB_PATH`, with `(provider, symbol)` collections and date-keyed records. Legacy response-cache rows and their migration markers are dropped on upgrade; existing durable collections remain intact.

**Testing**: Rust unit, provider-client, API contract, and SQLite migration/integration coverage with Wiremock fixtures. Run `cargo test` during implementation; validate the v2 MOEX history and quote routes. Existing project release gates still require a Linux amd64 image build, Semgrep, and Trivy.

**Target Platform**: Existing Linux container service for amd64 and arm64.

**Project Type**: Single Rust REST microservice.

**Performance Goals**: Retain the shared MOEX route target: at least 95% of successful requests complete end to end in under one second at 10 concurrent clients and 10 requests per second total. Quote requests remain uncached. Report the first full history load separately from the established cached-history profile.

**Constraints**: Use public MOEX ISS data without subscriber credentials. Do not alter existing route paths or record fields. Resolve board and market before interpreting source columns; board identity must be retained when more than one board exists for a symbol. Primary-board effective-date intervals have valid dates, inclusive endpoints, allow gaps and open-ended ends, and must not overlap; reversed intervals or overlaps make metadata unusable and return the standard MOEX dependency error. A missing source value becomes `null`; incomplete or malformed source data remains a dependency error. History uses the primary board applicable on each trading date. Quote behavior stays uncached and provider-specific: trade date and price come from the latest trade for equities and currency instruments, while `volume` reports fetch-time `NUMTRADES`; index quotes retain the latest published-value mapping.

Newly supported benchmark and currency symbols use one or more alphanumeric segments separated by single underscores after the existing normalization rules; leading, trailing, and repeated underscores are invalid. Existing supported symbols keep their current validation behavior. For history, syntax validation is followed by MOEX metadata recognition: an unrecognized symbol returns HTTP 400, and metadata failure returns the standard MOEX dependency error; neither outcome migrates cache data. After recognition, migrate the symbol's legacy history before collection lookup or history retrieval, and reuse the resolved instrument context for the history request.

**Scale/Scope**: Add MOEX index-market and currency-market support to one existing provider. Support IMOEX and GLDRUB_TOM as required examples and other symbols resolved by MOEX metadata. Retain existing MOEX equity behavior, cache semantics, and shared route behavior. No new public route or service is required.

## Constitution Check

- **REST API Contracts First**: PASS. The established v2 routes, six-field history records, quote record shape, and error behavior remain explicit in the contract.
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
├── cache_store.rs           # Durable history collections and refresh metadata
└── http/routes.rs           # Syntax-check, preflight provider, migrate, refresh, respond

tests/
├── moex_client.rs           # Metadata, history, trades, and marketdata URLs and pagination
├── moex_mapping.rs          # Shared market context and nullable-field mapping
├── moex_index.rs            # Index source mapping and v2 history/quote routes
├── moex_currency.rs         # Currency source mapping and v2 history/quote routes
├── api_moex.rs              # Equity regressions and first-request migration route behavior
├── cache_store.rs           # Idempotent migration and failure preservation
└── fixtures/moex/           # Index and currency ISS response fixtures

specs/contracts/openapi.yaml # Canonical API descriptions for supported MOEX categories
```

**Structure Decision**: Extend the existing Rust service and test layout. Keep shared metadata resolution, client transport, board selection, and common mappings in their current modules; use `index.rs` and `currency.rs` for category-specific record mapping. The provider-qualified v2 history route validates symbol syntax, preflights the MOEX instrument, fetches history, and merges validated records into the durable collection. Legacy serialized response-cache data is discarded on store upgrade and is never imported.
