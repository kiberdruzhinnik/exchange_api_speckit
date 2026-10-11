# Tasks

## 1. Configuration and cache deletion primitives

- [x] 1.1 Add optional `EXCHANGE_API_ADMIN_API_KEY` configuration and carry it into application state; verify config parsing covers unset, empty, and configured values.
- [x] 1.2 Add transactional provider/symbol deletion to `CacheStore`, including cascading history records and matching legacy cache rows; verify deletion is idempotent and leaves unrelated pairs intact.
- [x] 1.3 Add targeted invalidation to the in-memory history cache; verify a previously cached provider/symbol key becomes a miss while other keys remain cached.

## 2. Authenticated administrative endpoint

- [x] 2.1 Add the admin DELETE route with `X-API-Key` validation before target processing; verify missing, incorrect, unconfigured, and correct-key outcomes match the spec.
- [x] 2.2 Normalize the symbol with the selected provider, clear all cache layers, and map invalid provider, invalid symbol, and store errors to the documented envelopes; verify valid and invalid requests do not affect unrelated cache entries.
- [x] 2.3 Add API contract documentation for the endpoint, configuration, authentication, response, and errors; verify the OpenAPI contract includes the header and 204 response.
- [x] 2.4 Add route and store integration tests for cache deletion, no-entry idempotency, auth rejection, cache isolation, and repopulation through the existing history path; verify the targeted test suite passes.

## 3. Deployment and integration

- [x] 3.1 Document `EXCHANGE_API_ADMIN_API_KEY` in operator configuration and deployment examples; verify deployment documentation shows the key as a secret and existing configurations remain valid without it.
- [x] 3.2 Run the project’s required build and API contract checks, then verify the admin route is registered and unrelated v1/v2 routes remain available.
