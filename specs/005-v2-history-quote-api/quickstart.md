# Quickstart: V2 History and Quote Routes

## Prerequisites

- Rust toolchain matching the project (`Rust 1.85` or newer).
- The existing MOEX, SPBEX, and CBR provider configuration and history-store setup used by the service.

## Start the service

```sh
EXCHANGE_API_LISTEN_ADDR=127.0.0.1:8080 cargo run
```

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

**Fixture-backed v2 latency profile (2026-10-09)**: Ran six separate 10-second profiles (100 scheduled requests per route), with 10 concurrent clients and 10 requests per second, after warming the routes. All profiles issued 100 requests, had 100 successful responses, zero errors, zero missed slots, and 100/100 successful responses under one second. The recorded profile is a warm-cache measurement for MOEX history; the initial MOEX full-fetch latency was not recorded separately in this run.

| Route | Successful p95 |
|---|---:|
| MOEX history | 0.006428 s |
| MOEX quote | 0.003606 s |
| SPBEX history | 0.005446 s |
| SPBEX quote | 0.003535 s |
| CBR history | 0.005250 s |
| CBR quote | 0.004934 s |

A separate diagnostic profile against public upstreams passed for all three quote routes (MOEX 0.598259 s, SPBEX 0.327543 s, CBR 0.318874 s p95). Public-source history diagnostics recorded MOEX p95 17.558618 s (8 successes, 2 HTTP 502 errors, 90 missed slots), SPBEX p95 1.044326 s (99 successes, 1 missed slot), and CBR p95 2.855501 s (81 successes, 19 missed slots). These existing diagnostics do not document a separately timed initial MOEX fetch followed by a verified populated-cache run, so they do not establish SC-005 for public-source history. Repeat them using the cache-state procedure above before treating them as acceptance results. Source-backed results vary with upstream availability and latency; fixture-backed results measure the service route and storage path under a stable source.

**Linux amd64 build (2026-10-09)**: `docker buildx build --platform linux/amd64 -t exchange-api:v2-history-quote-api --load .` completed successfully.

**Semgrep (2026-10-09)**: Source scan completed with 0 findings. The two newly added Python scripts were scanned explicitly with `--no-git-ignore`; 0 findings.

```sh
docker run --rm -v "$PWD:/src" semgrep/semgrep:latest semgrep scan --config auto /src
docker run --rm -v "$PWD:/src" semgrep/semgrep:latest semgrep scan --config auto --no-git-ignore /src/scripts/measure-v2-latency.py /src/scripts/serve-v2-latency-fixtures.py
```

**Trivy (2026-10-09)**: `exchange-api:v2-history-quote-api` had 0 High or Critical vulnerabilities with available fixes.

```sh
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock aquasec/trivy:latest image --scanners vuln --severity HIGH,CRITICAL --ignore-unfixed exchange-api:v2-history-quote-api
```

**Rust validation (2026-10-09)**: `cargo test` passed all unit, integration, provider, API, persistence, and shutdown tests. `cargo fmt -- --check` passed.
