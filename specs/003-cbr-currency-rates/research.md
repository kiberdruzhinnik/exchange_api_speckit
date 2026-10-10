# Research: CBR Currency Rates API

## Official source and rate mapping

- **Decision**: Use the Bank of Russia's public XML interface. Read currency IDs
    and ISO codes from `XML_valFull.asp`, then intersect them with the codes in
    the current `XML_daily.asp` publication to validate current support. Cache
    that supported directory for 60 seconds. Fetch the entire source-available
    date range for the internal ID, from its earliest available rate through the
    latest published date, from `XML_dynamic.asp`; fetch the latest official
    rate from `XML_daily.asp` on every quote request.
- **Rationale**: The official interface documents the currency directory, dated
    daily rates, and date-range history in one public service. The directory
    supports dynamic validation of any currently supported currency without a
    hard-coded USD/EUR/CNY list. The historical response provides a source
    nominal and value; dividing the value by the nominal yields RUB per one
    requested unit.
- **Mapping**: Set `date` from the source effective date at UTC midnight; when
    both source `Value` and `Nominal` are present, set `close = Value /
    Nominal`; set `facevalue = Nominal` when provided. For a source field that
    is absent, serialize the corresponding API value as null; if Value or
    Nominal is absent, `close` is null because the per-unit value cannot be
    calculated. CBR has no daily OHLC or traded-volume data, so `high`, `low`,
    and `volume` are null. Reject non-positive provided nominals, non-finite or
    non-positive computed rates, invalid dates, duplicate dates, and
    malformed/unusable documents as dependency failures.
- **Directory freshness**: Cache the supported-currency directory in process for
    exactly 60 seconds, then fetch it again. This avoids an extra directory
    lookup on every request while bounding how long removed or added currencies
    can remain stale. The directory cache never stores currency-rate data; every
    quote request fetches the latest rate anew. History responses use the
    persistent history cache specified by the feature.
- **Live endpoint finding**: `XML_val.asp?d=0` omits `ISO_Char_Code`, while
    `XML_valFull.asp` includes it and includes legacy entries. Intersecting the
    full directory with the current daily publication provides stable CBR IDs
    while excluding entries without a current daily rate. `XML_daily.asp?d=0`
    returns the current publication and is used only for directory support
    validation; quote requests make a distinct uncached request to
    `XML_daily.asp`.
- **Alternatives considered**: Hard-coding common currencies was rejected
    because the spec accepts any CBR-supported currency. A non-CBR market-data
    provider was rejected because the feature requires official Bank of Russia
    rates. The CBR daily-data SOAP service also provides one-unit rates
    (`VunitRate`) and full currency metadata, but the simple documented XML
    endpoints cover the needed directory, daily quote, and historical dynamic
    operations with a smaller integration surface.
- **References**: [Bank of Russia XML
    interface](https://www.cbr.ru/development/SXML/); [Bank of Russia daily data
    service](https://www.cbr.ru/development/DWS/).

## Shared provider interface and persistence

- **Decision**: Add `src/cbr/` parallel to the existing adapters and make all
    three implement one provider-neutral history/quote interface. The common
    application layer normalizes surrounding whitespace and case, invokes the
    selected adapter, applies shared history persistence and error mapping, and
    serializes the same record types. Rename `LatestTradeRecord` to
    `LatestQuoteRecord`; the neutral name allows MOEX, SPBEX, and CBR adapters
    to retain their respective trade, candle, and official-rate semantics. Keep
    CBR lookup/parsing/mapping within its adapter and key history as
    `CBR:{SYMBOL}`.
- **Rationale**: CBR's clarified requirements call for matching provider
    capabilities, normalized outputs, common errors, and configuration. Reusing
    common records and cache behavior reduces duplicated API handling; a neutral
    quote name avoids encoding MOEX semantics into the other providers.
    Exchange-qualified keys avoid collisions.
- **Alternatives considered**: Independent per-provider handlers and duplicated
    config were rejected because they allow contracts and runtime settings to
    drift. A separate CBR service or database would duplicate the existing
    service boundary and persistence behavior. Provider-specific source clients
    and mappings remain separate behind the common interface.
- **Configuration**: Use `EXCHANGE_API_*` for every application setting,
    including provider-specific URLs and response bounds (for example,
    `EXCHANGE_API_CBR_API_BASE_URL`). Keep provider-specific settings
    independently configurable by retaining the provider name after the common
    prefix. Do not support provider-only or unprefixed compatibility aliases.
    Use canonical names in the configuration reader, tests, measurement scripts,
    compose example, and documentation.

## Shared REST contract and compatibility

- **Decision**: Define all six MOEX, SPBEX, and CBR history/quote paths and the
    reusable history record, quote record, parameter, and error-envelope
    components in `specs/contracts/openapi.yaml`. Each feature-level OpenAPI
    file references its provider's path items from this canonical file. All
    providers trim and uppercase symbols and share HTTP 400/502/503 categories,
    error envelope, empty-history result, no-quote result, and property policy.
    Keep `invalid_symbol` and `history_store_unavailable` common, but preserve
    established upstream codes on current `/v1` routes: `moex_unavailable`,
    `spbex_unavailable`, and `cbr_unavailable`.
- **Rationale**: The clarification requires structural consistency across
    providers, while explicitly preserving upstream code strings prevents
    breaking clients already using version 1. A canonical contract is the source
    of truth and keeps provider-specific quote mapping in operation
    descriptions/examples.
- **Alternatives considered**: Replacing all provider codes with
    `upstream_unavailable` on existing `/v1` paths was rejected as a
    client-breaking change. Three independently maintained OpenAPI schemas were
    rejected because nullability and code examples had already drifted.

## Error and input handling

- **Decision**: The common layer trims and uppercases symbols; each provider
    validates its syntax and source support. Return HTTP 400 with
    `invalid_symbol`, HTTP 502 with its existing provider-specific upstream
    code, and HTTP 503 with `history_store_unavailable`. A valid empty history
    returns `[]`; an empty quote result returns one six-field all-null record.
    Use the same error envelope and status outcomes for all providers.
- **Rationale**: This distinguishes invalid client input, temporary source
    failures, valid absence of data, and persistence failure consistently while
    keeping existing upstream code values stable.
- **Alternatives considered**: Renaming upstream codes on existing `/v1` routes
    was rejected for compatibility. Returning errors for absent quote data
    contradicts the specified nullable response.

## Validation and delivery

- **Decision**: Use representative XML fixtures for currencies, nominal values
    (including non-unit nominal), full history, empty data, and malformed data;
    test client status/size bounds, mapping, supported-symbol validation, route
    behavior, cache persistence/restart, uncached quote requests, and the
    canonical OpenAPI contract. Benchmark the local release binary against each
    provider's production source using 10 concurrent clients and a fixed 10
    total requests/second arrival rate; measure history and quote profiles so
    the one-second p95 gate is verified on all six routes. Build the Docker
    image successfully for `linux/amd64` (no amd64 runtime test is required),
    preserve `linux/arm64` support, and validate the shared SIGINT shutdown
    behavior: stop accepting requests, drain in-flight work, and cancel
    remaining work at 30 seconds. Run the required final Semgrep and Trivy
    scans.
- **Rationale**: Fixtures make results stable despite changing official rates.
    Live measurements against all three production sources verify the hard
    service-level gate on every route using the compiled binary. SIGINT and
    amd64 acceptance reuse the existing service-wide behavior: bounded graceful
    drain/cancellation and successful amd64 image build only. Container
    validation follows the existing multi-architecture distroless workflow.
- **Acceptance handling**: Consume complete response bodies, report actual
    scheduled/issued/skipped requests, successes, errors, and
    successful-response p95 per route. Runs below 10 requests/second or with
    skipped arrivals are invalid. If any valid profile misses the one-second p95
    target, optimize and repeat; the feature remains incomplete until all valid
    profiles pass.
