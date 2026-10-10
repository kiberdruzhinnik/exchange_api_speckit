# Research: SPBEX Ticker History and Quote API

## SPBEX chart feed and mapping

- **Decision**: Use the public chart endpoint `https://spbexchange.ru/api/reader/marketdata/charts/chistory` with normalized `symbol`, daily `resolution=1440`, `from=0`, and `to` set to the current Unix timestamp. Parse `bar_unixtime` (Unix seconds), `close`, `high`, and `low`; convert the timestamp to UTC; set `facevalue` to `1` and `volume` to null. Validate positive finite prices, timestamp, OHLC consistency, and duplicate timestamps; sort history ascending. Exclude candles dated on the current UTC calendar date from the history response before caching or persisting it; quote continues to return the latest candle including today's candle.
- **Rationale**: This preserves the source behavior requested by the user and matches the SPBEX adapter’s feed and `GetCurrentPrice` semantics. The referenced Go adapter has no volume in its chart candle type, so its `uint64` field defaults to zero; the clarified API contract uses null to mark absent source data.
- **Alternatives considered**: A separate executed-trade source was rejected because the user confirmed quote means the latest available daily candle. Returning zero volume was rejected because the feed provides no measurement.
- **References**: [SPBEX adapter](https://github.com/kiberdruzhinnik/go-exchange-api/blob/main/api/spbex.go), [adapter constants](https://github.com/kiberdruzhinnik/go-exchange-api/blob/main/constants/constants.go), [SPBEX chart page](https://spbexchange.ru/market-data/charts/).

## Quote freshness and latency

- **Decision**: On every quote request, query daily candles from a 1-day lookback ending at the current time. If that range is empty, double the lookback and query again, continuing until a candle is returned or the range reaches the Unix epoch. Select the newest candle found and return it as a one-element array. If the full available range is empty, return one all-null record. Do not read or write Moka or SQLite on the quote path.
- **Rationale**: The user selected the adapter’s latest-available-candle semantics and requires a fresh upstream read on each request. The chart endpoint accepts `from` and `to`; a recent 24-hour window minimizes source work for active symbols, while expansion preserves the latest available record for inactive symbols. The endpoint does not document a separate `latest` operation.
- **Alternatives considered**: A full-history request on every quote is the simplest exact match to `GetCurrentPrice`, but transfers the full history even for actively traded instruments. A fixed recent window without expansion can return a false no-quote result when the most recent candle is older. A cached quote violates freshness, and a separate trade feed changes the clarified behavior.
- **Performance gate**: Run the local release binary against production SPBEX with 10 concurrent clients and 10 total requests per second. Measure history-only, quote-only, and combined profiles; consume complete response bodies and report p95 per route, successes, errors, and actual issue rate. A run that skips or issues fewer than 10 requests per second is invalid and cannot pass. Each valid profile must achieve successful-response p95 below one second. If any valid profile misses, optimize the request/source path and rerun until it passes; record each run and the change made. Keep quote requests uncached during every run.

### Production acceptance run (2026-10-08)

- **Harness**: local optimized release binary, production SPBEX feed, SBER (confirmed active SPBEX symbol), 10 concurrent workers, fixed 10 requests/second arrival schedule, 120 seconds per profile, full response bodies consumed. The harness caps in-flight benchmark requests at ten and measures actual start rate.
- **History-only**: 1,200 scheduled/issued, 0 skipped, 0 errors, observed start rate 10.01 req/s, p95 0.003534 s.
- **Quote-only**: 1,200 scheduled/issued, 0 skipped, 0 errors, observed start rate 10.01 req/s, p95 0.458487 s.
- **Combined**: 1,200 scheduled/issued, 0 skipped, 0 errors, observed start rate 10.01 req/s, p95 0.502252 s; route p95 history 0.004526 s and quote 0.764562 s.
- **Outcome**: All three full-rate profiles pass the one-second p95 gate. Initial tests using sequential per-client waits allowed slower responses to stretch the actual arrival rate; the harness now submits requests on a fixed schedule. Under that valid schedule, ten upstream slots produced quote-only p95 above one second, while five slots also narrowly missed on a full-duration run. The final configuration uses the SPBEX-scoped root CA, a 1-day initial quote window with adaptive expansion, an sixteen-request upstream concurrency limit, ten benchmark workers, and active SBER. No quote caching was introduced.

## Rust service integration and history persistence

- **Decision**: Add `src/spbex/` alongside `src/moex/` and extend application state with a reusable SPBEX client. Add `/v1/spbex/{symbol}` and `/v1/spbex/{symbol}/quote` handlers in the existing router. Cache only fully validated serialized history using the current shared `HistoryCache` and SQLite `CacheStore`, with keys prefixed `SPBEX:` and qualified by the current UTC date so a pre-midnight filtered payload is not reused after date rollover. Quotes bypass both cache layers.
- **Rationale**: The service already provides async HTTP, reqwest timeouts, bounded response reads, TTL/singleflight history caching, persistent history, structured logs, and the required response record fields. Exchange-qualified keys prevent collisions with MOEX history for the same ticker.
- **Alternatives considered**: A separate SPBEX service or persistence store would duplicate existing capability and add operational coupling. Refactoring the MOEX client into a generic exchange abstraction is not required for two independently testable adapters.
- **Configuration**: Configure the SPBEX API base URL through `EXCHANGE_API_SPBEX_API_BASE_URL` and its response byte limit through `EXCHANGE_API_SPBEX_MAX_RESPONSE_BYTES`; reuse the service’s shared timeout and history cache TTL/capacity settings. Keep canonical names and defaults documented in `quickstart.md`.
- **TLS trust**: SPBEX currently serves a certificate chain issued by Russian Trusted Sub CA. Add the official Russian Trusted Root CA as an extra trust anchor to the SPBEX reqwest client only; retain standard certificate and hostname verification.

## Error and symbol handling

- **Decision**: Reuse the existing symbol syntax convention (trim whitespace, uppercase, accept ASCII letters, digits, period, underscore, and hyphen). Return malformed symbols as HTTP 400. Treat a successful empty candle array as valid empty history; after quote lookback expansion reaches the Unix epoch with no candles, return one all-null record. Map explicit upstream not-found/unknown-symbol responses to HTTP 400 where distinguishable, and other transport, status, oversized, malformed, or invalid-candle responses to HTTP 502. Durable-store errors on history return HTTP 503.
- **Rationale**: This implements the clarified distinction between malformed input, explicit source rejection, valid empty data, and dependency failure using the existing JSON error envelope. The SPBEX chart feed is not a separate instrument catalog, so a successful empty array is not evidence that a symbol is unsupported.
- **Alternatives considered**: Interpreting every empty array as invalid would violate the successful-empty-history behavior. Introducing an independent instrument catalog is outside current scope.

## Validation and delivery

- **Decision**: Use fixture-backed mapping/client/route tests, OpenAPI schema validation, persistent-cache restart validation, and production latency measurement with the compiled local binary. Build the Docker image for `linux/amd64`; preserve existing `linux/arm64` build/runtime support without adding amd64 runtime checks to acceptance.
- **Rationale**: These checks fit repository patterns and prove the new API independently of live market changes. The source benchmark must include both routes and combined traffic because only quote calls expose upstream latency on each request.
- **Required final scans**: Run Semgrep on source and Trivy on the built deliverable. Resolve all Semgrep findings and all fixable High/Critical Trivy findings; document scan outcomes and any unavailable fixes.
