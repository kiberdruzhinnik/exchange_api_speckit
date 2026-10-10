# Quickstart: V2 History and Quote Routes

## Prerequisites

- Rust toolchain matching the project (`Rust 1.85` or newer).
- The existing MOEX, SPBEX, and CBR provider configuration and history-store setup used by the service.

## Start the service

```sh
EXCHANGE_API_LISTEN_ADDR=127.0.0.1:8080 cargo run
```

`EXCHANGE_API_HISTORY_FULL_REFRESH_INTERVAL_SECS` and `EXCHANGE_API_HISTORY_REFRESH_RETRY_MAX_BACKOFF_SECS` are optional. When omitted, they default to 604800 seconds (seven days) and 900 seconds, respectively. These settings control background history refresh and retries; they do not enable or disable v2 routes. The v2 history and quote routes are available by default.

Configure any provider source and persistent history settings as required by the existing service deployment.

## Validate the v2 routes

Use supported provider-symbol pairs. The provider path segment is lowercase and selects exactly one adapter.

```sh
curl -i http://127.0.0.1:8080/v2/history/moex/SBER
curl -i http://127.0.0.1:8080/v2/quote/moex/SBER
curl -i http://127.0.0.1:8080/v2/history/spbex/SBER
curl -i http://127.0.0.1:8080/v2/quote/spbex/SBER
curl -i http://127.0.0.1:8080/v2/history/cbr/USD
curl -i http://127.0.0.1:8080/v2/quote/cbr/USD
```

Expected behavior:

- History returns the selected provider's complete retained history in its existing order and six-field format.
- Quote returns the selected provider's existing one-record current quote result.
- Invalid provider values return HTTP 400 with `invalid_provider`; malformed or unsupported symbols retain the `invalid_symbol` behavior.
- Upstream failures retain provider-specific HTTP 502 codes. History-store failures return HTTP 503.
- Existing v1 routes continue to work, for example `/v1/moex/SBER` and `/v1/moex/SBER/quote`.

## Automated API and contract checks

Run the Rust suite and verify it covers v2 provider selection, each history/quote response path, invalid provider and symbol errors, upstream/store errors, v1 compatibility, and both the feature contract and canonical OpenAPI contract:

```sh
cargo test
```

The subprocess regression test starts the service with both refresh settings absent and checks that the v2 routes reach their handlers:

```sh
cargo test --test api_default_config
```

## Performance acceptance

Run the six separate workload profiles with the local service running. Each profile uses 10 concurrent clients and 10 total requests per second for one route. The runner defaults to 55 seconds per profile (550 scheduled requests); shorter runs can be selected for a quick check.

For fixture-backed profiles, start the fixture upstream in one terminal:

```sh
python3 scripts/serve-v2-latency-fixtures.py
```

Start the service in another terminal with a fresh history database:

```sh
EXCHANGE_API_LISTEN_ADDR=127.0.0.1:8080 \
EXCHANGE_API_HISTORY_CACHE_DB_PATH=/tmp/exchange-api-v2-benchmark.sqlite3 \
EXCHANGE_API_MOEX_ISS_BASE_URL=http://127.0.0.1:18081/iss/ \
EXCHANGE_API_SPBEX_API_BASE_URL=http://127.0.0.1:18081/spbex/api/ \
EXCHANGE_API_CBR_API_BASE_URL=http://127.0.0.1:18081/cbr/ \
cargo run --release
```

For MOEX, first measure the initial uncached full-history fetch separately using a fresh database. Record its HTTP status and elapsed time; this paginated fetch is reported but is not subject to the one-second criterion:

```sh
curl --silent --show-error --output /dev/null \
  --write-out 'http_code=%{http_code} time_total_s=%{time_total}\n' \
  http://127.0.0.1:8080/v2/history/moex/SBER
```

Run the gated profiles only after that request has successfully populated the complete MOEX history. Warm the six routes once, then run the workload; the MOEX history profile now measures subsequent requests against the populated cache:

```sh
for path in \
  /v2/history/moex/SBER /v2/quote/moex/SBER \
  /v2/history/spbex/SBER /v2/quote/spbex/SBER \
  /v2/history/cbr/USD /v2/quote/cbr/USD; do
  curl --silent --show-error --fail "http://127.0.0.1:8080$path" >/dev/null
done
```

If the initial MOEX fetch fails, record the status and outcome, fix or wait for the source, and do not treat a partially populated collection as the completed-cache acceptance run.

Run the workload:

```sh
python3 scripts/measure-v2-latency.py --base-url http://127.0.0.1:8080 --duration-seconds 55
```

The runner records successful-response p95, the fraction of successful responses under one second, errors, and missed request slots for each route. At least 95% of successful responses in every profile must complete in under one second. The initial uncached MOEX full-fetch result is reported separately and excluded from that gate; the MOEX profile must run after the full history is populated. History requests use persistent storage. Failed responses are reported separately from successful-response latency.

## Implementation validation results

**Fixture-backed v2 latency acceptance profile (2026-10-10)**: Started with a fresh SQLite database. The initial uncached MOEX history request returned HTTP 200 in 0.004339 seconds against the one-page fixture and populated the collection; the six-route workload then ran after warming all routes. Each profile ran for 55 seconds with 550 scheduled requests, 10 concurrent clients, and 10 requests per second. All profiles issued 550 requests, returned 550 successes, had zero errors and zero missed slots, and had 550/550 successful responses under one second.

| Route | Successful p95 | Under one second | Errors | Missed slots |
|---|---:|---:|---:|---:|
| MOEX history (populated cache) | 0.007175 s | 550/550 | 0 | 0 |
| MOEX quote | 0.003819 s | 550/550 | 0 | 0 |
| SPBEX history (warmed) | 0.011301 s | 550/550 | 0 | 0 |
| SPBEX quote | 0.005003 s | 550/550 | 0 | 0 |
| CBR history (warmed) | 0.006338 s | 550/550 | 0 | 0 |
| CBR quote | 0.004519 s | 550/550 | 0 | 0 |

**Public-source MOEX cold-fetch diagnostic (2026-10-10)**: With a separate fresh database, the initial MOEX full-history request returned HTTP 200 in 5.031771 seconds and persisted 3,423 records. This exceeds one second, as expected for a paginated cold fetch, and is reported separately from the cache-populated acceptance target.

**Public-source diagnostics (2026-10-10)**: A subsequent run against the same service process measured the populated-cache MOEX history route at 0.692063 s p95, with 537/546 successful responses under one second, one HTTP 502, and 3 missed request slots (547/550 issued). Its successful-response latency fraction met the threshold, but the runner marked the profile's arrival schedule invalid. MOEX quote passed at 0.462408 s p95 (549/550 under one second; no errors or missed slots). SPBEX history recorded 0.453220 s p95 (530/540 under one second; no errors; 10 missed slots), and SPBEX quote passed at 0.380976 s p95 (550/550 under one second; no errors or missed slots). These SPBEX profiles were not preceded by the required warm-up, so they are diagnostic only. CBR history recorded 1.848394 s p95 among 143 successful responses (133/143 under one second), with 394 HTTP 502 errors and 13 missed slots; CBR quote recorded 0.081780 s p95 among 208 successful responses (208/208 under one second), with 342 HTTP 502 errors. Service logs identify upstream HTTP 429 rate limits for the CBR errors. These public-source results are diagnostics, not the controlled SC-005 acceptance run; they show external source limits and must not be presented as fixture-backed acceptance results.
**Linux amd64 build (2026-10-10)**: `docker buildx build --platform linux/amd64 -t exchange-api:v2-history-quote-api --load .` completed successfully.

**Semgrep (2026-10-10)**: The tracked-source scan completed with 0 findings across 139 files. Because the two benchmark scripts are untracked, they were scanned separately with `--no-git-ignore`; that scan covered 2 files and also found 0 issues.

```sh
docker run --rm -v /Users/konstantin/repos/agents/opencode2-workspace/coding/exchange_api:/src semgrep/semgrep:latest semgrep scan --config auto /src
docker run --rm -v /Users/konstantin/repos/agents/opencode2-workspace/coding/exchange_api:/src semgrep/semgrep:latest semgrep scan --config auto --no-git-ignore /src/scripts/measure-v2-latency.py /src/scripts/serve-v2-latency-fixtures.py
```

**Trivy (2026-10-10)**: `exchange-api:v2-history-quote-api` had 0 High or Critical vulnerabilities with available fixes.

```sh
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock aquasec/trivy:latest image --scanners vuln --severity HIGH,CRITICAL --ignore-unfixed exchange-api:v2-history-quote-api
```

**Rust validation (2026-10-10)**: `cargo test` passed all unit, integration, provider, API, persistence, and shutdown tests. `cargo fmt -- --check` passed.
