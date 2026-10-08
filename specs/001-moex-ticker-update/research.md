# Research: MOEX Ticker History API

## Decisions

### Rust HTTP service

- **Decision**: Keep the existing single Rust web service using Axum/Tokio, `reqwest` with Rustls, Serde, and `tracing`. Add SQLx with SQLite support for the durable history cache; retain Moka as the bounded in-memory hot cache and same-symbol singleflight layer.
- **Rationale**: This matches the existing repository and async request flow. SQLx provides async SQLite access; SQLx 0.8's `sqlite` feature builds and links SQLite from source, so the runtime does not need a SQLite shared library. The builder must provide target-aware C build tools for both supported architectures.
- **Alternatives considered**: A separate database service adds deployment and network dependencies for a small per-instance cache. A JSON-file cache requires custom indexing, locking, atomic replacement, and pruning behavior. A process-local cache alone does not satisfy restart persistence.
- **References**: [SQLx 0.8.6 SQLite driver](https://docs.rs/sqlx/0.8.6/sqlx/sqlite/index.html), [Tokio runtime](https://docs.rs/tokio/latest/tokio/runtime/), [Reqwest TLS](https://docs.rs/reqwest/latest/reqwest/tls/).

### Durable per-instance history cache

- **Decision**: Store complete serialized history responses in an embedded SQLite database under the configured `MOEX_HISTORY_CACHE_DB_PATH` on a durable volume attached to each service instance. Use a row keyed by symbol with the JSON body, fetch time, expiry time, and logical body byte count. Default the cache TTL to 60 seconds and its maximum aggregate cached response bytes to 64 MiB. Keep an equally bounded Moka hot cache in front of SQLite; a hot-cache miss checks the durable store before contacting ISS. Cache only complete successful history responses. A database hit is usable only while `expires_at` is later than the current time. Persist by transaction, and prune expired entries then oldest-fetched entries to remain within the configured payload capacity.
- **Rationale**: SQLite makes complete replacements atomic and supports indexed symbol/expiry lookups without requiring an additional service. The existing Moka layer preserves low-latency warm requests and request coalescing. The mounted database survives an application process/container restart; a local volume per instance matches the user decision and avoids cross-instance cache consistency requirements.
- **Operational constraints**: Enable SQLite WAL for concurrent readers and short writes. Keep the database, `-wal`, and `-shm` files on the same local per-instance filesystem. SQLite WAL requires processes to share a host and is not suitable for a database concurrently accessed over a multi-host network filesystem. Do not share an instance database among replicas. Use a bounded busy timeout/pool, short write transactions, and an explicit synchronous mode suitable for a cache that must survive application restart. Mount the volume at `/var/lib/exchange-api`; the container runs as UID/GID 65532 and the directory must be writable by that identity.
- **Alternatives considered**: A remote relational database is unnecessary for non-shared per-instance data. A single snapshot file is possible but requires custom crash-safe writes and capacity accounting. WAL is not used on a shared multi-host filesystem.
- **References**: [SQLite WAL concurrency and same-host constraint](https://www.sqlite.org/wal.html), [SQLite transaction behavior](https://www.sqlite.org/lang_transaction.html), [SQLite synchronous durability settings](https://www.sqlite.org/pragma.html), [SQLx SQLite](https://docs.rs/sqlx/0.8.6/sqlx/sqlite/index.html).

### MOEX ISS history and latest trades

- **Decision**: Continue using named ISS response blocks/columns and cursor/offset pagination for daily history. The first history page provides the cursor total and page size; fetch remaining offsets concurrently with a 128-page per-symbol bound, validating consistent columns and the aggregate response-byte limit before mapping. For the quote, call the ISS trades resource on every request with a one-row limit and reverse ordering so the returned row is the latest executed trade. Map `PRICE` to `price`, `QUANTITY` to `size`, and the execution date/time fields (`TRADEDATE` and `TRADETIME`) to an ISO 8601 UTC `time`. Convert the Moscow exchange-local timestamp to UTC. Never put trade responses in Moka or SQLite.
- **Rationale**: The user selected the latest executed trade, not bid/ask values. The ISS trades response exposes the execution values directly. The `limit=1` reverse query avoids downloading a session's full trade list. A valid response with an empty trades block means no latest trade and maps to the all-null response. HTTP errors, malformed JSON, missing required columns, or invalid values are dependency failures.
- **Endpoint pattern**: `GET /iss/engines/stock/markets/shares/securities/{SYMBOL}/trades.json?limit=1&reversed=1&iss.meta=off`. If market-level routing is unavailable for a particular ISS market, resolve and use the primary board for the current trading date, without changing history board-selection behavior.
- **Alternatives considered**: Use a current `LAST` marketdata field (less directly tied to the executed-trade record); request only one fixed board (may miss the latest trade if board assignment changes); cache the quote (contradicts explicit freshness); treat an empty trades block and upstream failure identically (contradicts the clarified error behavior).
- **References**: [MOEX ISS trades reference](https://iss.moex.com/iss/reference/425), [MOEX ISS developer manual](https://www.moex.com/files/4be999zbzp80bx2bgmwayrtyx0), [MOEX description of ISS data and delayed unauthenticated access](https://www.moex.com/a8531). A production ISS probe for SBER/TQBR returned `TRADEDATE`, `TRADETIME`, `PRICE`, and `QUANTITY` in the trades block.

### Primary-board history selection and `facevalue`

- **Decision**: Fetch unauthenticated history with named columns and follow the ISS history cursor until all available pages are retrieved. Use listing history to select the board primary on each trading date, then emit the daily row for that board and sort ascending. Map `date` from `TRADEDATE`, price fields and volume by name, and the API's `facevalue` property from current `LOTSIZE` for the selected primary board. Use null for unavailable nullable values and return the documented dependency error if a record lacks a valid date or cannot be assigned to a primary board.
- **Rationale**: This implements the clarified per-date board rule and preserves the requested response shape. The expected SBER value of `1` comes from board-specific LOTSIZE, not generic FACEVALUE (`3`) or a historical LOTSIZE series.
- **Alternatives considered**: Hard-code TQBR, use today's board for all past rows, combine rows from multiple boards, map positional response fields, or use FACEVALUE.
- **References**: [MOEX ISS developer manual](https://www.moex.com/files/4be999zbzp80bx2bgmwayrtyx0), [SBER primary board/security metadata](https://iss.moex.com/iss/securities/SBER.json?iss.meta=off), [SBER TQBR reference data](https://iss.moex.com/iss/engines/stock/markets/shares/boards/TQBR/securities/SBER.json?iss.meta=off).

### Latency profile and acceptance measurement

- **Decision**: Define normal operating conditions as 10 concurrent clients issuing 10 requests per second total. Measure complete response-body latency from request receipt to response completion. Run history-only, quote-only, and combined endpoint workloads at that profile; report p95 for each route and the combined run. Include cache hits, cold/expired history misses, and uncached quote calls. Use the locally compiled application binary and production `https://iss.moex.com/iss` for live acceptance.
- **Rationale**: The fixed concurrency and rate make the user's p95 target reproducible. Route-specific results prevent a fast history cache from masking slow uncached quote responses. The previous live history run recorded cold misses around 12.9–22.4 seconds, so the target depends on the normal symbol reuse/cache-hit ratio; those misses must remain visible in the aggregate result.
- **Alternatives considered**: Measure only warm-cache history (would omit a real path); report only the aggregate of a mixed route set (could hide a slow endpoint); exclude successful cold misses or quote calls (would misrepresent the requirement).
- **Acceptance interpretation**: At least 95% of successful requests in each measured endpoint workload must complete under one second. Upstream errors remain failures and are counted separately; slow successful quote calls count toward quote p95.

### Unauthenticated data and error mapping

- **Decision**: Send no subscriber credentials. Use only data returned to an unauthenticated ISS client. Return `400` for malformed or unsupported symbols, `502` for failed or malformed MOEX responses, `503` when the history store is unavailable, and a successful one-element all-null quote array only when ISS succeeds and reports no trade. Use the shared error envelope for errors.
- **Rationale**: An ISS access failure is not equivalent to an empty history or no executed trade. MOEX documents that free ISS data may be delayed and that data availability depends on the endpoint/product.
- **References**: [MOEX ISS access overview](https://www.moex.com/a8531), [MOEX ISS developer manual](https://www.moex.com/files/4be999zbzp80bx2bgmwayrtyx0).

### Docker runtime and architectures

- **Decision**: Use the existing multi-stage Docker Buildx approach with a target-aware Rust builder and `gcr.io/distroless/cc-debian13:nonroot` runtime for `linux/amd64` and `linux/arm64`. Keep an exec-form entrypoint and port 8080. Create `/var/lib/exchange-api` writable by UID/GID 65532 for the mounted SQLite volume.
- **Rationale**: This matches the user's container constraints. Bundled SQLite is statically linked, while the existing Rust TLS/runtime linkage still uses the distroless C/C++ runtime. Cross-compilation uses the target Rust triple and matching C compiler.
- **Alternatives considered**: Build only for the developer host, use a general-purpose runtime, or copy an architecture-specific shared library into the image.
- **References**: [Docker multi-platform builds](https://docs.docker.com/build/building/multi-platform/), [Docker multi-stage builds](https://docs.docker.com/get-started/docker-concepts/building-images/multi-stage-builds/), [Distroless supported architectures](https://github.com/GoogleContainerTools/distroless/blob/main/README.md), [Distroless cc image](https://github.com/GoogleContainerTools/distroless/blob/main/cc/README.md).

### Bounded SIGINT shutdown

- **Decision**: Handle Ctrl+C with Tokio's signal support and Axum graceful shutdown. Once SIGINT is received, stop accepting new connections and allow in-flight requests and owned background work to finish for at most 30 seconds. Track or otherwise retain control of spawned work so work still active at the deadline can be cancelled before process exit. Log the signal, drain result, and deadline cancellation outcome.
- **Rationale**: Axum's graceful-shutdown API stops serving after the supplied future completes and its example waits for outstanding requests. Tokio recommends separating signal detection, notifying tasks, and waiting for tasks. A deadline is required so stuck upstream or client work cannot keep the service alive indefinitely. Tokio timeout cancels by dropping its future, so detached or independently spawned tasks must also be tracked/cancelled; merely timing out the top-level serve future is not sufficient evidence that all work has stopped.
- **Alternatives considered**: Rely on the operating system's default SIGINT termination (does not drain requests); wait indefinitely for graceful completion (can hang); time out only the serve future without controlling child tasks (does not guarantee that spawned work ends).
- **References**: [Tokio graceful shutdown guide](https://tokio.rs/tokio/topics/shutdown), [Tokio `ctrl_c`](https://docs.rs/tokio/latest/tokio/signal/fn.ctrl_c.html), [Axum graceful-shutdown API](https://docs.rs/axum/latest/axum/serve/struct.Serve.html), [Axum graceful-shutdown example](https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs), [Tokio timeout cancellation behavior](https://docs.rs/tokio/latest/tokio/time/fn.timeout.html).

## Resolved Decisions

- **Language/runtime**: Existing Rust stable service, edition 2024, MSRV 1.85; Axum/Tokio.
- **History persistence**: SQLite on a durable volume attached to each service instance; no shared database across instances.
- **History cache**: Complete serialized successful responses only; 60-second TTL, 64 MiB default logical payload capacity, bounded Moka hot cache backed by SQLite, expiry checked before serving.
- **Quote behavior**: One ISS latest-trade request per API quote call, no quote cache; return a one-element history-shaped array with `date`, `close`, `high`, `low`, `volume`, and `facevalue`. Map trade time, price, and size to `date`, `close`, and `volume`; set the other fields to null. An empty valid trades block returns one record with all six fields null; failed or malformed ISS responses use the dependency error.
- **MOEX data source**: Production `https://iss.moex.com/iss`, unauthenticated, with no subscriber credentials.
- **Performance profile**: 10 concurrent clients, 10 requests per second total; p95 measured end to end on each route and combined.
- **Container**: Docker/Buildx, distroless Debian 13 nonroot, Linux amd64 required and arm64 supported.
- **Shutdown**: Ctrl+C stops accepting new connections, drains in-flight work, and exits within 30 seconds; remaining tracked work is cancelled at the deadline.
- **Validation**: Cargo unit/integration/contract checks, API contract validation, persistence/restart acceptance, production-data and latency measurement with the locally compiled binary, successful image builds for Linux amd64 and arm64, arm64 runtime smoke validation, and final Semgrep and Trivy scans. AMD64 acceptance requires image build success only; amd64 runtime startup and endpoint checks are outside acceptance.
