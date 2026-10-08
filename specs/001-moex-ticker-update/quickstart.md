# Quickstart

## Run locally

Create a local directory for the durable history cache and start the service:

```sh
mkdir -p ./data
MOEX_HISTORY_CACHE_DB_PATH=./data/history.sqlite3 cargo run
```

The service listens on `0.0.0.0:8080`. `MOEX_HISTORY_CACHE_DB_PATH` selects the history cache database path (container default `/var/lib/exchange-api/history.sqlite3`). `MOEX_REQUEST_TIMEOUT_SECS` sets the MOEX request timeout (default 15 seconds). `MOEX_HISTORY_CACHE_TTL_SECS` sets history freshness/cache TTL (default 60 seconds). `MOEX_HISTORY_CACHE_MAX_BYTES` sets the maximum cached response payload bytes (default 64 MiB). `MOEX_MAX_ISS_RESPONSE_BYTES` bounds each ISS response (default 4 MiB), and `MOEX_MAX_HISTORY_BYTES` bounds accumulated history (default 64 MiB). `MOEX_ISS_BASE_URL` can point to a compatible ISS endpoint in fixture-backed tests.

Request daily history:

```sh
curl http://localhost:8080/v1/moex/SBER
```

The response is an ascending JSON array of daily records. `date` is the MOEX trading date at midnight UTC; unavailable market values may be `null`; `facevalue` is populated from the current board-specific `LOTSIZE` for the primary board selected on that trading date. ISS history is requested without subscriber credentials and is limited to data MOEX exposes without a subscription. A recognized ticker with no history returns `[]`; malformed or unsupported symbols return `400`; upstream denial, timeout, malformed JSON, or unusable history returns `502`; history-store failures return `503`.

Request the latest executed trade:

```sh
curl http://localhost:8080/v1/moex/SBER/quote
```

The service fetches the latest trade from MOEX ISS for each request and does not reuse a prior quote. The response is a one-element array with the same six fields as history: trade time (ISO 8601 UTC), price, and size map to `date`, `close`, and `volume`; `high`, `low`, and `facevalue` are null. A recognized ticker with a valid ISS response containing no trades returns `200` with one record whose six values are null. An ISS failure or malformed trade response returns `502`.

Known history comparison case: `SBER` on `2026-10-06`; expected `facevalue` is `1`.

## Graceful shutdown

Start the service locally, then press Ctrl+C in the terminal running it. The service stops accepting new connections, lets in-flight requests finish, and exits within 30 seconds. Work still active at the deadline is cancelled. The grace-period timer starts when SIGINT is received.

Automated shutdown acceptance runs with `cargo test --test shutdown`. It verifies SIGINT handling in a local service process, completion of a controlled in-flight HTTP request after shutdown starts, and cancellation of work when a short test deadline expires.

Shutdown implementation validation on 2026-10-08: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` passed (38 tests). The final `linux/amd64` and `linux/arm64` distroless images built successfully. The arm64 image returned ready status and exited with status 0 after SIGINT. Semgrep `auto` scanned source, tests, workflow, and container build files with zero findings after pinning GitHub Actions to commit SHAs. Trivy found zero fixable High or Critical vulnerabilities in either image.

## Run automated checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

The test suite includes fixture-backed ISS client and mapping cases, both API routes and error outcomes, OpenAPI schema checks, and persistent-cache expiry/restart coverage.

Validation on 2026-10-08: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` all passed.

Security scans on 2026-10-08 used Semgrep `auto` rules on 63 tracked files, including source, tests, scripts, and Dockerfile (338 rules; zero findings). Trivy scanned both final architecture images for High and Critical vulnerabilities with fixes available; it reported zero findings on each image.

## Validate latency

Use the locally compiled application binary and production `https://iss.moex.com/iss`. Apply a total load of 10 concurrent clients issuing 10 requests per second. Measure from request receipt through completion of the complete response body. Measure the history route alone, the quote route alone, and a combined run; report p95 for each route and the combined run. Include cold and expired history misses as well as history cache hits. The quote endpoint always contacts ISS. At least 95% of successful requests must finish under one second. Report upstream errors separately; successful slow quote calls count toward the quote p95.

Run the reproducible acceptance workload (the default 120 seconds per route includes a history-cache expiry and enough warm requests to measure a stable p95):

```sh
scripts/measure-moex-latency.sh --duration-seconds 120 --symbols SBER,GAZP,VTBR,ROSN
```

The script builds and runs the local release binary against production ISS, reads every response body before recording latency, and prints success counts, actual request rate, route p95, and upstream errors.

The bounded-scheduler acceptance run on 2026-10-08 used the local release binary and production ISS with four symbols for 120 seconds per profile. It scheduled requests at 10 per second with ten client workers, allowed at most one active request per client, skipped saturated or overdue ticks instead of queueing work for later bursts, and reported scheduled, issued, skipped, successful, and failed requests. History scheduled 1,200 requests, issued 1,006, skipped 194, had 996 successes and 10 SBER HTTP 502 errors, achieved 8.38 issued requests/second, and had successful-response p95 of 0.0025 seconds. Quote scheduled and issued 1,200 requests, skipped none, had 1,199 successes and one SBER HTTP 502 error, achieved 10.00 requests/second, and had successful-response p95 of 0.418 seconds. Combined scheduled 1,200 requests, issued 1,184, skipped 16, had no errors, achieved 9.87 issued requests/second, and had successful-response p95 of 0.448 seconds; history p95 was 0.0046 seconds and quote p95 was 0.480 seconds. Quote met the full target issue rate. The history-only run was capacity-limited, and the combined run was slightly below 10 requests/second; their p95 values are recorded for the actual issued loads. History traffic included cache hits, misses, and expirations. A live SBER history response contained `2026-10-06T00:00:00Z` with `facevalue: 1`.

## Health and readiness

`GET /health/live` reports process liveness. `GET /health/ready` reports readiness and returns `503` if the history store cannot be queried.

## Run in Docker with persistent history

Build the required amd64 image:

```sh
docker buildx build --platform linux/amd64 --load -t exchange-api:amd64 .
mkdir -p ./exchange-api-data
sudo chown 65532:65532 ./exchange-api-data
docker run --rm --platform linux/amd64 -p 8080:8080 \
  -v "$PWD/exchange-api-data:/var/lib/exchange-api" \
  -e MOEX_HISTORY_CACHE_DB_PATH=/var/lib/exchange-api/history.sqlite3 \
  exchange-api:amd64
```

The Rust build stage compiles the binary and the runtime uses the distroless nonroot image. The history database and its SQLite WAL sidecars must remain together in the mounted per-instance directory. Do not mount one WAL database for concurrent access from multiple hosts or replicas. Restarting the container with the same mounted directory preserves unexpired history; expired history is fetched again. The quote endpoint never reads or writes the history cache.

Architecture validation on 2026-10-08 built separate `linux/amd64` and `linux/arm64` images with Buildx. This satisfies AMD64 acceptance, which requires successful image build only; starting the image or exercising routes on amd64 is outside acceptance. The arm64 image also started successfully with a Docker managed volume and returned `{"status":"ready"}` and `{"status":"ok"}` from readiness and liveness.

Build a multi-architecture image for amd64 deployment and arm64 development:

```sh
docker buildx build --platform linux/amd64,linux/arm64 --push -t registry.example/exchange-api:latest .
```

Verify that the build produces both image platforms. AMD64 validation is build-only; no AMD64 runtime or route check is required. The arm64 runtime smoke check may be run on a compatible host.
