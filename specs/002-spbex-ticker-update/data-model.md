# Data Model: SPBEX Ticker History and Quote API

## SPBEX Ticker

Identifies an instrument in the SPBEX chart feed.

| Field | Type | Rules |
|---|---|---|
| `symbol` | string | Trim outer whitespace, uppercase ASCII letters, then validate against `[A-Za-z0-9._-]+`; use the normalized value for upstream requests and collection identity. |

## Daily Market Record

One daily candle mapped from the SPBEX chart feed and serialized in a history array.

| API field | Type | Source and validation |
|---|---|---|
| `date` | ISO 8601 UTC string | `bar_unixtime`, interpreted as Unix seconds; must be positive. History is sorted oldest to newest. |
| `close` | number | Source `close`; finite and greater than zero. |
| `high` | number | Source `high`; finite and greater than zero. |
| `low` | number | Source `low`; finite and greater than zero. |
| `volume` | null | Source does not provide volume. |
| `facevalue` | number | Constant `1`, matching the referenced adapter. |

For every candle, `low <= close <= high` and `low <= high`. A response with duplicate timestamps or any unusable candle is rejected as an upstream-data failure rather than returned partially. A successful empty history array maps to `[]`.

## Latest Quote Record

A one-element array containing the latest record after a fresh chart-feed request. It uses the same six JSON keys. When the feed contains no candles across the full available range, `date`, `close`, `high`, `low`, and `facevalue` are null; `volume` remains null. A valid quote is never read from history cache.

## Durable History Collection

Validated daily records are retained in SQLite by normalized `(provider, symbol)` and date. Each history request fetches records after the latest retained date, merges them, and returns the complete ordered collection. Records do not expire automatically. Quotes are fetched for each request and are not persisted.
