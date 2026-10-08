# Research: MOEX Ticker History API

## Decisions

### Rust HTTP service

- **Decision**: Build a single Rust web service using the current stable Rust toolchain, Axum on Tokio, and `reqwest` for outbound HTTP with an explicit Rustls TLS backend. Use Serde for JSON and `tracing` for structured request and dependency logs.
- **Rationale**: The existing repository is a Rust HTTP service. Axum and Tokio fit its async request model; `reqwest` provides an async HTTP client. Rustls avoids a runtime OpenSSL dependency for HTTPS to MOEX ISS.
- **Alternatives considered**: Other Rust HTTP frameworks and a synchronous HTTP client. The feature needs concurrent inbound and outbound network I/O, and no existing codebase preference exists to justify a different choice.
- **References**: [Tokio runtime docs](https://docs.rs/tokio/latest/tokio/runtime/), [reqwest TLS docs](https://docs.rs/reqwest/latest/reqwest/tls/).

### Latency objective and response cache

- **Decision**: Cache only complete successful ticker responses in a bounded process-local async cache keyed by symbol. Use a configurable 60-second TTL and byte-weighted capacity (64 MiB default). Coalesce concurrent cache misses for the same symbol; publish an entry only after all history pages, board selection, and field mapping succeed. Do not serve stale data after an upstream failure.
- **Rationale**: MOEX history is paginated, and a full-history cold fetch requires multiple sequential upstream requests. Returning the completed cached response avoids that network work for repeat requests while preserving the full-history contract. A byte bound controls memory use, and coalescing avoids duplicate upstream work during concurrent misses. The user accepted a p95 target under normal conditions, so misses are measured and included in the overall target rather than excluded.
- **Alternatives considered**: Fetch all history on every request (cannot reliably meet the latency target for large histories); serve stale data while MOEX is failing (conflicts with the specified upstream dependency error behavior); add a distributed cache/database (unneeded deployment service and durable state for this feature).
- **Assumptions and risks**: Normal request traffic repeats symbols often enough for the cache hit ratio to support the 95th-percentile target. Cold starts, expired entries, low-reuse symbols, or memory evictions can exceed one second; validate hit and miss latency separately and report the complete workload percentile. The 60-second TTL is a planning default because no freshness window was specified. A process-local cache is not shared between replicas and resets on restart.
- **References**: The [MOEX ISS manual](https://www.moex.com/files/4be999zbzp80bx2bgmwayrtyx0) describes paged history retrieval. [Moka's async cache](https://docs.rs/moka/latest/moka/future/struct.Cache.html) supports async access, capacity limits, and single-flight `get_with`; its [builder](https://docs.rs/moka/latest/moka/future/struct.CacheBuilder.html) supports TTL and weighted capacity.

### Docker, distroless runtime, and architecture

- **Decision**: Use a multi-stage Docker build with a `BUILDPLATFORM` Debian Rust builder and a target-platform distroless runtime. Cross-compile for both `linux/amd64` and `linux/arm64` with the matching Rust target and Debian cross-compiler. Use `gcr.io/distroless/cc-debian13:nonroot`, an exec-form entrypoint, and port 8080.
- **Rationale**: The Rustls/AWS-LC executable dynamically needs `libgcc_s.so.1`, while the original Dockerfile copied only the ARM64 library path into a base image. The distroless `cc` runtime supplies the required C/C++ runtime for each target. Cross-compiling on `BUILDPLATFORM` avoids requiring QEMU/binfmt support to execute a target-architecture compiler and avoids architecture-specific runtime library copies.
- **Alternatives considered**: Run the target-architecture Rust builder under QEMU (requires registered emulation and is slower); keep `base-debian13` and copy an architecture-specific library via target-aware logic; fully static linking with extra linker configuration; use a general-purpose runtime with unnecessary tools.
- **References**: [Docker multi-platform builds](https://docs.docker.com/build/building/multi-platform/), [Docker multi-stage builds](https://docs.docker.com/get-started/docker-concepts/building-images/multi-stage-builds/), [Distroless README and supported architectures](https://github.com/GoogleContainerTools/distroless/blob/main/README.md), [Distroless cc image contents](https://github.com/GoogleContainerTools/distroless/blob/main/cc/README.md).

### MOEX security identity, date-specific main board and history retrieval

- **Decision**: Retrieve unauthenticated share history with its board identifier and named columns, retrieve MOEX listing history, and select each history row from the board designated primary for that trading date. Follow `history.cursor` until all unauthenticated rows are read, then sort by trading date ascending.
- **Rationale**: MOEX history is returned in pages and uses a columnar JSON structure. The official manual directs clients to use named blocks/columns and a `start` offset for complete results. The feature clarification requires date-specific primary-board selection, so a current-board-only lookup is insufficient.
- **Alternatives considered**: Hard-code a board such as TQBR; consume a single first page; select today's board for all historical dates; combine records from every board.
- **References**: [MOEX ISS Developer Manual v1.4](https://www.moex.com/files/4be999zbzp80bx2bgmwayrtyx0), [MOEX ISS interface overview](https://www.moex.com/a2920), [MOEX markets and boards definitions](https://ftp.moex.ru/pub/ClientsAPI/ASTS/docs/ASTS_Markets_and_Boards.pdf).
- **Assumption**: MOEX listing history exposes enough effective-date and main-board information to associate each daily history row with its primary board. If metadata cannot determine a row's primary board, treat the source data as unusable rather than silently including data from another board.

### Field mapping and `facevalue`

- **Decision**: Map `date` from `TRADEDATE`, and `close`, `high`, `low`, and `volume` from the correspondingly named fields in the history response. Keep the requested JSON property `facevalue`, and populate it from the current `LOTSIZE` field in the securities reference for the primary board selected on each record's trading date. Return `null` when the selected board's current LOTSIZE is unavailable.
- **Rationale**: The user explicitly chose current LOTSIZE per selected board to satisfy the expected SBER value `1`. MOEX history rows do not contain LOTSIZE or FACEVALUE, while the board-specific security reference exposes LOTSIZE. The API field name remains `facevalue` to preserve the requested response shape; its source is LOTSIZE, not MOEX FACEVALUE, and it is not a historical lot-size value.
- **Alternatives considered**: Use MOEX `FACEVALUE` (current SBER value is `3`, which does not satisfy the expected value); require a historical LOTSIZE series (not available in the history response and not required by the user's final decision); positional field mapping (fragile because ISS blocks can change).
- **Evidence**: The [TQBR SBER security reference](https://iss.moex.com/iss/engines/stock/markets/shares/boards/TQBR/securities/SBER.json?iss.meta=off) exposes board-specific `LOTSIZE=1` and `FACEVALUE=3`. The [SBER board metadata](https://iss.moex.com/iss/securities/SBER.json?iss.meta=off) identifies primary-board/date ranges. The [SBER history endpoint](https://iss.moex.com/iss/history/engines/stock/markets/shares/boards/TQBR/securities/SBER.json?from=2026-10-06&till=2026-10-06&iss.meta=off) supplies the daily market row without either reference field. The selected board is determined for each row's trading date.

### Upstream entitlement and authentication

- **Decision**: Make all MOEX requests without subscription credentials and consume only history returned to an unauthenticated client. Map an unsuccessful upstream response, including HTTP 403, to the documented dependency error; never convert denied access into an empty history.
- **Rationale**: The feature explicitly excludes subscription credentials. The MOEX manual documents subscriber-only historical data and unauthenticated delayed data; availability can depend on endpoint and data permissions, so the service must follow the response it receives.
- **Alternatives considered**: Send subscriber credentials (contradicts the feature decision); treat an access denial as no records (misleading); use a local historical data store (outside feature scope).
- **References**: [MOEX ISS Developer Manual, authentication and permissions](https://www.moex.com/files/4be999zbzp80bx2bgmwayrtyx0).

## Resolved Decisions

- **Language and runtime**: Current stable Rust, Tokio async runtime.
- **HTTP dependencies**: Axum, `reqwest` with Rustls, Serde, and `tracing`; exact crate versions are selected and locked when implementation begins.
- **Data storage**: No durable storage. A bounded process-local TTL cache stores complete successful responses to support the latency objective.
- **Testing**: Rust unit, integration, and API contract checks via Cargo; mock MOEX responses for deterministic error/pagination/field-mapping cases.
- **Container runtime**: Docker multi-stage build, distroless Debian 13 C/C++ runtime, non-root identity.
- **Container architecture**: Buildx cross-compilation for target `linux/amd64` and `linux/arm64`; amd64 is required, with arm64 retained for native development. Use a native amd64 CI runner for release runtime validation; multi-platform output is pushed to a registry.
- **External API pagination**: Fetch every page using the upstream cursor/row count until complete.
- **Latency target**: At least 95% of successful requests must complete end to end in under one second under normal operating conditions. Validate against a representative symbol distribution, including cold starts, cache hits, expiry, and large histories; report hit and miss timings and the aggregate percentile. Upstream request timeouts and failures remain covered separately.
