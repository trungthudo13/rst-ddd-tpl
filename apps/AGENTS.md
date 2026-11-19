# Workspace Rules

These rules apply to every package in `apps/`. Read the instructions in each
affected package before changing code.

## Responsibility and rationale

The workspace separates business behavior, use-case orchestration, and technical
integration into Cargo libraries. Transport contracts belong to the root app's
presentation modules. This keeps each source of change local and allows the inner
layers to run without a server or database.
The root manifest defines the `app` package and the workspace. All packages under
`apps/` are libraries; root `src/` owns presentation, the executable, and composition.

## Dependency direction

```text
application          -> domain -> shared
                     -> shared
infrastructure       -> domain/shared
root app             -> application + domain + infrastructure + shared
```

- `shared` owns provider-independent infrastructure interfaces, every enum and
  named constant, and the shared error contract, `shared::Error`. It depends on no layer crate, transport framework, or
  database library.
- `domain` depends only on `shared` and domain-supporting value/port libraries.
- `application` depends only on `domain`, `shared`, and ordinary value libraries.
- `infrastructure` implements ports declared in `shared`, using domain entities
  where required. It never imports application or presentation.
- Keep infrastructure interfaces in `shared/ports/`. Use associated types or
  generic parameters for domain values so shared never imports domain/application.
- Select concrete implementations in the composition root and inject them into
  use cases. Shared contains the contracts, not provider factories or registrations.
- Root presentation modules consume application/domain contracts. They may use API
  frameworks and serialization libraries, but must not import shared,
  infrastructure, or host configuration/registration modules directly. This is a
  source-module boundary within the root package; see [root rules](../src/AGENTS.md).
- Root `app` owns DI registration, route selection, Swagger UI, middleware,
  listeners, and configuration. Presentation provides endpoint/service definitions.
- Keep business invariants out of adapters, transport handlers, and interceptors.
- Use `<module>.rs` and same-named child folders rather than `mod.rs`.
- Keep namespace files limited to declarations and re-exports. A generated gRPC
  service trait dispatcher may delegate to operation handlers in its namespace.

## Error contracts

- Define service errors only as variants of `shared::Error` in `shared/src/errors.rs`.
- All layers propagate the same enum; application and domain may re-export it.
- Map provider failures explicitly into a defined variant while preserving private
  diagnostics. Do not return raw strings, boxed errors, or dynamic error contracts.
- Presentation owns exhaustive HTTP status and public-message mapping. A new
  variant must require an explicit mapping; catch-all error arms are forbidden.
  Any future requested internal protocol mapper must also handle every variant.
- A transparent HTTP response wrapper and any future framework-required protocol
  signatures are transport adapters, not additional service error taxonomies.
- Keep the HTTP error adapter, its wrapper conversions, and response implementation
  together in root `src/api/error.rs`, outside schema mapper modules.

## Mappers

- Every layer must define explicit mappers for the representation conversions it
  owns. Keep them in capability-specific `mappers/` modules within the owning
  namespace; handlers, services, and adapters must delegate conversions to them.
- Place each mapper in a layer allowed to depend on both source and destination
  types. Mapping requirements never permit reversing a dependency direction.
- Implement both directions for required conversions. When the source lacks
  destination data, require explicit caller-supplied context or return an error;
  do not invent values or assume a lossy conversion can be reversed.
- Mappers only convert representations. Business rules and business validation
  belong in domain services. Preserve validated entity construction, value
  semantics, optional values, and error handling.
- Business models are domain-owned service input/output values, distinct from
  transport schemas and infrastructure persistence models.

| Owning layer | Required mappings |
| --- | --- |
| Presentation | REST/Protobuf schemas ↔ commands/queries; application results and domain business models ↔ response schemas |
| Application | Commands/queries ↔ domain entities and domain service business models |
| Domain | Entities ↔ business models and domain values |
| Infrastructure | Persistence/provider models ↔ domain entities and business models |
| Shared | Shared-owned contract values ↔ foundational values, without importing other layers |

## Definition of done

- Dependency direction is acyclic and responsibilities stay in their owning layer.
- Public behavior has tests in the package that owns it.
- Documentation and examples match the implemented behavior and use English.
- All checks pass from the workspace root:

```bash
cargo fmt --all -- --check
cargo check --workspace --locked --all-targets
cargo test --workspace --locked --all-targets
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo lint
```
