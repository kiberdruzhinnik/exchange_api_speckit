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

Run six separate workload profiles, one for each v2 provider history and quote route. Each profile must use 10 concurrent clients and 10 total requests per second for its route. Record successful-response p95 and error rate for each route; at least 95% of successful responses in every profile must complete in under one second. Include history requests with persistent storage and quote requests with fixture-backed or available provider sources. Do not count failed responses as successful responses for the latency criterion, but record their rate separately.
