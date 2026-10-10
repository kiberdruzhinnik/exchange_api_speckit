# SPBEX Feature Quickstart

This guide validates SPBEX history and latest-candle behavior after
implementation. See [OpenAPI contract](contracts/openapi.yaml) for the complete
request and response schema and [data model](data-model.md) for field
validation.

## Prerequisites and configuration

- Rust toolchain from `rust-toolchain.toml`, Cargo, and the committed lockfile.
- Network access to `https://spbexchange.ru/api` for live checks.
- The SPBEX client includes the Russian Trusted Root CA used by SPBEX's TLS
    chain. Keep this root updated from the official certificate source as it
    approaches expiry.
- A syntactically valid SPBEX symbol with chart data for non-empty examples.

Optional environment variables:

| Variable | Default | Purpose |
| --- | --- | --- |
| `EXCHANGE_API_SPBEX_API_BASE_URL` | `https://spbexchange.ru/api/` | Public chart-feed base URL; override for fixture/mock tests. |
| `EXCHANGE_API_SPBEX_MAX_RESPONSE_BYTES` | `16777216` | Maximum bytes accepted from one chart-feed response. |
| `EXCHANGE_API_LISTEN_ADDR` | `0.0.0.0:8080` | Shared service listen address. |
| `EXCHANGE_API_REQUEST_TIMEOUT_SECS` | `15` | Shared upstream request timeout, including SPBEX. |
| `EXCHANGE_API_HISTORY_CACHE_TTL_SECS` | `60` | Existing history freshness period. |
| `EXCHANGE_API_HISTORY_CACHE_MAX_BYTES` | `67108864` | Existing aggregate history-cache payload limit. |
| `EXCHANGE_API_HISTORY_CACHE_DB_PATH` | `/var/lib/exchange-api/history.sqlite3` | Existing SQLite history-store path. |

## Local checks

Run fixture-backed mapping, client, route, cache, and contract validation:

```sh
cargo fmt --check
cargo test
```

Start the local service using the configured persistent history path:

```sh
EXCHANGE_API_HISTORY_CACHE_DB_PATH=./exchange-api-data/history.sqlite3 cargo run
```

In another terminal, request both routes (replace `SYMBOL` with a known SPBEX
listing):

```sh
curl -i http://127.0.0.1:8080/v1/spbex/SYMBOL
curl -i http://127.0.0.1:8080/v1/spbex/SYMBOL/quote
```

History returns an ascending array of six-field records dated before the current
UTC calendar date; candles dated today are excluded, even from cached or
persisted history payloads. `volume` is null and `facevalue` is `1`. Empty
successful history is `[]`. Quote returns one latest-candle record, may include
a candle dated today, and must contact the feed on every request. It starts with
a 1-day range and expands the range when empty until it finds a candle or
reaches the Unix epoch; only an empty result across the full available range
produces one record with nullable market fields. Invalid symbols or explicit
upstream rejection return HTTP 400, other upstream failures return HTTP 502, and
history-store failures return HTTP 503. Restart the service with the same DB
path and request history again before the cache TTL expires to confirm
persistence; after expiry, the source is fetched again.

## Production latency acceptance

Build and run the local release binary against production SPBEX. The acceptance
harness fully reads response bodies, schedules arrivals at 10 requests per
second on a fixed cadence, and caps in-flight requests at 10 workers. It reports
history-only, quote-only, and combined results separately. History runs must
include cache hits, cold misses, and expired entries. Quote runs always contact
the upstream feed. Record successful-response p95, actual issue rate, skipped
requests, and upstream failures separately. A run is invalid if it issues fewer
than 10 requests per second or skips scheduled requests; it cannot pass based on
p95 alone. Each valid route-specific run and the combined profile must achieve
successful-response p95 under one second. This is a hard acceptance gate.

```sh
cargo build --release --locked
scripts/measure-spbex-latency.sh
```

Production acceptance on 2026-10-08 with SBER, 10 workers, and a 120-second
full-rate run passed all profiles: history p95 0.003534 s, quote p95 0.458487 s,
and combined p95 0.502252 s (history route 0.004526 s, quote route 0.764562 s).
Each profile issued 1,200 requests at an observed 10.01 requests per second,
with no skips or errors. The SPBEX client permits up to eight concurrent
upstream requests.

The default benchmark symbol is `SBER`, verified to have live SPBEX chart data.
Pass `--symbols` only with symbols that have been confirmed on SPBEX; an empty
chart feed expands quote lookback to the full available range and measures a
different failure-free path.

The quote path depends on live chart-feed response time and size because it
fetches the latest candle without cache. If a valid run misses the target,
optimize the request/source path and repeat the measurement; the feature remains
incomplete until all valid profiles pass. Record each attempt, the source
latency/response size, the change made, and the result. Upstream errors are
reported separately and are not successful latency samples.

## Container build and final scans

Build the required amd64 image. Build success is the amd64 acceptance check;
runtime route checks on amd64 are not required. Preserve the existing arm64
build/runtime support.

```sh
docker buildx build --platform linux/amd64 --load -t exchange-api:spbex-amd64 .
```

On the final build, run Semgrep on source and Trivy against the built
deliverable. Resolve all Semgrep findings and all fixable High/Critical Trivy
findings before declaring the feature complete, and document any unavailable
fix.

Final validation on 2026-10-08: Semgrep scanned 21 Rust source files and
reported 0 findings. Trivy scanned `exchange-api:spbex-amd64` (Debian 13.7
runtime) and reported 0 High/Critical vulnerabilities, including 0 fixable
findings.

## Security and operations

The application is intended for a private network. SPBEX routes do not add
application-level authentication. Request completion logs include the normalized
symbol, cache outcome, status, outcome category, and elapsed time; dependency
and store failures are logged with diagnostic context.

The SPBEX client's TLS verifier uses the Russian Trusted Root CA as an
additional trust anchor for that client only. The rest of the service keeps the
platform trust store.

## Final shared-provider latency acceptance

On 2026-10-09, the final shared-handler binary was measured against production
SPBEX using SBER and 10 concurrent clients at 10 total requests per second. Each
timed profile scheduled and issued all 1,200 requests with zero skips and zero
upstream errors. History-only p95 was 0.003834 s, quote-only p95 with the final
16-request upstream limit was 0.751137 s, and combined overall p95 was 0.611771
s (history 0.002233 s, quote 0.912929 s). The combined result improved from an
initial 1.225619-second quote p95 after increasing the upstream concurrency
allowance from eight to sixteen. The history cache was warmed before timed
profiles and configured with a 300-second test TTL to avoid an expiry refresh
interrupting fixed-arrival measurements.

Final shared-provider quality gates on 2026-10-09 passed: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test` (81 tests), `cargo
test --test shutdown` (3 tests), and the Linux amd64 Docker image build. Semgrep
reported zero findings and Trivy reported zero fixable High/Critical image
vulnerabilities; detailed final scan results are recorded in the CBR quickstart.

## Production history cache lifecycle

The separate `scripts/measure-spbex-latency.sh --profile lifecycle --symbols
SBER` probe uses a fresh temporary store and measures cold fetch, warm memory
hit, and expiry-triggered source refresh without mixing those requests into a
fixed-arrival profile. On 2026-10-09, cold history returned 200 in 0.491263 s
(`miss`, one source fetch), warm history returned 200 in 0.000484 s
(`memory_hit`, no fetch), and the expiry refresh returned 200 in 0.226268 s
(`miss`, one source fetch). The probe verified all three transitions with the
default 15-second upstream timeout.
