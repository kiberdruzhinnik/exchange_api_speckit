# Data Model: MOEX Benchmark and Currency Instrument Support

## MOEX Instrument Context

Resolves a normalized symbol to the source market metadata used for its history and quote requests.

| Field | Type | Required | Rule |
|---|---|---:|---|
| `symbol` | string | yes | Uppercase normalized MOEX symbol, such as `IMOEX` or `GLDRUB_TOM`. |
| `category` | enum-like value | yes | Index, currency, or existing traded instrument category determined from MOEX metadata. |
| `engine` | string | yes | MOEX ISS engine, such as `stock` or `currency`. |
| `market` | string | yes | MOEX ISS market, such as `index` or `selt`. |
| `board` | string | yes for a source row | Board identifier associated with the row, such as `SNDX` or `CETS`. |
| `primary_from` | date | yes | Valid first date on which this board is primary for history selection; inclusive. |
| `primary_through` | date or open-ended | yes | Valid last date on which this board is primary, inclusive, or open-ended while current. Must be on or after `primary_from` when present. |
| `lot_size` | number or null | no | Current board-specific LOTSIZE if applicable; absent for index data. |

The symbol alone is not enough to identify source data when multiple MOEX boards publish it. History uses the primary board that applies to each record date. Current quotes use the board that is primary at request time. Primary-board intervals may have gaps and may have an open end, but must not overlap; dates are inclusive. Invalid dates, reversed intervals, or overlaps make the metadata unusable. For a history request, the provider resolves this context after syntax validation and before cache migration. A recognized context is retained for the following history request. An unknown symbol is an invalid-symbol response; metadata retrieval or parsing failure is a MOEX dependency error.

For newly supported benchmark and currency symbols, validation accepts alphanumeric parts separated by single underscores after existing case and whitespace normalization. Leading, trailing, and consecutive underscores are invalid. This rule does not change validation for existing supported symbols.

## Daily Market Record

The existing normalized API record, unique within a provider collection by symbol and record date.

| Field | Type | Required | Source / rule |
|---|---|---:|---|
| `date` | ISO 8601 UTC timestamp | yes | Daily `TRADEDATE` at midnight UTC. |
| `close` | number or null | yes | Named market history close field. |
| `high` | number or null | yes | Named market history high field. |
| `low` | number or null | yes | Named market history low field. |
| `volume` | number or null | yes | Named market volume when available; do not substitute trade count or turnover. |
| `facevalue` | number or null | yes | Current LOTSIZE for the selected board where applicable; null when the market has no such value. |
| `source_board` | source metadata, not serialized | yes when multiple boards exist | Board selected for the record date; excluded from the six-field public schema. |

Index history maps `CLOSE`, `HIGH`, and `LOW`; use `VOLUME` only when present. GLDRUB_TOM history maps `CLOSE`, `HIGH`, and `LOW`; its source does not include daily `VOLUME`, so that field is null. The API continues to return exactly the six public fields.

## Current Value Record

One response record for a fresh quote request.

| Instrument category | `date` | `close` | `high` / `low` | `volume` | `facevalue` |
|---|---|---|---|---|---|
| Existing traded instruments | Existing trade date/time normalized to UTC | Latest trade price | null | Fetch-time `NUMTRADES` from current marketdata | null |
| Currency market | Primary-board update date combined with last-trade time, normalized to UTC | `LAST` | null | Fetch-time `NUMTRADES` | null |
| Index | Published update timestamp, normalized to UTC where present | `CURRENTVALUE`, otherwise `LASTVALUE` | null | null | null |

For traded instruments, `volume` is the current number of trades reported when the quote is fetched. It is not the quantity of the latest trade (`QTY`); LOTSIZE conversion is not used for quote volume. This quote-specific rule does not change daily-history volume mapping.

The quote request is built from the resolved engine, market, and current primary board. Index and currency quotes use the category's current-marketdata response; existing traded instruments retain the established trades-based request. The quote request is not stored in history. If the source has no valid current value, use the established one-record all-null result. Invalid or incomplete data remains an upstream error.

## Persistent History Collection and Legacy Cache

The existing collection identity remains `(provider, symbol)`, here `("moex", normalized_symbol)`. Records are unique by date and merged atomically under the established history-refresh rules.

| Legacy item | Type | Migration rule |
|---|---|---|
| Legacy cache key | string | Exact MOEX key `MOEX:{normalized_symbol}`. |
| Legacy response body | JSON array of six-field records | Parse fully before import; malformed data is a store migration failure, not an empty history. |
| Current history collection | collection plus dated rows | For a metadata-recognized symbol, create or merge before history retrieval and before the first request reads its latest date. Unknown symbols and metadata failures do not trigger migration. |
| Migration state | existing collection presence / migration bookkeeping | Import is idempotent; pre-existing collection data is already migrated. Keep the legacy row until durable merge succeeds. |

One MOEX instrument has zero or more daily market records and at most one quote result per request. Multiple boards may relate to one symbol, but each history date is attributed to the primary board selected for that date. On the first history request whose metadata resolves successfully, migration precedes collection lookup and history retrieval; the response includes migrated records combined with fetched records under the existing merge behavior. Failed source requests do not alter committed history; failed migration returns the documented history-store error and preserves the legacy source row.
