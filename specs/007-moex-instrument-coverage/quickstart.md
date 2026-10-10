# Quickstart: MOEX Benchmark and Currency Support

## Prerequisites

- Rust toolchain and Cargo from the repository toolchain file.
- Network access to public MOEX ISS for live checks, or the fixture-backed tests for offline checks.
- The existing service configuration. Subscriber credentials are not part of this feature.

## Fixture-backed validation

From the repository root:

```sh
cargo test moex
cargo test --test moex_index
cargo test --test moex_currency
cargo test --test api_moex
cargo test --test api_quote
cargo test --test api_contract
cargo test --test api_errors
cargo test cache_store
```

Expected outcomes:

- IMOEX history maps index `CLOSE`, `HIGH`, and `LOW` fields, sorts by trading date, and preserves the six-field response shape.
- GLDRUB_TOM history uses the CETS primary board, maps its available OHLC values, and returns null for source fields it does not provide.
- Index and currency quote requests use current marketdata from the resolved engine, market, and primary board; index quotes map the latest published index value and currency quotes map the latest CETS trade fields. Both return the established one-record quote shape and bypass history storage. Equities retain their existing trades-based quote request.
- Currency quote volume converts source lots to instrument units with the selected board LOTSIZE. A missing source field remains `null`; an empty or unavailable current value remains one all-null quote record.
- Newly supported symbols accept alphanumeric parts separated by single underscores after normalization; leading, trailing, and repeated underscores are rejected without changing existing symbols' validation behavior.
- A cached legacy `MOEX:{SYMBOL}` response is migrated only after symbol syntax validation and successful MOEX metadata recognition, but before collection lookup or history retrieval. The first history response includes migrated records combined with fetched records under existing merge rules. Route-level tests verify unknown symbols return HTTP 400 without migration, metadata failures return the MOEX dependency error without migration, and recognized symbols migrate before history retrieval while reusing the preflight metadata. If migration fails, the route returns the history-store error and the legacy row remains intact.
- Existing SBER responses, invalid-symbol errors, upstream errors, and v1/v2 route compatibility remain unchanged.

## Live ISS validation

Start the service with `cargo run` from the repository root. Then request:

```sh
curl -sS http://localhost:8080/v1/moex/IMOEX
curl -sS http://localhost:8080/v1/moex/IMOEX/quote
curl -sS http://localhost:8080/v1/moex/GLDRUB_TOM
curl -sS http://localhost:8080/v1/moex/GLDRUB_TOM/quote
curl -sS http://localhost:8080/v2/history/moex/IMOEX
curl -sS http://localhost:8080/v2/quote/moex/GLDRUB_TOM
```

Compare available values against the matching MOEX ISS market and board, not against a different board for the same symbol. Market values are time-sensitive; record the observation date when comparing live results. Verify a valid but empty response remains distinct from HTTP 400 invalid symbols and HTTP 502 MOEX dependency errors.

## Persistence and performance validation

Use a temporary configured history database for migration checks. Seed a legacy `MOEX:IMOEX` or `MOEX:GLDRUB_TOM` response and arrange for the metadata and history fixtures to return a recognized instrument and additional records. On the first recognized history request, verify the response and normalized MOEX collection contain both migrated and fetched records. Verify malformed and syntactically valid but unrecognized symbols do not trigger migration; simulate metadata failure and verify it returns the MOEX dependency error without migration. Restart the service and verify the dated records remain available. Inject a failed migration and verify the source row remains intact and the route returns the history-store error.

Measure the established MOEX workload of 10 concurrent clients at 10 requests per second. Report the initial full-history fetch separately; run the latency acceptance profile after histories are populated. Quotes must cause a fresh ISS read on each request.

## Implementation validation results (2026-10-10)

**Fixture-backed MOEX profile**: Started the release service with `scripts/serve-v2-latency-fixtures.py` and a fresh SQLite database. Initial history requests returned HTTP 200 in 0.004310 seconds for IMOEX and 0.002618 seconds for GLDRUB_TOM. After both histories were populated, `scripts/measure-moex-latency.py --duration-seconds 55 --symbols IMOEX,GLDRUB_TOM --profile all` ran separate history, quote, and combined profiles with 10 client slots and 10 requests per second. Each profile issued 550/550 requests with no errors or missed slots. Latencies below are pooled across the two symbols.

| Profile | Successful responses | p95 | Errors | Missed slots | p95 under 1 second |
|---|---:|---:|---:|---:|---:|
| History | 550/550 | 0.011752 s | 0 | 0 | 550/550 |
| Quote | 550/550 | 0.008072 s | 0 | 0 | 550/550 |
| Combined history | 275/275 | 0.011318 s | 0 | 0 | 275/275 |
| Combined quote | 275/275 | 0.008207 s | 0 | 0 | 275/275 |

The fixture-backed workload validates route and storage overhead; it does not measure public MOEX latency or upstream rate limits. Requests completed faster than the 100 ms arrival interval, so the runner observed one in-flight request at a time despite having 10 client slots available.

**Semgrep**: `docker run --rm -v "$PWD:/src" semgrep/semgrep semgrep scan --config auto --no-git-ignore /src/src/moex` completed on 9 MOEX source files with 0 findings.

**Linux amd64 build**: `docker buildx build --platform linux/amd64 -t exchange-api:moex-instrument-coverage --load .` completed successfully.

**Trivy image scan**: `docker run --rm -v /var/run/docker.sock:/var/run/docker.sock aquasec/trivy:latest image --scanners vuln --severity HIGH,CRITICAL exchange-api:moex-instrument-coverage` found 0 High or Critical vulnerabilities in the image's Debian 13.7 packages. Trivy reported no language-specific files in the image.
