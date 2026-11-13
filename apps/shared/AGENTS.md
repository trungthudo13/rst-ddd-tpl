# Shared Package

Applies to `apps/shared/`.

## Responsibility and rationale

Own the project's provider-independent contracts: infrastructure interfaces,
shared errors, and genuinely shared constants/enums. This is the common contract
layer that use cases consume and concrete adapters implement. Projects can replace
an adapter without changing its consumers or exposing a provider SDK to them.

`DomainError`, `RepositoryError`, and `ApplicationError` are centralized here.
Their meanings still belong to their respective layers. Transport-specific
errors and statuses belong to presentation and are not shared contracts.

## Boundaries

- Depend on no other layer crate.
- Use only foundational libraries such as `uuid`, `thiserror`, and `async-trait`.
- Declare infrastructure interfaces in `src/ports/`, grouped by capability.
- Express required capabilities without implementation, SQL, transport, or SDK
  types. Keep provider selection and dependency registration in the composition root.
- Use associated types or generic parameters when a port exchanges domain values.
  For example, `ItemRepository::Entity` is bound to `domain::Item` by the adapter
  and its consumers, without shared depending on domain.
- Do not move entities or their business invariants into shared to satisfy a port.
- Do not import Axum, Tonic, HiQLite, provider SDKs, HTTP statuses, or DTOs.
- Do not add business workflows, adapter code, or unrelated convenience helpers.
- Keep provider detail out of client-facing error mappings.
- Presentation maps the errors exposed by application/domain into its own
  transport errors; it does not import shared directly.
- Map a missing repository record to an application not-found error; preserve
  backend failures as repository errors for boundary-level logging and mapping.

## Definition of done

Every infrastructure interface has an actual consumer and implementation (or a
test double). Shared values/helpers need real reuse. Contracts introduce no
outward dependency, support injection without provider types, and preserve tested
error semantics. Run the workspace checks in `apps/AGENTS.md`.
