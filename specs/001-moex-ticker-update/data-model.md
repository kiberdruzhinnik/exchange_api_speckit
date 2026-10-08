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

## Relationships and behavior

- One ticker has zero or more daily market records.
- A successful lookup with no history returns an empty collection.
- Each record is selected from the primary board applicable on that trading date, and the output is sorted by `date` ascending after all upstream pages are combined.
- A record with a missing or invalid trading date is unusable upstream data and causes a dependency error; nullable market values do not invalidate a record.
- If the ticker cannot be resolved to a primary-board assignment for its trading date, it is treated as unsupported/unusable upstream data and reported with the corresponding documented error.
- All MOEX requests are unauthenticated. Returned history is limited to records MOEX ISS makes available without subscriber credentials.
- MOEX row pagination is transport state, not a persisted domain entity; the service follows cursor metadata until all source records have been retrieved.
