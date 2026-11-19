# Shared Package

Applies to `apps/shared/`.

## Responsibility and rationale

Own the project's provider-independent contracts: infrastructure interfaces,
shared errors, and all enum and named constant definitions. This is the common
contract layer that use cases consume and concrete adapters implement. Projects can replace
an adapter without changing its consumers or exposing a provider SDK to them.

`Error` in `src/errors.rs` is the single runtime service error enum. Ports,
domain behavior, and application use cases propagate it unchanged. Application
and domain re-export it; transport statuses remain presentation-owned.

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
- Preserve optional missing records as `None`; required missing records use
  `Error::NotFound`. Unexpected provider failures use `Error::Unexpected` with
  private diagnostics for safe boundary-level logging and response mapping.
- Define every enum and named constant here, including test and associated
  constants. Use generic payloads for layer-owned types. Keep entities, DTOs,
  workflows, and business validation in their owning layers.

## Mappers

- Define explicit `mappers/` modules for conversions between shared-owned
  contracts/values and foundational types, including both required directions.
- Keep schema ↔ command/query mappings in presentation, command/query ↔
  entity/business-model mappings in application, and domain conversions in domain.
  Shared mappers must never import those layers or become a cross-layer registry.
- Express domain values in ports through associated types or generic parameters;
  concrete conversion remains with the consuming or implementing layer. Test
  shared mappings for value preservation and conversion errors.

## Definition of done

Every infrastructure interface has an actual consumer and implementation (or a
test double). Enums and constants follow the shared-only ownership rule; other
shared values/helpers need real reuse. Contracts introduce no
outward dependency, support injection without provider types, and preserve tested
error semantics. Run the workspace checks in `apps/AGENTS.md`.
