# Data Model: SPBEX Ticker History and Quote API

## SPBEX Ticker

Identifies an instrument in the SPBEX chart feed.

| Field    | Type   | Rules                                                                                                                                                   |
| -------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `symbol` | string | Trim outer whitespace, uppercase ASCII letters, then validate against `[A-Za-z0-9._-]+`; use the normalized value for upstream requests and cache keys. |

## Daily Market Record

One daily candle mapped from the SPBEX chart feed and serialized in a history
array.

| API field   | Type                | Source and validation                                                                              |
| ----------- | ------------------- | -------------------------------------------------------------------------------------------------- |
| `date`      | ISO 8601 UTC string | `bar_unixtime`, interpreted as Unix seconds; must be positive. History is sorted oldest to newest. |
| `close`     | number              | Source `close`; finite and greater than zero.                                                      |
| `high`      | number              | Source `high`; finite and greater than zero.                                                       |
| `low`       | number              | Source `low`; finite and greater than zero.                                                        |
| `volume`    | null                | Source does not provide volume.                                                                    |
| `facevalue` | number              | Constant `1`, matching the referenced adapter.                                                     |

For every candle, `low <= close <= high` and `low <= high`. A response with
duplicate timestamps or any unusable candle is rejected as an upstream-data
failure rather than returned partially. A successful empty history array maps to
`[]`.

## Latest Quote Record

A one-element array containing the latest record after a fresh chart-feed
request. It uses the same six JSON keys. When the feed contains no candles
across the full available range, `date`, `close`, `high`, `low`, and `facevalue`
are null; `volume` remains null. A valid quote is never read from history cache.

## History Cache Entry

Reuse the existing persistent cache representation: normalized
exchange-qualified key (`SPBEX:{symbol}`), complete serialized response body,
fetch time, expiry time, and response byte count. Only complete successful
responses are stored. Entries are eligible for reuse before expiry and survive
process restart for the remaining freshness period. Cache capacity and TTL use
the existing service settings.
