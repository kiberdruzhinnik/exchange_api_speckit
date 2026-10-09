# Implementation Plan: V2 History and Quote Routes

**Branch**: `[005-v2-history-quote-api]` | **Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification at `specs/005-v2-history-quote-api/spec.md`

## Summary

Add provider-qualified v2 routes for history and current quotes while preserving every existing v1 route and response contract. Route the `{PROVIDER}` segment to the existing MOEX, SPBEX, or CBR provider, then reuse the current history and quote service functions. Document both versions in OpenAPI and apply the existing per-route performance target to the six v2 provider operations.

## Technical Context

**Language/Version**: Rust 2024 edition, minimum Rust 1.85

**Primary Dependencies**: Axum 0.8, Tokio 1.53, Reqwest 0.13, SQLx 0.8, Serde; no new dependencies

**Storage**: No storage changes. History continues to use the existing SQLite store and refresh behavior; quotes remain uncached.

**Testing**: Existing Rust unit, API integration, and OpenAPI contract checks run with `cargo test`; use fixture-backed requests to verify provider dispatch and v1 compatibility.

**Target Platform**: Existing Linux server/container service

**Project Type**: Existing single Rust REST microservice

**Performance Goals**: Measure each of the six v2 provider history and quote routes in a separate workload profile at 10 total requests per second with 10 concurrent clients; at least 95% of successful responses in each profile must complete in under one second. For MOEX history, measure and report the initial uncached paginated full fetch separately, then apply the target to subsequent history requests after the full history is populated in the cache.

**Constraints**: New paths are `/v2/history/{PROVIDER}/{SYMBOL}` and `/v2/quote/{PROVIDER}/{SYMBOL}`. `{PROVIDER}` is exactly one of `moex`, `spbex`, or `cbr`. Preserve v1 routes, provider-specific symbol validation, response mappings, history lifecycle, quote freshness, error behavior, and health/readiness behavior. Unknown providers return HTTP 400 with the shared error envelope and `invalid_provider` code. No configuration or persistence changes.

**Scale/Scope**: Six v2 provider operations (history and quote for three providers), alongside the six existing v1 operations. No new provider or data volume is introduced.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- REST API Contracts First: **PASS** — add versioned v2 paths and document parameters, response schemas, errors, and compatibility in the OpenAPI contract.
- Documentation Is Part of Delivery: **PASS** — document v2 paths, provider values, examples, and retained v1 routes; existing configuration and health/readiness behavior remain unchanged.
- Clear Microservice Boundaries: **PASS** — add route dispatch within the existing exchange API service and use its existing provider adapters.
- Compatibility and Change Management: **PASS** — v2 is additive and v1 routes remain available with their current behavior.
- Practical Quality and Operability: **PASS WITH RELEASE GATES** — add API and contract-schema coverage, verify the v2 latency criterion, and retain the required final Semgrep and Trivy scans for the built deliverable.
- Private-network security: **PASS** — v2 inherits the existing deployment boundary; this feature does not add authentication, authorization, or new external data access.

No constitution violations identified.

## Project Structure

### Documentation (this feature)

```text
specs/005-v2-history-quote-api/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
└── contracts/
    └── openapi-v2.yaml
```

The implementation also updates the canonical `specs/contracts/openapi.yaml` to include the v2 operations while preserving its existing v1 paths.

### Source Code (repository root)

```text
src/
└── http/
    ├── routes.rs       # v2 provider dispatch and route registration; v1 handlers stay available
    └── errors.rs       # invalid_provider client error mapping

tests/
├── api_contract.rs          # v2 history and provider dispatch behavior
├── api_quote.rs             # v2 quote dispatch and freshness behavior
├── api_errors.rs            # unknown provider and existing error envelopes
└── api_contract_schema.rs   # v1 and v2 OpenAPI route coverage

specs/contracts/openapi.yaml # canonical public API contract
```

**Structure Decision**: Extend the existing Axum router and shared service functions in `src/http/routes.rs`. V2 handlers select an existing provider from the path segment and then use the same normalization, history retrieval, quote fetching, and error mapping as the corresponding v1 handler. Keep route versioning in the existing service; add no service, runtime store, or provider module.

## Complexity Tracking

No constitution violations require a complexity exception.

## Phase 0: Outline & Research

Research decisions and alternatives are recorded in [research.md](research.md). The feature has no unresolved clarification markers.

## Phase 1: Design & Contracts

- [data-model.md](data-model.md) records the provider selector, provider-normalized symbol, and existing history and quote response entities; no stored data changes are required.
- [contracts/openapi-v2.yaml](contracts/openapi-v2.yaml) specifies the two provider-qualified v2 path templates, allowed provider values, response schemas, and error behavior. The implementation will merge these operations into the canonical OpenAPI document.
- [quickstart.md](quickstart.md) provides fixture-backed route checks, v1 compatibility checks, and the six-route v2 performance acceptance profile.

### Constitution Re-check

- V1 routes and response contracts remain intact while v2 is added: **PASS**.
- The OpenAPI contract covers provider selection, symbols, success and error responses for both versions: **PASS**.
- Route tests and contract-schema checks cover the new dispatch paths and compatibility: **PASS**.
- The one-second acceptance target is explicitly assigned to each v2 provider operation; MOEX's initial uncached full fetch is reported separately and its populated-cache profile must meet the target: **PASS**.
- No new storage, configuration, authentication, or service boundary is introduced: **PASS**.
