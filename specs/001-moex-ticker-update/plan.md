# Implementation Plan: MOEX Ticker History API

**Branch**: `001-moex-ticker-update` | **Date**: 2026-10-08 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/001-moex-ticker-update/spec.md`

## Summary

Build a Rust HTTP service exposing `GET /v1/moex/{SYMBOL}` for daily history and `GET /v1/moex/{SYMBOL}/quote` for the latest executed trade. Fetch history from unauthenticated MOEX ISS, select each record from the board that was primary on its trading date, and map named source fields into the specified JSON array. Populate `facevalue` from current board-specific `LOTSIZE`; the SBER record dated 2026-10-06 is expected to be `1`. Cache only complete history responses in a bounded in-memory hot cache backed by per-instance SQLite so fresh entries survive application restarts. Fetch quote data from ISS for every quote request and never cache it. Return quotes as a one-element history-shaped array: trade execution time, price, and size map to `date`, `close`, and `volume`; `high`, `low`, and `facevalue` are null. A valid no-trade result is one record with all six values null. On SIGINT (Ctrl+C), stop accepting new connections, drain in-flight work, and exit within 30 seconds, cancelling remaining work at the deadline. Package the service as a non-root distroless Debian image for Linux amd64 and arm64.

## Technical Context

**Language/Version**: Rust stable, edition 2024; package MSRV is Rust 1.85 as declared in `Cargo.toml`.

**Primary Dependencies**: Axum and Tokio for HTTP/async runtime; `reqwest` with Rustls for upstream HTTPS; Serde/serde_json for data mapping; SQLx SQLite for async durable cache access; Moka for bounded in-memory history hot-cache/singleflight; `chrono-tz` for exchange-local trade time conversion; `tracing` for structured logs.

**Storage**: SQLite database at `EXCHANGE_API_HISTORY_CACHE_DB_PATH` (container default `/var/lib/exchange-api/history.sqlite3`) on a durable volume attached to each instance. Store one complete serialized history response per symbol with `fetched_at`, `expires_at`, and response byte count. Use a bounded SQLite backing store (64 MiB default logical response-payload capacity) and a bounded 64 MiB Moka hot cache, both with a 60-second default TTL. A hit is valid only when its expiry is in the future. Persist only after every ISS page and mapping step succeeds; quotes bypass both caches. SQLite read/write failures on the history path return a documented `503` service error. Use SQLite WAL on the instance-local mounted filesystem, not a volume concurrently shared across hosts.

**Testing**: Cargo unit and integration tests, mocked MOEX ISS fixtures for pagination, parsing, nulls, and failures, OpenAPI contract validation, and Docker validation. Verify SIGINT stops acceptance of new requests, allows in-flight requests to finish, and cancels work still active at the 30-second deadline. Build the Linux amd64 image successfully; amd64 runtime startup and route checks are outside acceptance. Smoke-start the arm64 image on a compatible runtime.

**Target Platform**: Linux containers for `linux/amd64` (required) and `linux/arm64` (native developer platform), port 8080. Build with Docker Buildx. Compile on `BUILDPLATFORM` using the Rust target triple and matching Debian cross-compiler, then use the target-platform `gcr.io/distroless/cc-debian13:nonroot` runtime, which includes the GCC runtime dependency on both architectures.

**Project Type**: Single Rust web service in the existing Cargo application.

**Performance Goals**: Normal operating conditions are 10 concurrent clients issuing 10 requests per second in total. At least 95% of successful requests to both public endpoints must complete end to end in under one second. Measure each route under that profile and run a combined profile so one fast route cannot hide a slow route. History requests use the durable cache and a bounded in-memory hot cache, with same-symbol misses coalesced; quote requests make an uncached ISS trades request each time. Include cache hits, cold misses, expirations, and quote calls in acceptance measurements. Upstream quote latency remains an external dependency and must be reported even if it prevents the target from passing.

**Constraints**: Fetch only history/trade data MOEX ISS makes available without subscription credentials; never send subscriber credentials. Full history may require multiple pages; retrieve the first cursor page, then fetch remaining pages concurrently with a bounded per-symbol fan-out of 128 requests. Use an upstream request timeout and bounded response handling; failures return the documented dependency error and never partial history or a fabricated no-trade result. Select the primary board by trading date for history. Populate history `facevalue` from current board-specific `LOTSIZE`, not `FACEVALUE` or a historical LOTSIZE series; return null when unavailable. The quote endpoint requests one latest trade on every call using the ISS security trades resource (`limit=1`, reverse order), maps `PRICE`, `QUANTITY`, `TRADEDATE`, and `TRADETIME`, and converts the exchange-local trade timestamp to UTC. A successful empty trade block for a recognized ticker returns a one-element six-field array with all values null; malformed or failed upstream responses return the documented dependency error. On SIGINT, reject new connections, gracefully drain existing work for no more than 30 seconds, and cancel outstanding work when the deadline expires. Build and ship for Linux amd64; retain arm64 support.

**Scale/Scope**: One ticker per request. History includes all daily records MOEX ISS makes available without credentials, across the primary board applicable to each date. Public endpoints have no query parameters or pagination. Durable history cache state belongs to each service instance and is not shared across instances; unexpired cache entries survive application restarts. Bound both stored response payloads and in-memory hot-cache bytes. Do not serve expired history or quote data from cache and do not serve stale history after an upstream error.

## Constitution Check

- **REST API Contracts First**: Pass after this design update. Both history and quote routes, response schemas, status codes, and error representation are defined in the OpenAPI contract.
- **Documentation Is Part of Delivery**: Pass after this design update. OpenAPI and quickstart cover both routes, runtime configuration, dependencies, and operational validation.
- **Clear Microservice Boundaries**: Pass. This is one independently deployable MOEX history service with a named upstream dependency.
- **Compatibility and Change Management**: Pass. The route is versioned under `/v1`; changes must preserve this response or define migration/versioning.
- **Practical Quality and Operability**: Pass with design requirements for actionable errors, structured logs including request duration and cache/storage outcome, health/readiness endpoints, fixture-backed checks, restart-persistence validation, bounded graceful SIGINT shutdown, the 10-client/10-request-per-second latency profile, successful Linux amd64 image build, arm64 runtime smoke validation, and final Semgrep/Trivy gates.
- **Private-network security posture**: Pass. No application authentication is introduced by this feature. The service makes unauthenticated MOEX ISS requests and does not use subscriber credentials.

**Post-Design Gate**: PASS. The Phase 1 design documents both versioned API routes and error outcomes, preserves the existing history response contract, keeps the capability within one service, documents its runtime configuration, durable history behavior, and bounded SIGINT shutdown, and includes the required automated and final security scan gates.

## Project Structure

### Documentation (this feature)

```text
specs/001-moex-ticker-update/
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
Cargo.toml
Cargo.lock
Dockerfile
.dockerignore
src/
├── main.rs                  # configuration, HTTP listener, app state, SIGINT shutdown coordination
├── cache.rs                 # bounded TTL cache for complete ticker responses
├── cache_store.rs           # SQLite schema, expiry lookup, atomic upsert, pruning
├── config.rs                # listener, upstream timeout, DB path, cache bounds/TTL
├── http/
│   ├── mod.rs
│   ├── routes.rs            # history and quote routes, health/readiness
│   └── errors.rs            # documented JSON error mapping
├── moex/
│   ├── mod.rs
│   ├── client.rs            # unauthenticated ISS transport, timeout, pagination, history/LOTSIZE/trades
│   ├── board.rs             # primary-board assignment by trading date
│   ├── mapping.rs           # named source-column mapping and nulls
│   ├── models.rs            # ISS history and trade response types
│   └── validation.rs        # symbol validation
└── domain.rs                # ticker, daily-record, and latest-trade response models
tests/
├── api_contract.rs
├── api_errors.rs
├── api_quote.rs
├── moex_client.rs
├── moex_mapping.rs
└── fixtures/moex/
    ├── history-page.json
    ├── history-empty.json
    ├── history-multi-page.json
    ├── board-listing.json
    └── security-description.json
```

**Structure Decision**: Keep one Rust web-service application. Runtime routes, ISS integration, per-date board/reference selection, persistent cache, response mapping, and process shutdown coordination are separated into small modules while remaining in one deployable microservice. The Axum listener uses graceful shutdown to stop accepting connections; active request/server tasks are tracked or otherwise controlled so they can drain and be cancelled at a global 30-second deadline. SQLite database files and WAL sidecars must remain together on an instance-local durable filesystem. The Dockerfile must not hard-code an architecture-specific runtime library path; Buildx cross-compiles the executable for each target and selects the matching distroless runtime. SQLx's SQLite feature statically bundles SQLite; the builder needs target-aware C tooling while the runtime image needs no SQLite shared library.

**Research Gate**: Pass. Clarified API semantics, current board-specific LOTSIZE mapping, SQLite persistence and WAL constraints, latest-trade ISS mapping, the 10-client/10-request-per-second profile, cross-platform Docker build/runtime choices, and cooperative bounded SIGINT shutdown are recorded in `research.md`. No unresolved technical questions remain in this plan.

## Complexity Tracking

No constitution violations or additional services are required. SQLite is embedded persistence owned by this service and remains isolated to each instance's attached durable volume.
