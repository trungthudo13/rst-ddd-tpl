# Domain Layer

Applies to `apps/domain/`.

## Responsibility and rationale

Own entities, aggregate roots, invariants, and domain behavior. Keeping these
rules independent of transport and storage ensures that HTTP, gRPC, and future
entry points enforce the same behavior. Infrastructure interfaces are centralized
in shared rather than declared in domain.

The demo entity is `entities/item.rs`. `DomainError` is a shared contract
re-exported by domain. `shared::ItemRepository` exchanges an associated entity
type, which its consumers bind to `Item`. Add value objects, domain services,
and domain events only when actual business behavior requires them.

## Boundaries

- Depend only on `shared` and domain-supporting value libraries such as `uuid`;
  never import application, infrastructure, or presentation.
- Do not import Axum, Tonic, HiQLite, transport serialization, or HTTP status types.
- Do not include SQL, connections, retries, provider configuration, network calls,
  commands/queries, DTOs, persistence rows, or use-case orchestration.
- Constructors and behavior must protect private entity fields. A failed state
  change must leave the entity valid and unchanged.
- Do not declare infrastructure interfaces or implementations in domain. Use
  shared contracts when a domain capability actually requires an external port.

## Logic placement

| Concern | Owner |
| --- | --- |
| Nonblank normalized Item title | Domain constructor and rename behavior |
| UUID and current time for the demo | Application |
| Persistence and provider retries | Infrastructure |
| Request parsing and status mapping | Presentation |
| Coordinating persistence and publication | Application |

## File conventions and verification

- Use `entities/<module>.rs` for entities and capability names for ports.
- Use `<module>.rs` and same-named folders; do not use `mod.rs`.
- Keep tests under `src/tests/`, grouped by domain module.
- Test invariants, rejected transitions, and identity preservation without a
  server or database. Do not return persistence models from the domain.
- Run the workspace checks in `apps/AGENTS.md`.
