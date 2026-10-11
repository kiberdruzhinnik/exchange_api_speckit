# Spec Delta

## MODIFIED Requirements

### Requirement: Clear one provider and symbol history cache
The system MUST expose an authenticated operation that removes durable history for exactly one supported provider and normalized symbol.

#### Scenario: Clear a cached collection
- **WHEN** an authorized caller sends `DELETE /v2/admin/cache/clear/{provider}/{symbol}` for a supported provider and symbol with cached history
- **THEN** the service removes that provider/symbol's durable history collection and records, and returns `204 No Content`

#### Scenario: Clear an uncached pair
- **WHEN** an authorized caller clears a valid provider/symbol pair that has no cached history
- **THEN** the service returns `204 No Content` and leaves all other history collections unchanged

#### Scenario: Subsequent history request repopulates cache
- **WHEN** a history request for a cleared provider/symbol completes successfully
- **THEN** the response is built using the existing provider refresh behavior and the resulting history is retained as usual
