<!--
Sync Impact Report
Version change: 1.0.0 → 1.1.0
Modified principles: V. Practical Quality and Operability (expanded with final-build Semgrep and Trivy gates)
Added sections: none
Removed sections: none
Follow-up TODOs: confirm original ratification date
-->
# exchange_api Constitution

## Core Principles

### I. REST API Contracts First
Every externally consumed service interface MUST use documented REST conventions and an explicit,
versioned contract. Request and response schemas, status codes, error formats, and compatibility
expectations MUST be documented before consumers depend on them. Contract changes MUST identify
their impact on existing clients. This keeps independently developed clients and services
predictable.

### II. Documentation Is Part of Delivery
Each service and API MUST document its purpose, endpoints, configuration, dependencies, and
operational use. Documentation MUST be updated with behavior or contract changes and MUST provide
enough detail for another developer to use and maintain the service without relying on undocumented
tribal knowledge.

### III. Clear Microservice Boundaries
Each microservice MUST own a clearly defined business or technical capability and expose it through
its documented API. Services MUST avoid unnecessary shared runtime state and direct access to
another service's internal implementation or data store. Cross-service dependencies MUST be
explicit and documented so services can be developed and operated independently.

### IV. Compatibility and Change Management
Changes to APIs and service behavior MUST preserve existing consumers or include a documented
migration path. Breaking changes MUST be versioned and communicated before release. Shared schemas
and inter-service contracts MUST be reviewed for compatibility as part of the change.

### V. Practical Quality and Operability
Services MUST provide actionable error responses and enough logging to diagnose request failures
and inter-service problems. Implementation and operational complexity MUST be proportionate to the
service's purpose. Automated checks MUST cover important API behavior and service boundaries;
the checks selected for a change MUST be documented when they cannot run in the development
environment. A final build MUST run Semgrep source analysis and Trivy vulnerability scans against
the built deliverable. All Semgrep findings MUST be resolved before work is declared complete.
Every Trivy finding rated High or Critical for which a fix is available MUST be fixed before work
is declared complete. Scan results and any unavailable scan or fix MUST be documented with the
reason and affected artifact.

## REST API Standards

APIs MUST use consistent resource naming, HTTP methods, status codes, and error representations.
Inputs MUST be validated and failures MUST return documented client-actionable errors. Services
MUST document configuration and health or readiness behavior needed by their deployment
environment. API descriptions and examples MUST stay aligned with the implemented contract.

## Service Architecture and Development Workflow

New capabilities SHOULD be assigned to an existing service when they fit its ownership boundary;
new services MUST have a documented reason and explicit ownership. Service-to-service communication
MUST use documented interfaces. Changes MUST receive review for API compatibility, documentation,
service boundaries, and relevant automated checks before integration.

The application is intended for a private network. Project-specific security controls beyond
those required by the deployment platform are outside this constitution's quality requirements;
service specifications MUST state any security requirements that apply to a particular capability
or deployment.

## Governance

This constitution governs project specifications, plans, implementation, and reviews. A change to
these principles MUST be made by updating this document and recording its rationale in the change
that proposes it. The constitution version MUST follow semantic versioning: MAJOR for incompatible
principle changes or removals, MINOR for new or materially expanded principles or sections, and
PATCH for clarifications and non-semantic wording changes. The version and last-amended date MUST
be updated with each amendment.

Reviews MUST check proposed work against the principles and record material exceptions with their
rationale. Exceptions MUST identify the affected service or API and any follow-up needed to restore
compliance. Implementation plans and reviews MUST verify that API documentation, compatibility,
service ownership, and operational guidance are addressed where relevant.

**Version**: 1.1.0 | **Ratified**: TODO(RATIFICATION_DATE): confirm original adoption date | **Last Amended**: 2026-10-08
