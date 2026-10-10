# Data Model: MOEX Ticker History API

## Ticker

Identifies the MOEX instrument requested by the client and resolved against MOEX security and board metadata.

| Field | Type | Required | Source / rule |
|---|---|---:|---|
| `symbol` | string | yes | `SYMBOL` path value; passed as MOEX `SECID` after validation |
| `primary_board` | string | yes for each history date | MOEX listing history identifies the main board applicable on each trading date |
| `facevalue` | number or null | yes in response records | Current board-specific MOEX `LOTSIZE` from the primary board selected for the record date; null if unavailable. The API key remains `facevalue` as requested. |

## Daily market record

Represents one daily record from MOEX ISS history for the board that was primary on the record's trading date.

| Field | Type | Required | Source / rule |
|---|---|---:|---|
| `date` | string (`YYYY-MM-DDT00:00:00Z`) | yes | MOEX `TRADEDATE`; must parse as a trading date |
| `close` | number or null | yes | MOEX `CLOSE`; retain null if absent/null |
| `high` | number or null | yes | MOEX `HIGH`; retain null if absent/null |
| `low` | number or null | yes | MOEX `LOW`; retain null if absent/null |
| `volume` | number or null | yes | MOEX `VOLUME` (instrument units, not turnover `VALUE`); retain null if absent/null |
| `facevalue` | number or null | yes | Current `LOTSIZE` from the selected primary board's security reference; null if unavailable. This value is not historical per trading date. |

## Latest trade quote record

Represents the most recent executed MOEX trade returned by ISS for the requested ticker at the time of the API request. Each quote response contains exactly one record with the same six fields as a daily market record. The quote is fetched on every request and is never persisted or placed in the in-memory cache.

| Field | Type | Required | Source / rule |
|---|---|---:|---|
| `date` | string (`date-time`) or null | yes | `TRADEDATE` + `TRADETIME`, interpreted in Europe/Moscow and converted to UTC; null when a valid ISS response has no trade. |
| `close` | number or null | yes | MOEX ISS trade `PRICE`; null when a valid ISS response has no trade. |
| `high` | null | yes | Always null for a trade quote. |
| `low` | null | yes | Always null for a trade quote. |
| `volume` | number or null | yes | MOEX ISS trade `QUANTITY`; null when a valid ISS response has no trade. |
| `facevalue` | null | yes | Always null for a trade quote. |

## Durable history cache entry

Stores the complete public history response for one symbol so an unexpired result survives an application restart.

| Field | Type | Required | Source / rule |
|---|---|---:|---|
| `symbol` | string | yes | Unique MOEX ticker key. |
| `response_body` | bytes (JSON) | yes | Complete serialized ascending `Daily market record` array; never partial. |
| `fetched_at` | UTC timestamp | yes | Time the complete response was retrieved and mapped. |
| `expires_at` | UTC timestamp | yes | `fetched_at` plus configured history-cache TTL; expired rows are not served. |
| `response_bytes` | integer | yes | Serialized response byte count used for the configured aggregate payload capacity. |

## Relationships and behavior

- One ticker has zero or more daily market records.
- A ticker has at most one durable history cache entry; an entry contains the entire history response, is replaced atomically after a successful full refresh, and is valid only until its expiry time.
- One quote request returns a one-element array. A valid empty trades block produces one record whose six fields are null; an ISS error or malformed trade row is a dependency error and is not represented as an empty quote.
- A successful lookup with no history returns an empty collection.
- Each record is selected from the primary board applicable on that trading date, and the output is sorted by `date` ascending after all upstream pages are combined.
- A record with a missing or invalid trading date is unusable upstream data and causes a dependency error; nullable market values do not invalidate a record.
- If the ticker cannot be resolved to a primary-board assignment for its trading date, it is treated as unsupported/unusable upstream data and reported with the corresponding documented error.
- All MOEX requests are unauthenticated. Returned history is limited to records MOEX ISS makes available without subscriber credentials.
- MOEX row pagination is transport state, not a persisted domain entity; the service follows cursor metadata until all source records have been retrieved.
- The SQLite database and its WAL sidecars live together on a durable instance-local volume. No cache data is shared between service instances, and no quote data is stored in SQLite or the Moka hot cache.
