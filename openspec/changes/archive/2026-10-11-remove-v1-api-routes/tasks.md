# Tasks

## 1. Remove the v1 route surface

- [x] 1.1 Remove the six v1 route registrations and wrapper handlers; delete tests dedicated to the aliases and move retained provider behavior tests to v2, verifying with the affected API suites and `rg -n '/v1/' tests src` returning no matches.
- [x] 1.2 Update the canonical and provider feature API contracts to list v2 routes only, verifying their structure with `cargo test --test api_contract_schema`.

## 2. Remove the in-memory response cache

- [x] 2.1 Remove `HistoryCache`, its `AppState` wiring and cache-only builders and parameters, and the transient max-bytes and obsolete TTL configuration; make v2 history requests without a store fetch directly without retaining a response, verified by repeated-request coverage and `cargo test --lib --test api_contract`.

## 3. Remove legacy SQLite response-cache support

- [x] 3.1 Remove response-cache table creation, CRUD, importer, and migration-marker handling; drop existing legacy response-cache tables on store open while retaining durable collections, verified by an upgrade test and `cargo test --test cache_store`.
- [x] 3.2 Update admin cache clearing and its tests to target durable history collections only, verifying provider/symbol isolation and error behavior with `cargo test --test admin_cache`.

## 4. Remove stale route and cache references

- [x] 4.1 Update current API descriptions, feature specs, plans, data models, and quickstarts that promise v1 routes or legacy response-cache import; preserve archived change records and verify no stale claims remain in maintained `specs/` documentation with a targeted `rg` search.
