# CBR Feature Quickstart

Use this guide to validate the Bank of Russia history and quote routes after implementation. See the [CBR OpenAPI view](contracts/openapi.yaml) and its [canonical shared contract](../contracts/openapi.yaml) for request and response schemas, and the [data model](data-model.md) for field mapping.

## Prerequisites and configuration

- Rust toolchain from `rust-toolchain.toml`, Cargo, and the committed lockfile.
- Network access to `https://www.cbr.ru/` for live checks. CBR may rate-limit quote traffic; upstream HTTP failures map to the documented HTTP 502 response.
- A supported CBR currency symbol such as `USD`, `CNY`, or `EUR`.

| Variable | Default | Purpose |
|---|---|---|
| `EXCHANGE_API_CBR_API_BASE_URL` | `https://www.cbr.ru/` | Bank of Russia XML service base URL; override for fixture/mock tests. |
| `EXCHANGE_API_CBR_MAX_RESPONSE_BYTES` | `16777216` | Maximum bytes accepted from one CBR XML response. |
| Currency directory freshness | 60 seconds | In-process supported-currency list refresh interval; quote rate values are never cached. |
| `EXCHANGE_API_LISTEN_ADDR` | `0.0.0.0:8080` | Shared service listen address. |
| `EXCHANGE_API_REQUEST_TIMEOUT_SECS` | `15` | Shared upstream request timeout. |
| `EXCHANGE_API_HISTORY_CACHE_TTL_SECS` | `60` | Shared history freshness period. |
| `EXCHANGE_API_HISTORY_CACHE_MAX_BYTES` | `67108864` | Shared aggregate history-cache payload limit. |
| `EXCHANGE_API_HISTORY_CACHE_DB_PATH` | `/var/lib/exchange-api/history.sqlite3` | Shared durable history-store path. |

## Local checks

Run fixture-backed parsing, mapping, client, route, cache, and contract checks:

```sh
cargo fmt --check
cargo test
```

Start the service with a persistent cache path:

```sh
EXCHANGE_API_HISTORY_CACHE_DB_PATH=./exchange-api-data/history.sqlite3 cargo run
```

In another terminal, request history and a fresh quote:

```sh
curl -i http://127.0.0.1:8080/v1/cbr/USD
curl -i http://127.0.0.1:8080/v1/cbr/USD/quote
```

History returns all available rates in ascending date order. Each `close` is RUB per one currency unit: a CBR source value of 5 RUB for a nominal of 100 units becomes `0.05`; if `Value` or `Nominal` is missing, `close` is null. `facevalue` is the source nominal when available. CBR does not provide high, low, or volume, so these fields are null. Quote returns one latest-rate record and fetches rate data on every request. The supported-currency directory refreshes every 60 seconds. It intersects `XML_valFull.asp` identifiers with the codes in `XML_daily.asp?d=0`; rate data is not part of that cache. Invalid or unsupported symbols return HTTP 400 `invalid_symbol`; source failures, including CBR rate limits, return HTTP 502 `cbr_unavailable`; history-store failures return HTTP 503 `history_store_unavailable`. These use the same envelope and status categories as MOEX and SPBEX while preserving established upstream codes. Restart the service with the same database path before history TTL expiry to confirm persisted history remains available.

## Production latency acceptance

Build and run the local release binary against the production Bank of Russia XML service. This CBR harness consumes full bodies, schedules 10 requests per second on a fixed cadence, uses 10 concurrent clients, and reports actual issue rate, skipped requests, failures, and successful-response p95 separately for history-only, quote-only, and combined traffic. It separately probes history cold fetch, warm memory hit, and expiry refresh, then warms history before timed profiles so a slow cache fill does not invalidate the fixed-arrival run. Each quote reaches the source. Timed profiles must be shorter than the configured history TTL. A run below the requested rate or with skipped arrivals is invalid. CBR profiles must meet successful-response p95 under one second. The complete feature gate covers all six MOEX, SPBEX, and CBR history and quote routes; rerun all three provider harnesses after the shared-route refactor and require each route to pass.

```sh
cargo build --release --locked
scripts/measure-cbr-latency.sh
```

Recorded CBR baseline: 2026-10-09, symbol `USD`, production CBR source, local optimized binary. The lifecycle probe returned 200 for cold, warm, and expiry requests and observed one cold source fetch, one warm memory hit, and one expiry-triggered source fetch at the 60-second TTL. Each 55-second timed profile issued all 550 scheduled arrivals (10 concurrent clients, 10 total requests/second), with zero skips and errors: history-only p95 0.002810 s; quote-only p95 0.059297 s; combined overall p95 0.047897 s, history p95 0.002958 s, and quote p95 0.145707 s. An earlier exploratory run saw intermittent CBR HTTP 429 responses, which the API reports as HTTP 502; the final instrumented run had no dependency errors. These results cover CBR before the shared-provider refactor only and do not replace final six-route acceptance.

## SIGINT shutdown

Run the service locally and press Ctrl+C in its terminal. It stops accepting new requests, allows in-flight work to finish, and exits within 30 seconds; remaining work is canceled at the deadline. The same process-level behavior is shared by MOEX, SPBEX, and CBR. The process-level shutdown checks verify Ctrl+C handling, stop-accepting behavior, completion of a controlled in-flight HTTP request, and cancellation after a test deadline. Run them with:

```sh
cargo test --test shutdown
```

Recorded shutdown acceptance on 2026-10-09: `cargo test --test shutdown` passed all 3 tests. The process-level test sent SIGINT, observed a successful exit within the 30-second limit, and confirmed the listener no longer accepted connections. A controlled in-flight request completed during drain. The cancellation test used a shortened grace interval to verify remaining work is canceled at the deadline; the service configures the production grace period to 30 seconds.

## Container and final scans

Build the required amd64 image; build success is the amd64 acceptance check, with no amd64 runtime startup or route checks required. Preserve existing arm64 build/runtime support.

```sh
docker buildx build --platform linux/amd64 --load -t exchange-api:cbr-amd64 .
```

On the final build, run Semgrep on source and Trivy against the built deliverable. Resolve all Semgrep findings and all fixable High/Critical Trivy findings before declaring the feature complete, and document any unavailable fix.

Recorded final checks on 2026-10-09: `docker buildx build --platform linux/amd64 --load -t exchange-api:cbr-amd64 .` succeeded. Semgrep (`semgrep scan --config auto --error --no-git-ignore /src/src /src/tests /src/scripts /src/Dockerfile`) scanned the implementation and test paths (36 files) and found 0 findings. Trivy scanned the saved amd64 image archive for High and Critical vulnerabilities with unavailable fixes excluded; it found 0 findings. The archive was used because the Trivy container cannot access the host Docker daemon.

## Final shared-provider latency acceptance

On 2026-10-09, the final shared-handler binary was measured against production CBR using USD and 10 concurrent clients at 10 total requests per second. The cache lifecycle probe verified one cold upstream fetch, a warm memory hit, and one fetch after the 60-second expiry. Each 55-second timed profile scheduled and issued all 550 requests with zero skips and zero upstream errors. History-only p95 was 0.003090 s, quote-only p95 was 0.042939 s, and combined overall p95 was 0.033285 s (history 0.005350 s, quote 0.039645 s).

Final cross-provider scans on 2026-10-09: the Linux amd64 distroless image built successfully. Semgrep `auto` scanned all 30 Rust source and test files plus the Python/Bash measurement scripts and Dockerfile with zero findings. Trivy scanned the built amd64 image archive (Debian 13.7, 14 OS packages) for High/Critical vulnerabilities with fixes available and found zero findings.

## EXCHANGE_API configuration namespace update

Validation on 2026-10-09 confirmed that all shared and provider-specific application settings use the `EXCHANGE_API_*` prefix, with prior provider-only names ignored. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (81 tests), and `cargo test --test shutdown` (3 tests) passed. All three measurement shell wrappers passed `bash -n`, their Python harnesses passed `py_compile`, and an audit found no provider-only environment lookups in runtime configuration or measurement wrappers. The Linux amd64 image built successfully with `docker build --platform linux/amd64 -t exchange-api:config-prefix .`. Semgrep `auto` scanned 42 files and reported 0 findings. Trivy scanned the built Debian 13.7 image (14 OS packages) for High and Critical vulnerabilities with fixes available and reported 0 findings.
