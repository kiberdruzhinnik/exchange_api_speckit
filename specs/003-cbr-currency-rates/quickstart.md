# CBR Feature Quickstart

Use this guide to validate the Bank of Russia history and quote routes after implementation. See the [OpenAPI contract](contracts/openapi.yaml) for request and response schemas and the [data model](data-model.md) for field mapping.

## Prerequisites and configuration

- Rust toolchain from `rust-toolchain.toml`, Cargo, and the committed lockfile.
- Network access to `https://www.cbr.ru/` for live checks. CBR may rate-limit quote traffic; upstream HTTP failures map to the documented HTTP 502 response.
- A supported CBR currency symbol such as `USD`, `CNY`, or `EUR`.

| Variable | Default | Purpose |
|---|---|---|
| `CBR_API_BASE_URL` | `https://www.cbr.ru/` | Bank of Russia XML service base URL; override for fixture/mock tests. |
| `CBR_MAX_RESPONSE_BYTES` | `16777216` | Maximum bytes accepted from one CBR XML response. |
| Currency directory freshness | 60 seconds | In-process supported-currency list refresh interval; quote rate values are never cached. |
| `MOEX_REQUEST_TIMEOUT_SECS` | `15` | Existing shared upstream timeout. |
| `MOEX_HISTORY_CACHE_TTL_SECS` | `60` | Existing history freshness period. |
| `MOEX_HISTORY_CACHE_MAX_BYTES` | `67108864` | Existing aggregate history-cache payload limit. |
| `MOEX_HISTORY_CACHE_DB_PATH` | `/var/lib/exchange-api/history.sqlite3` | Existing durable history-store path. |

## Local checks

Run fixture-backed parsing, mapping, client, route, cache, and contract checks:

```sh
cargo fmt --check
cargo test
```

Start the service with a persistent cache path:

```sh
MOEX_HISTORY_CACHE_DB_PATH=./exchange-api-data/history.sqlite3 cargo run
```

In another terminal, request history and a fresh quote:

```sh
curl -i http://127.0.0.1:8080/v1/cbr/USD
curl -i http://127.0.0.1:8080/v1/cbr/USD/quote
```

History returns all available rates in ascending date order. Each `close` is RUB per one currency unit: a CBR source value of 5 RUB for a nominal of 100 units becomes `0.05`; if `Value` or `Nominal` is missing, `close` is null. `facevalue` is the source nominal when available. CBR does not provide high, low, or volume, so these fields are null. Quote returns one latest-rate record and fetches rate data on every request. The supported-currency directory refreshes every 60 seconds. It intersects `XML_valFull.asp` identifiers with the codes in `XML_daily.asp?d=0`; rate data is not part of that cache. Invalid or unsupported symbols return HTTP 400, source failures (including CBR rate limits) return HTTP 502, and history-store failures return HTTP 503. Restart the service with the same database path before history TTL expiry to confirm persisted history remains available.

## Production latency acceptance

Build and run the local release binary against the production Bank of Russia XML service. The acceptance harness consumes full bodies, schedules 10 requests per second on a fixed cadence, uses 10 concurrent clients, and reports actual issue rate, skipped requests, failures, and successful-response p95 separately for history-only, quote-only, and combined traffic. It separately probes history cold fetch, warm memory hit, and expiry refresh, then warms history before timed profiles so a slow cache fill does not invalidate the fixed-arrival run. Each quote reaches the source. Timed profiles must be shorter than the configured history TTL. A run below the requested rate or with skipped arrivals is invalid. Every valid profile must meet successful-response p95 under one second; otherwise optimize the request/source path and repeat until it passes.

```sh
cargo build --release --locked
scripts/measure-cbr-latency.sh
```

Recorded production run: 2026-10-09, symbol `USD`, production CBR source, local optimized binary. The lifecycle probe returned 200 for cold, warm, and expiry requests and observed one cold source fetch, one warm memory hit, and one expiry-triggered source fetch at the 60-second TTL. Each 55-second timed profile issued all 550 scheduled arrivals (10 concurrent clients, 10 total requests/second), with zero skips and errors: history-only p95 0.002810 s; quote-only p95 0.059297 s; combined overall p95 0.047897 s, history p95 0.002958 s, and quote p95 0.145707 s. An earlier exploratory run saw intermittent CBR HTTP 429 responses, which the API reports as HTTP 502; the final instrumented run had no dependency errors. The harness rejects skipped or under-rate timed profiles and any route p95 at or above one second.

## Container and final scans

Build the required amd64 image; build success is the amd64 acceptance check. Preserve existing arm64 build/runtime support.

```sh
docker buildx build --platform linux/amd64 --load -t exchange-api:cbr-amd64 .
```

On the final build, run Semgrep on source and Trivy against the built deliverable. Resolve all Semgrep findings and all fixable High/Critical Trivy findings before declaring the feature complete, and document any unavailable fix.

Recorded final checks on 2026-10-09: `docker buildx build --platform linux/amd64 --load -t exchange-api:cbr-amd64 .` succeeded. Semgrep (`semgrep scan --config auto --error --no-git-ignore /src/src /src/tests /src/scripts /src/Dockerfile`) scanned the implementation and test paths (36 files) and found 0 findings. Trivy scanned the saved amd64 image archive for High and Critical vulnerabilities with unavailable fixes excluded; it found 0 findings. The archive was used because the Trivy container cannot access the host Docker daemon.
