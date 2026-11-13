# Workspace Rules

These rules apply to every package in `apps/`. Read the instructions in each
affected package before changing code.

## Responsibility and rationale

The workspace separates business behavior, use-case orchestration, technical
integration, and transport contracts into Cargo packages. This keeps each source
of change local and allows the inner layers to run without a server or database.
The root manifest defines the `app` package and the workspace. All packages under
`apps/` are libraries; root `src/` owns the executable and composition root.

## Dependency direction

```text
presentation library -> application -> domain -> shared
                    -> domain
infrastructure       -> domain/shared
root app             -> presentation + application + domain + infrastructure + shared
```

- `shared` owns provider-independent infrastructure interfaces and shared error
  contracts. It depends on no layer crate, transport framework, or database library.
- `domain` depends only on `shared` and domain-supporting value/port libraries.
- `application` depends only on `domain`, `shared`, and ordinary value libraries.
- `infrastructure` implements ports declared in `shared`, using domain entities
  where required. It never imports application or presentation.
- Keep infrastructure interfaces in `shared/ports/`. Use associated types or
  generic parameters for domain values so shared never imports domain/application.
- Select concrete implementations in the composition root and inject them into
  use cases. Shared contains the contracts, not provider factories or registrations.
- Presentation depends only on application/domain among workspace libraries.
  It may use API frameworks and serialization libraries, but never shared or
  infrastructure directly. Cargo enforces this package boundary.
- Root `app` owns DI registration, route selection, Swagger UI, middleware,
  listeners, and configuration. Presentation provides endpoint/service definitions.
- Keep business invariants out of adapters, transport handlers, and interceptors.
- Use `<module>.rs` and same-named child folders rather than `mod.rs`.
- Keep namespace files limited to declarations and re-exports. A generated gRPC
  service trait dispatcher may delegate to operation handlers in its namespace.

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
```
