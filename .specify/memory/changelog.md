# Changelog

## Merged Features Log

### CBR Currency Rates API — archived 2026-10-09
**Branch:** 003-cbr-currency-rates
**Spec:** [specs/003-cbr-currency-rates/spec.md](../../specs/003-cbr-currency-rates/spec.md)

**What was added:**
- Bank of Russia currency history and fresh latest-official-rate quote behavior, including RUB-per-unit normalization and provider-specific mappings.
- CBR XML source validation, nullable field behavior, standard provider errors, shared provider contract, and canonical `EXCHANGE_API_*` configuration requirements.
- CBR-specific latency, Linux amd64 build, SIGINT shutdown, documentation, and test coverage guidance for the service.
- Kept the established indefinite date-keyed SQLite history model; the feature plan's old TTL-based storage wording is superseded.

**New Components:**
- CBR XML client, source models, mapping, validation, and provider adapter under `src/cbr/`.
- CBR source, mapping, route, provider-contract, and OpenAPI schema tests.

**Tasks Completed:** 63/63 tasks

### SPBEX Ticker History and Quote API — archived 2026-10-09
**Branch:** 002-spbex-ticker-update
**Spec:** [specs/002-spbex-ticker-update/spec.md](../../specs/002-spbex-ticker-update/spec.md)

**What was added:**
- Public SPBEX daily history and latest-candle quote requirements, including current UTC-date exclusion from history and fresh adaptive quote lookback.
- SPBEX symbol normalization, valid-empty versus explicit rejection behavior, OHLC validation, and nullable-volume mapping.
- SPBEX-specific TLS trust, source configuration, route, testing, and latency guidance in the consolidated plan.
- Kept current indefinite date-keyed SQLite storage; did not carry forward the feature plan’s legacy history TTL and durable byte-limit wording.

**New Components:**
- SPBEX chart-feed client, source models, mapping, and symbol validation module guidance under `src/spbex/`.
- Fixture-backed SPBEX client, mapping, API, and schema tests.

**Tasks Completed:** 34/34 tasks

### MOEX Ticker History API — archived 2026-10-09
**Branch:** 001-moex-ticker-update
**Spec:** [specs/001-moex-ticker-update/spec.md](../../specs/001-moex-ticker-update/spec.md)

**What was added:**
- MOEX ISS history and latest-trade quote routes, including primary-board-per-date selection, UTC date handling, and the `LOTSIZE` to `facevalue` mapping.
- MOEX symbol/upstream validation, empty-history and no-trade outcomes, error documentation, and bounded SIGINT shutdown requirements.
- Shared API, entity, edge-case, acceptance, planning, and test guidance for the MOEX provider.
- Resolved the legacy plan’s 60-second TTL and 64 MiB durable response-cache design in favor of the current indefinite date-keyed SQLite history model.

**New Components:**
- MOEX provider module guidance for board selection, mapping, models, validation, and ISS client behavior.
- MOEX fixture-backed client, mapping, quote, API error, and contract checks in the consolidated plan.

**Tasks Completed:** 47/47 tasks

### Permanent History Cache Refresh — archived 2026-10-09
**Branch:** 004-history-cache-refresh
**Spec:** [specs/004-history-cache-refresh/spec.md](../../specs/004-history-cache-refresh/spec.md)

**What was added:**
- Indefinite durable history retention with incremental per-request fetching and complete-history responses.
- Configurable background full refreshes for every stored provider/symbol collection.
- Atomic validation and merge, persisted retries with a configurable backoff cap, and last-good-data preservation.
- Startup rejection and migration guidance for the removed history cache TTL setting.

**New Components:**
- `src/history_refresh.rs` background worker and scheduler.
- Provider/symbol collection and date-keyed history persistence in `src/cache_store.rs`.

**Tasks Completed:** 24/24 tasks
