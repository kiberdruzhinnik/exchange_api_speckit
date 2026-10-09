# CBR Currency Rates Data Model

## Supported Currency

The Bank of Russia's currently supported daily currency, identified to clients by its ISO character code (for example, `USD`).

| Field | Meaning | Validation |
|---|---|---|
| `symbol` | Public three-letter currency code | Trimmed, uppercased, then matched against the current CBR daily-currency directory |
| `cbr_id` | CBR internal identifier used to request history | Obtained from the official currency directory; not exposed in the API |

## Daily Currency Rate

A source row for one supported currency on its effective date.

| API field | Source mapping | Type and rules |
|---|---|---|
| `date` | CBR effective date | ISO 8601 UTC timestamp at midnight, such as `2013-03-25T00:00:00Z`; invalid or absent dates make the source response unusable |
| `close` | CBR value divided by nominal | Numeric RUB per one currency unit; `5` RUB per `100` units becomes `0.05`. Null if either required source value is absent. A computed value must be positive. |
| `high` | Not supplied by CBR | JSON `null` |
| `low` | Not supplied by CBR | JSON `null` |
| `volume` | Not supplied by CBR | JSON `null` |
| `facevalue` | CBR nominal quantity | Numeric nominal when supplied; otherwise JSON `null` |

History contains zero or more records ordered oldest to newest. A successful empty history is `[]`. Duplicate effective dates or unusable source rate data are treated as upstream failures rather than silently emitted.

## Latest Quote

The quote response is an array containing exactly one record using the same six fields as history. It represents the latest official rate fetched at request time. When the source has no rate row for the supported currency, all six fields are JSON `null`. On a non-publication day, return the latest published row and its actual effective date.

## Cache Identity and Lifecycle

History cache identity is `CBR:{NORMALIZED_SYMBOL}`. Successful, fully validated history is cached using the service's existing history TTL and durable SQLite store. Expired entries are refreshed before being returned. Quote rates are never cached. Supported-currency metadata may use a short-lived local cache, but it is distinct from quote data and must be refreshed so changes in the official directory are reflected.
