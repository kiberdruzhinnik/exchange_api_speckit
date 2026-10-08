# Quickstart

## Run locally

```sh
cargo run
```

The service listens on `0.0.0.0:8080`. Override `LISTEN_ADDR` to change the bind address, `MOEX_REQUEST_TIMEOUT_SECS` to change the per-request upstream timeout, `MOEX_HISTORY_CACHE_TTL_SECS` to change the response cache TTL (default 60 seconds), and `MOEX_HISTORY_CACHE_MAX_BYTES` to change the cache capacity (default 64 MiB). `MOEX_MAX_ISS_RESPONSE_BYTES` bounds each ISS response body (default 4 MiB), and `MOEX_MAX_HISTORY_BYTES` bounds accumulated history data (default 64 MiB). `MOEX_ISS_BASE_URL` can point to a compatible ISS endpoint in tests.

```sh
curl http://localhost:8080/v1/moex/SBER
```

The response is an ascending JSON array of daily records. `date` is the MOEX trading date at midnight UTC; market values may be `null`; `facevalue` is populated from the current board-specific `LOTSIZE` for the primary board selected on that trading date, or `null` when unavailable. The JSON property keeps the requested name `facevalue`. The service requests public ISS history without subscriber credentials, so its history is limited to data MOEX exposes without a subscription. A recognized ticker with no rows returns `[]`; malformed/unknown symbols return 400; ISS denial, timeout, malformed JSON, or unusable data return 502.

Known comparison case: `SBER` on `2026-10-06`; expected `facevalue` is `1`.

Latency acceptance: measure from request receipt through completion of the full response body. Under the representative normal operating workload, at least 95% of successful requests must complete in under one second. Include cold-cache requests, cache hits, expirations, and large histories in the run; report hit and miss timings separately without excluding misses from the aggregate percentile.

## Build and run in Docker

Build and run an amd64 image. The Docker builder cross-compiles for the requested target, so an ARM64 host does not need QEMU for the build:

```sh
docker buildx build --platform linux/amd64 --load -t exchange-api:amd64 .
docker run --rm --platform linux/amd64 -p 8080:8080 exchange-api:amd64
```

To publish one multi-architecture tag for amd64 deployment and arm64 development:

```sh
docker buildx build --platform linux/amd64,linux/arm64 --push -t registry.example/exchange-api:latest .
```

The image compiles in a Rust builder stage and runs as the distroless nonroot user. Use a native amd64 CI runner for release smoke validation when available; verify the image platforms and its health endpoint.
