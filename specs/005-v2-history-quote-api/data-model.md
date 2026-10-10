# Data Model: V2 History and Quote Routes

This feature adds URL-level provider selection only. It does not add stored entities or require a database migration.

## Provider Selector

Identifies the existing data adapter selected by a v2 request.

| Value | Provider |
|---|---|
| `moex` | Moscow Exchange market data |
| `spbex` | Saint Petersburg Exchange market data |
| `cbr` | Bank of Russia currency data |

The path value is lowercase and must match one of the three supported values. Any other value is rejected as a client error with code `invalid_provider`.

## Provider-Symbol Request

Represents the route-selected provider and a caller-supplied symbol.

| Field | Meaning | Rules |
|---|---|---|
| provider | Source selection from the path | Required; exactly `moex`, `spbex`, or `cbr` |
| symbol | Ticker or currency code from the path | Required; normalized and validated by the selected existing provider |

The same symbol text may exist at more than one provider. Provider selection is determined only by the provider path segment; v2 does not search other providers.

## History Response

The v2 history route returns the selected provider's complete retained history using the existing array of six-field daily records: `date`, `close`, `high`, `low`, `volume`, and `facevalue`. Record ordering, null handling, mappings, and empty-history behavior remain provider-specific and unchanged. History uses the existing persistent collection and refresh lifecycle.

## Current Quote Response

The v2 quote route returns the selected provider's existing one-element array of six-field quote records. Quote meaning stays provider-specific: MOEX returns the latest executed trade, SPBEX returns its latest available daily candle, and CBR returns the latest official rate. Existing no-data behavior and freshness remain unchanged; quote results are not stored in the history collection.
