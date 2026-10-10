# Main Implementation Plan

> **Revision**: 2026-10-09 — Added the durable history collection, incremental refresh, scheduled refresh worker, retry, and validation architecture for all providers.
> **Revision**: 2026-10-09 — Resolved the MOEX plan’s conflicting durable-cache scalar in favor of the current indefinite, date-keyed SQLite history model: the older plan specified complete serialized responses with a 60-second TTL and 64 MiB durable capacity [Source: specs/001-moex-ticker-update/plan.md -> "Storage"]; the current plan retains date-keyed records indefinitely with collection metadata and no automatic eviction [Source: specs/004-history-cache-refresh/plan.md -> "Storage"]. This choice follows the user’s archive clarification.
> **Revision**: 2026-10-09 — Resolved the SPBEX plan’s legacy storage value: it called for using existing history TTL and byte limits [Source: specs/002-spbex-ticker-update/plan.md -> "Storage"]; current durable storage remains indefinite, date-keyed SQLite history without TTL expiration or automatic durable-row eviction, and `EXCHANGE_API_HISTORY_CACHE_MAX_BYTES` does not cap durable history [Source: .specify/memory/plan.md -> "Storage"], consistent with the project’s accepted TTL deprecation.
> **Revision**: 2026-10-09 — Added CBR's XML adapter, shared provider integration, route and configuration details.
> **Revision**: 2026-10-09 — Resolved CBR's conflicting storage scalar: its plan specified a 60-second history TTL and expiry [Source: specs/003-cbr-currency-rates/plan.md -> "Storage"]; the current service retains date-keyed SQLite history indefinitely without TTL expiry [Source: specs/004-history-cache-refresh/plan.md -> "Storage"].
> **Revision**: 2026-10-10 — Added provider-qualified v2 history and quote routes to the existing router and OpenAPI contract while retaining all v1 routes.
> **Revision**: 2026-10-10 — Archived v2 route availability with unset refresh settings, its all-six-route subprocess coverage, and the separate MOEX cold-fetch performance reporting rule.
> **Revision**: 2026-10-10 — Added process-wide log-color configuration, formatter behavior, and subprocess output coverage.

## Summary

The service exposes provider-qualified v1 and v2 history and quote routes for MOEX, SPBEX, and CBR. V2 paths select exactly one provider using a `moex`, `spbex`, or `cbr` segment; existing v1 routes remain available. MOEX history is sourced from public ISS with per-date primary-board selection; MOEX quotes fetch the latest trade per request. SPBEX history uses its public daily chart feed and SPBEX quotes fetch the latest available candle on every request. CBR history and quotes use the Bank of Russia's public XML interface, normalize rates to RUB per currency unit, and fetch the latest official rate for each quote request. The service persists normalized history indefinitely in SQLite, fetches only records newer than the latest stored date during a user request, and runs a background full refresh for each stored provider/symbol collection on its configured interval. Atomic record merges preserve committed history and apply source revisions. Existing six-field response contracts and provider boundaries remain in place. Log coloring is configured process-wide at startup and applied through the existing tracing formatter; it is enabled by default and can be disabled with recognized `EXCHANGE_API_LOG_COLOR` values. [Source: specs/004-history-cache-refresh/plan.md -> "Summary"] [Source: specs/001-moex-ticker-update/plan.md -> "Summary"] [Source: specs/002-spbex-ticker-update/plan.md -> "Summary"] [Source: specs/003-cbr-currency-rates/plan.md -> "Summary"] [Source: specs/005-v2-history-quote-api/plan.md -> "Summary"] [Source: specs/006-log-coloring/plan.md -> "Summary"]

## Technical Context

**Language/Version**: Rust 2024 edition, minimum Rust 1.85; repository toolchain 1.89. [Source: specs/004-history-cache-refresh/plan.md -> "Language/Version"] [Source: specs/002-spbex-ticker-update/plan.md -> "Language/Version"] [Source: specs/006-log-coloring/plan.md -> "Language/Version"]

**Primary Dependencies**: Axum 0.8, Tokio 1.53, Reqwest 0.13 with Rustls, SQLx 0.8 with SQLite, Serde/serde_json, Moka, chrono/chrono-tz, tracing, Serde-enabled quick-xml, and async-trait; `tracing-subscriber` 0.3.23 provides the existing `fmt` formatter with ANSI control. Wiremock is used for fixture-backed tests. The CBR plan does not state versions for quick-xml or async-trait. The SPBEX Reqwest client adds the official Russian Trusted Root CA to that client's verifier while retaining certificate and hostname verification; keep that trust anchor updated from its official source. [Source: specs/004-history-cache-refresh/plan.md -> "Primary Dependencies"] [Source: specs/001-moex-ticker-update/plan.md -> "Primary Dependencies"] [Source: specs/002-spbex-ticker-update/plan.md -> "Primary Dependencies"] [Source: specs/002-spbex-ticker-update/plan.md -> "Constraints"] [Source: specs/002-spbex-ticker-update/quickstart.md -> "Keep this root updated from the official certificate source"] [Source: specs/003-cbr-currency-rates/plan.md -> "Primary Dependencies"] [Source: specs/006-log-coloring/plan.md -> "Primary Dependencies"]

**Storage**: SQLite history store at `EXCHANGE_API_HISTORY_CACHE_DB_PATH`, with provider/symbol collections, date-keyed records, successful-refresh timestamps, and persisted retry state. Migration preserves legacy `MOEX:{symbol}` and `CBR:{symbol}` responses and consolidates `SPBEX:{symbol}:{UTC-date}` rows into one collection per normalized symbol; collection identities use normalized exchange-qualified symbols such as `SPBEX:SBER` and `CBR:USD`. Durable rows are not expired or automatically evicted. Use WAL on a durable instance-local volume; keep database and sidecars together and do not share a SQLite/WAL database across hosts. Quotes bypass history storage. The log-color setting adds no persisted state. The CBR plan's older shared history TTL/capacity setting is superseded by the indefinite durable-history model. [Source: specs/004-history-cache-refresh/plan.md -> "Storage"] [Source: specs/004-history-cache-refresh/data-model.md -> "Collection identity"] [Source: specs/001-moex-ticker-update/plan.md -> "Storage"] [Source: specs/002-spbex-ticker-update/plan.md -> "Storage"] [Source: specs/003-cbr-currency-rates/plan.md -> "Storage"] [Source: specs/006-log-coloring/data-model.md -> "Log Color Setting"]

**Testing**: Rust unit, provider-client, API contract, scheduler, SQLite migration, and integration checks run with `cargo test`; fixture-backed clients use Wiremock. SPBEX adds fixture tests for chart parsing/mapping, route errors and quote lookback, OpenAPI schema assertions, and restart persistence. CBR adds XML client and mapping fixtures, supported-currency validation, provider contract and route errors, uncached latest-rate quotes, durable history, and canonical OpenAPI schema checks. Feature quickstarts record completed validation results. Configuration parsing for log coloring is tested with `src/config.rs`, while `tests/log_color.rs` exercises actual formatter output in subprocesses. [Source: specs/006-log-coloring/plan.md -> "Testing"] [Source: specs/004-history-cache-refresh/plan.md -> "Testing"] [Source: specs/004-history-cache-refresh/quickstart.md -> "Validate the history lifecycle"] [Source: specs/002-spbex-ticker-update/plan.md -> "Testing"] [Source: specs/003-cbr-currency-rates/plan.md -> "Testing"] [Source: specs/003-cbr-currency-rates/research.md -> "Validation and delivery"]

**Target Platform**: Linux server/container; amd64 image build is required, arm64 remains supported, and graceful SIGINT shutdown is bounded to 30 seconds. [Source: specs/006-log-coloring/plan.md -> "Target Platform"] [Source: specs/004-history-cache-refresh/plan.md -> "Target Platform"] [Source: specs/001-moex-ticker-update/plan.md -> "Target Platform"] [Source: specs/002-spbex-ticker-update/plan.md -> "Target Platform"] [Source: specs/003-cbr-currency-rates/plan.md -> "Target Platform"]

**Project Type**: Existing single Rust REST microservice with an in-process background refresh worker and provider adapters for MOEX, SPBEX, and CBR. [Source: specs/006-log-coloring/plan.md -> "Project Type"] [Source: specs/004-history-cache-refresh/plan.md -> "Project Type"] [Source: specs/002-spbex-ticker-update/plan.md -> "Project Type"] [Source: specs/003-cbr-currency-rates/plan.md -> "Project Type"]

**Performance Goals**: For the existing six v1 routes, under 10 concurrent clients issuing 10 requests per second total, at least 95% of successful responses on each route complete in under one second; MOEX measurements include history and uncached quote requests, while SPBEX and CBR history-only, quote-only, and combined profiles must sustain the full rate and meet the gate. For the six v2 provider history and quote routes, use a separate profile per route at the same load and threshold; report the initial uncached MOEX history fetch separately and apply its gate only after the full history is populated in cache. Recorded provider workload p95 values are in feature quickstarts. Log coloring has no measurable request-path effect because its setting is resolved at startup and only changes formatting. [Source: specs/006-log-coloring/plan.md -> "Performance Goals"] [Source: specs/004-history-cache-refresh/plan.md -> "Performance Goals"] [Source: specs/001-moex-ticker-update/plan.md -> "Performance Goals"] [Source: specs/002-spbex-ticker-update/plan.md -> "Performance Goals"] [Source: specs/003-cbr-currency-rates/plan.md -> "Performance Goals"] [Source: specs/005-v2-history-quote-api/plan.md -> "Performance Goals"]

**Constraints**: Retain history indefinitely; remove `EXCHANGE_API_HISTORY_CACHE_TTL_SECS`; configure full-refresh cadence with `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS` (default 604800 seconds); configure the positive retry cap with `EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS` (default 900 seconds); validate full results before atomic commit; preserve upstream request and response-size limits. MOEX uses public ISS without subscriber credentials, named response columns, complete cursor pagination with bounded fan-out, and per-date primary-board selection. SPBEX uses the public daily chart feed at resolution `1440`; history excludes candles dated today in UTC, and quotes expand a fresh one-day lookback exponentially until finding a candle or reaching the Unix epoch. Bound SPBEX concurrent upstream requests to sixteen, validate response size/JSON/timestamps/finite positive OHLC consistency/duplicate dates, sort history ascending, and configure source URL and response-size limit independently. CBR validates ISO symbols against the current CBR currency directory (cached for 60 seconds), reads the source-available history range, maps `Value / Nominal` to RUB per unit, uses UTC effective dates, and bounds XML responses with provider-specific configuration; quotes fetch the latest official rate each time. Log coloring is a startup-only setting: `EXCHANGE_API_LOG_COLOR` defaults to enabled, and case-insensitive `false`, `0`, `no`, or `off` disable ANSI coloring while all other values enable it. The existing formatter must preserve log content and severity; no logging dependency or persistent state is added. [Source: specs/006-log-coloring/plan.md -> "Constraints"] [Source: specs/004-history-cache-refresh/plan.md -> "Constraints"] [Source: specs/001-moex-ticker-update/plan.md -> "Constraints"] [Source: specs/001-moex-ticker-update/research.md -> "MOEX ISS history and latest trades"] [Source: specs/002-spbex-ticker-update/plan.md -> "Constraints"] [Source: specs/003-cbr-currency-rates/plan.md -> "Constraints"] [Source: specs/003-cbr-currency-rates/research.md -> "Official source and rate mapping"]

**Scale/Scope**: Three provider namespaces and arbitrary validated ticker/currency symbols; each request addresses one normalized symbol. Refresh every stored collection, including symbols not requested since the last sweep, with bounded concurrency. Log coloring is limited to one process-wide setting and the existing formatter; it adds no routes or service boundaries. [Source: specs/006-log-coloring/plan.md -> "Scale/Scope"] [Source: specs/004-history-cache-refresh/plan.md -> "Scale/Scope"] [Source: specs/002-spbex-ticker-update/plan.md -> "Scale/Scope"] [Source: specs/003-cbr-currency-rates/plan.md -> "Scale/Scope"]

## Project Structure

### Source Code

```text
src/
├── cache.rs
├── cache_store.rs
├── config.rs
├── history_refresh.rs
├── provider.rs
├── http/routes.rs
├── main.rs
├── shutdown.rs
├── moex/client.rs
├── moex/board.rs
├── moex/mapping.rs
├── moex/models.rs
├── moex/validation.rs
├── spbex/client.rs
├── spbex/models.rs
├── spbex/mapping.rs
├── spbex/validation.rs
├── cbr/mod.rs
├── cbr/client.rs
├── cbr/models.rs
├── cbr/mapping.rs
├── cbr/validation.rs
└── cbr/provider.rs

tests/
├── cache_store.rs
├── api_contract.rs
├── api_spbex.rs
├── api_contract_schema.rs
├── history_refresh.rs
├── api_errors.rs
├── api_quote.rs
├── moex_client.rs
├── moex_mapping.rs
├── spbex_client.rs
├── spbex_mapping.rs
├── cbr_client.rs
├── cbr_mapping.rs
├── api_cbr.rs
├── provider_contract.rs
├── log_color.rs
└── provider client and API fixtures
```

The existing Rust service owns provider clients, routes, persistence, and worker lifecycle; no separate service or runtime store is added. SPBEX and CBR remain separate adapters under their respective directories, with exchange-qualified normalized history identities and no independent persistence service. CBR parses its public XML directory, history, and latest-rate responses within its adapter. Log coloring extends existing `src/config.rs` and `src/main.rs`; no production module or service is added. [Source: specs/006-log-coloring/plan.md -> "Structure Decision"] [Source: specs/004-history-cache-refresh/plan.md -> "Structure Decision"] [Source: specs/002-spbex-ticker-update/plan.md -> "Keep one deployable Rust service"] [Source: specs/003-cbr-currency-rates/plan.md -> "Structure Decision"]

## Routing and Configuration

V1 history routes are `GET /v1/moex/{SYMBOL}`, `GET /v1/spbex/{SYMBOL}`, and `GET /v1/cbr/{SYMBOL}`; their quote routes use the same provider paths with `/quote`. V2 exposes `GET /v2/history/{PROVIDER}/{SYMBOL}` and `GET /v2/quote/{PROVIDER}/{SYMBOL}`, where `PROVIDER` is exactly `moex`, `spbex`, or `cbr`; unknown providers return HTTP 400 with `invalid_provider`. Both versions dispatch to existing provider behavior and preserve the six-field response contract. V2 routes are registered independently of `EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS` and `EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS`; when unset these settings default to 604800 and 900 seconds. MOEX history uses ISS pagination and board mapping; its quote fetches the latest trade every request and bypasses history storage. SPBEX history uses the chart feed at daily resolution, excludes current UTC-date candles, and sorts records ascending. Its quote adaptively expands the fresh chart-feed lookback and bypasses history storage. CBR history fetches source-available dated XML history; its quote fetches the latest official rate on every request and bypasses history storage. Each history request fetches after the latest retained date. The worker performs full collection refreshes in the background. [Source: specs/004-history-cache-refresh/plan.md -> "Summary"] [Source: specs/001-moex-ticker-update/plan.md -> "Summary"] [Source: specs/001-moex-ticker-update/contracts/openapi.yaml -> paths] [Source: specs/002-spbex-ticker-update/plan.md -> "Summary"] [Source: specs/002-spbex-ticker-update/contracts/openapi.yaml -> paths] [Source: specs/003-cbr-currency-rates/plan.md -> "Summary"] [Source: specs/003-cbr-currency-rates/contracts/openapi.yaml -> paths] [Source: specs/005-v2-history-quote-api/plan.md -> "Summary"] [Source: specs/005-v2-history-quote-api/plan.md -> "Constraints"]

CBR support validation intersects `XML_valFull.asp` currency identifiers with codes in the current `XML_daily.asp?d=0` publication; history uses `XML_dynamic.asp` for the full source-available range, and each quote makes a fresh request to `XML_daily.asp`. The full directory is used because the shorter directory endpoint omits ISO character codes; intersecting with the current daily publication excludes legacy currencies without current rates. [Source: specs/003-cbr-currency-rates/research.md -> "Official source and rate mapping"]

The SPBEX client reads `EXCHANGE_API_SPBEX_API_BASE_URL` (default `https://spbexchange.ru/api/`) and `EXCHANGE_API_SPBEX_MAX_RESPONSE_BYTES` (default `16777216`), and reuses the shared request timeout. [Source: specs/002-spbex-ticker-update/quickstart.md -> "EXCHANGE_API_SPBEX_API_BASE_URL"] [Source: specs/002-spbex-ticker-update/quickstart.md -> "EXCHANGE_API_SPBEX_MAX_RESPONSE_BYTES"] [Source: specs/002-spbex-ticker-update/plan.md -> "Constraints"]

`EXCHANGE_API_LOG_COLOR` controls ANSI output at process startup: it is enabled when absent; `false`, `0`, `no`, and `off` disable it case-insensitively; every other value enables it. The setting does not change filtering or message formatting beyond ANSI sequences. [Source: specs/006-log-coloring/contracts/log-color-config.md -> "EXCHANGE_API_LOG_COLOR"]

The CBR client reads `EXCHANGE_API_CBR_API_BASE_URL` (default `https://www.cbr.ru/`) and `EXCHANGE_API_CBR_MAX_RESPONSE_BYTES` (default `16777216`). Its current supported-currency directory is cached for 60 seconds; this metadata cache does not cache rate history or quote responses. [Source: specs/003-cbr-currency-rates/quickstart.md -> "EXCHANGE_API_CBR_API_BASE_URL"] [Source: specs/003-cbr-currency-rates/quickstart.md -> "EXCHANGE_API_CBR_MAX_RESPONSE_BYTES"] [Source: specs/003-cbr-currency-rates/research.md -> "Directory freshness"]

`EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS` is a positive integer in seconds, default `604800`. `EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS` is a positive integer in seconds, default `900`. Setting the removed `EXCHANGE_API_HISTORY_CACHE_TTL_SECS` causes startup failure with a migration message. Docker Compose configures both new settings. [Source: specs/004-history-cache-refresh/plan.md -> "Constraints"] [Source: specs/004-history-cache-refresh/quickstart.md -> "Configure and start"]

## Refresh and Failure Handling

The worker enumerates stored collections and runs at most four refreshes concurrently. Retryable network, timeout, selected status, and invalid/incomplete-data failures use persisted exponential backoff starting at one second and capped by configuration; other HTTP 4xx failures wait one configured interval. Full results are validated before a transaction commits records and success metadata. Failures preserve last-good history. MOEX history pagination validates consistent named columns, all pages, and response bounds before mapping; page fan-out is bounded to 128 per symbol. SPBEX and CBR full refreshes use the same all-or-nothing collection validation and merge; their clients validate each bounded response before mapping. [Source: specs/004-history-cache-refresh/research.md -> "Decision 3: Run a durable, bounded-concurrency background full refresh"] [Source: specs/004-history-cache-refresh/research.md -> "Decision 8: Retry background full-refresh failures with a configurable cap"] [Source: specs/004-history-cache-refresh/research.md -> "Decision 9: Validate a full response before committing any part of it"] [Source: specs/001-moex-ticker-update/research.md -> "MOEX ISS history and latest trades"] [Source: specs/002-spbex-ticker-update/plan.md -> "Constraints"] [Source: specs/003-cbr-currency-rates/plan.md -> "Constraints"]

## Testing Strategy

SPBEX fixtures cover chart parsing and mapping, route errors, quote lookback, OpenAPI schema validation, restart persistence, and full-rate history-only, quote-only, and combined latency profiles. CBR fixtures cover XML parsing/mapping, supported currencies, empty and malformed data, route errors, and fresh quote behavior. Recorded SPBEX and CBR profiles on 2026-10-08/09 met the under-one-second p95 gate at the required 10 requests per second. [Source: specs/002-spbex-ticker-update/plan.md -> "Testing"] [Source: specs/002-spbex-ticker-update/research.md -> "Quote freshness and latency"] [Source: specs/003-cbr-currency-rates/plan.md -> "Testing"] [Source: specs/003-cbr-currency-rates/research.md -> "Validation and delivery"]

Important API, provider, configuration, migration, scheduler, provider-specific parsing/mapping/error behavior, and shutdown behavior is covered by automated Rust tests using fixtures and mocked upstream responses. Log-color parsing is covered by configuration unit tests, and actual disabled/default/enabled formatter output is checked through subprocess capture in `tests/log_color.rs`. [Source: specs/006-log-coloring/plan.md -> "Testing"] The v2 subprocess test starts the real service with both refresh settings unset and checks all six provider history and quote routes. Performance acceptance profiles each v2 route at 10 total requests per second with 10 concurrent clients and requires at least 95% of successful responses per route under one second; the initial uncached MOEX history fetch is measured separately, with the gated profile run after full history is populated. Quote calls remain uncached and history tests include cache and persistence behavior. Release checks include a Linux amd64 image build, Semgrep source scan, and Trivy image scan. Feature quickstarts record the 2026-10-09 and 2026-10-10 performance and release-check results. [Source: specs/004-history-cache-refresh/plan.md -> "Testing"] [Source: specs/004-history-cache-refresh/quickstart.md -> "Measured acceptance (2026-10-09)"] [Source: specs/004-history-cache-refresh/quickstart.md -> "Release checks (2026-10-09)"] [Source: specs/001-moex-ticker-update/plan.md -> "Testing"] [Source: specs/001-moex-ticker-update/plan.md -> "Project Structure"] [Source: specs/003-cbr-currency-rates/plan.md -> "Testing"] [Source: specs/005-v2-history-quote-api/plan.md -> "Testing"] [Source: specs/005-v2-history-quote-api/plan.md -> "Performance Goals"]

## Complexity Tracking

The SQLite schema migration and bounded scheduler extend the existing service boundary and require no separate service or runtime store. [Source: specs/004-history-cache-refresh/plan.md -> "Complexity Tracking"]
