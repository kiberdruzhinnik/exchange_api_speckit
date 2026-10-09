# Research: V2 History and Quote Routes

## Decision 1: Use provider-qualified v2 path parameters

- **Decision**: Add `GET /v2/history/{PROVIDER}/{SYMBOL}` and `GET /v2/quote/{PROVIDER}/{SYMBOL}`. Accept the exact provider values `moex`, `spbex`, and `cbr`.
- **Rationale**: The current service supports three sources and symbols can overlap. An explicit provider segment makes the selected data source visible in the URL and avoids ambiguous cross-provider lookup.
- **Alternatives considered**: Search every provider for a symbol (ambiguous and may call unintended upstreams); add a provider query parameter (less visible in the route and not the clarified path shape); expose v2 for one provider only (does not cover the requested shared route organization).

## Decision 2: Dispatch to existing provider implementations

- **Decision**: Add two v2 route templates and dispatch the provider path value to the existing MOEX, SPBEX, or CBR adapter. Reuse the current shared history and quote serving functions for normalization, fetching, persistence, response construction, and provider-specific errors. Keep all six v1 routes registered.
- **Rationale**: The route change does not alter provider behavior or the cache lifecycle. Reusing the current functions keeps the two API versions aligned and avoids duplicating provider logic.
- **Alternatives considered**: Duplicate provider calls and response assembly in separate v2 handlers (risks divergence); replace the v1 routes (breaks existing clients and conflicts with the user's compatibility choice).

## Decision 3: Return a distinct client error for unknown providers

- **Decision**: An unrecognized `{PROVIDER}` returns HTTP 400 with the shared error envelope and code `invalid_provider`. Keep malformed or unsupported symbols on the existing `invalid_symbol` outcome.
- **Rationale**: The provider segment is an explicit path input, and identifying the invalid field gives clients an actionable distinction from symbol validation. A client error matches the existing invalid-symbol status class.
- **Alternatives considered**: Treat an unknown provider as an invalid symbol (misidentifies the input); return 404 (can imply the resource is absent rather than the provider value being invalid); fall back to a default provider (risks returning data from the wrong source).

## Decision 4: Describe v2 in the shared OpenAPI contract

- **Decision**: Define the two path templates in the feature contract and add them to `specs/contracts/openapi.yaml` during implementation. Document the provider enum, provider-specific symbol validation, six-field response schemas, empty results, and existing error envelopes. Preserve the six documented v1 operations.
- **Rationale**: The service's public contract is maintained in one canonical OpenAPI document. V2 adds routes but reuses the current schemas and provider error outcomes.
- **Alternatives considered**: Keep v2 documentation only in a feature-local contract (leaves the canonical consumer contract incomplete); replace v1 OpenAPI paths (misstates the selected compatibility behavior).

## Decision 5: Apply the existing latency gate to the v2 operations

- **Decision**: Measure the six v2 provider-specific history and quote operations at 10 total requests per second with 10 concurrent clients; at least 95% of successful responses on each route must complete in under one second.
- **Rationale**: The user confirmed the existing service target applies to v2, so the new routes receive the same measurable performance gate as existing routes.
- **Alternatives considered**: Exempt v2 from the latency gate (would leave the new public contract without the established service performance target); define a different number (not requested and would weaken consistency without evidence).

## Decision 6: Leave storage and configuration unchanged

- **Decision**: Do not change persistence, history refresh scheduling, quote caching behavior, provider configuration, or health/readiness endpoints.
- **Rationale**: The specification changes URL organization and explicitly preserves the established data, cache, quote freshness, errors, and health behavior.
- **Alternatives considered**: Add version-specific storage or configuration (introduces behavior outside the requested route change and could split v1/v2 results).
