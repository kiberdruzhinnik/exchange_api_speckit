# Research: MOEX Benchmark and Currency Instrument Support

## Decisions

### Resolve each instrument to its MOEX market and board

- **Decision**: Use the instrument's ISS board metadata to identify engine, market, board, primary status, and effective dates. Request history and current data with the resolved market and board; choose the primary board for each history date.
- **Rationale**: IMOEX is on stock/index/SNDX, while GLDRUB_TOM is on currency/selt/CETS. An unscoped GLDRUB_TOM response includes CETS, CNGD, and LICU rows, so a symbol-only source path can mix distinct boards. MOEX's developer manual describes the engine/market/board/security path hierarchy and recommends listing metadata for board history.
- **Alternatives considered**: Keep the current hardcoded stock/shares paths (does not address either case); special-case each symbol (does not cover similar instruments); use an unscoped first result (can mix board records).
- **Evidence**: [MOEX ISS developer manual](https://www.moex.com/files/4be999zbzp80bx2bgmwayrtyx0); live metadata examples [IMOEX](https://iss.moex.com/iss/securities/IMOEX.json?iss.only=boards&iss.meta=off) and [GLDRUB_TOM](https://iss.moex.com/iss/securities/GLDRUB_TOM.json?iss.only=boards&iss.meta=off).

Direct unauthenticated ISS history requests for GLDRUB_TOM and IMOEX returned JSON history rows during planning on 2026-10-10. The developer manual also contains a general statement that historical ISS data other than indices may require a subscription. The design therefore uses only publicly returned data, adds no credentials, and keeps upstream rejection as a dependency error.

### Keep named-column mapping but allow market-specific fields

- **Decision**: Parse records by named columns and treat fields absent for a market as unavailable. Preserve the shared API shape; map absent volume or face value to `null` rather than substituting turnover or a different market field.
- **Rationale**: Index history includes `CLOSE`, `HIGH`, `LOW`, and `VOLUME`; GLDRUB_TOM history includes `CLOSE`, `HIGH`, `LOW`, `NUMTRADES`, and `WAPRICE` but no `VOLUME`. Both expose `TRADEDATE` and `BOARDID`. Index securities do not use the same LOTSIZE reference as tradable securities.
- **Alternatives considered**: Depend on fixed field positions (ISS schemas differ by market); map `NUMTRADES` or turnover into volume (changes meaning); fail when optional fields are absent (would reject valid currency/index records).
- **Evidence**: Live ISS history responses for [IMOEX](https://iss.moex.com/iss/history/engines/stock/markets/index/boards/SNDX/securities/IMOEX.json?limit=1&iss.meta=off) and [GLDRUB_TOM](https://iss.moex.com/iss/history/engines/currency/markets/selt/boards/CETS/securities/GLDRUB_TOM.json?limit=1&iss.meta=off). MOEX's [FX market guide](https://www.moex.com/files/4wcf2eg0gwpgk4220krw4xtan1).

### Select quote value according to instrument category

- **Decision**: Keep the existing trades-based quote for equities. For currency market data, use the selected board's latest `LAST`, `TIME`, and `QTY` fields; interpret `QTY` as lots and convert to instrument units using the board LOTSIZE when producing volume. For index quotes, use the latest non-null `CURRENTVALUE`, falling back to `LASTVALUE`, and represent the published timestamp/value in the existing one-record response shape; fields not supplied by the index quote are null.
- **Rationale**: The currency market response identifies the last trade price, time, and lot quantity in marketdata; the trades register route used by the current shares implementation does not return a currency trade response. Index marketdata uses index-level fields and has no executed trade price field named `LAST`. MOEX's published currency market-data field definition describes `QTY` as volume of the last trade in lots.
- **Alternatives considered**: Reuse the shares trades route for every category (not supported by index and fails for currency); return index `LASTVALUE` without considering `CURRENTVALUE` (can return the previous published value while the current value exists); cache quote data (violates the established fresh-quote contract).
- **Evidence**: Live current-data examples for [IMOEX](https://iss.moex.com/iss/engines/stock/markets/index/boards/SNDX/securities/IMOEX.json?iss.only=securities,marketdata&iss.meta=off) and [GLDRUB_TOM](https://iss.moex.com/iss/engines/currency/markets/selt/boards/CETS/securities/GLDRUB_TOM.json?iss.only=securities,marketdata&iss.meta=off); [MOEX currency-market QTY field definition](https://ftp.moex.com/pub/ClientsAPI/ASTS/Bridge_Interfaces/MarketData/Currency50_Info_English.htm).

### Recognize a MOEX instrument before importing matching legacy history

- **Decision**: Defer import of legacy MOEX rows from the global startup migration. On a MOEX history request, normalize and validate symbol syntax, then resolve MOEX metadata before importing any cache. An unrecognized instrument returns the standard invalid-symbol error; unsuccessful or unusable metadata returns the standard MOEX dependency error. Neither outcome migrates cache data. For a recognized instrument, import the exact legacy `MOEX:{SYMBOL}` response into the durable `moex` collection before reading the latest stored date or requesting history. Existing collections count as already migrated. Perform migration atomically and idempotently; leave the legacy row intact if parsing or storage fails and return the standard history-store error. Retain and reuse the resolved instrument context for the subsequent history request rather than issuing metadata twice. Continue the existing startup migration behavior for CBR and SPBEX rows.
- **Rationale**: This meets the clarification that only a recognized symbol's first history request migrates cached data, while ensuring the incremental fetch starts after migrated history. Reusing the resolved context avoids an extra ISS metadata round-trip. Per-symbol handling remains safe even if the store's one-time global schema migration marker already exists.
- **Alternatives considered**: Import after syntax validation but before metadata lookup (allows unsupported symbols to alter stored history); delete legacy data after failed conversion (data loss); fetch history before migration (same-date source results could be overwritten by late legacy records); keep eagerly importing MOEX rows at startup (does not meet per-symbol first-request timing).
- **Evidence**: Current flow in `src/http/routes.rs::serve_history`; existing legacy import in `src/cache_store.rs::migrate_legacy_rows`; date-keyed collection merge in `src/cache_store.rs::merge_records`; [history refresh specification](../004-history-cache-refresh/spec.md).

### Retain the public route and error contract

- **Decision**: Keep `/v1/moex/{SYMBOL}`, `/v1/moex/{SYMBOL}/quote`, `/v2/history/moex/{SYMBOL}`, and `/v2/quote/moex/{SYMBOL}` with their current response shapes and standard validation, upstream, and store errors.
- **Rationale**: The change expands supported MOEX categories but does not introduce a separate product surface. Provider-qualified routes already dispatch through the same MOEX provider.
- **Alternatives considered**: Add a market path parameter (breaks callers and duplicates source resolution); expose separate index/currency providers (fragments one MOEX capability and changes public routing).

## Operational Constraints

- MOEX ISS may provide delayed current market data to unauthenticated users; the index market is documented as available online. Do not add subscriber credentials. The history access policy must follow the project's existing public-data constraint and report upstream rejection as a dependency failure.
- Current data and history responses may change over time. Fixtures should pin observed response shapes; live values should be compared with an as-of date and must not be embedded as stable expected values.
- All user-controlled symbols remain validated before they are interpolated into ISS resource paths.
