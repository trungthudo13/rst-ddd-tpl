# Project Instructions

Read [ARCHITECTURE.md](ARCHITECTURE.md) before changing code.

Do not add line, block, or documentation comments to `.rs` files. Run `cargo lint`
to check Rust comments, mapper placement, inline struct arguments, and workspace
Clippy. The repository checks report violations and never rewrite source files.

Define every enum and named constant in `apps/shared/src/`, including test and
associated constants. Other layers import or re-export them. Use generic enum
payloads when their concrete types belong to another layer; keep entities,
DTOs, workflows, and business validation in their owning layers. `cargo lint`
enforces this with `repo::shared_values`.

Define every service failure as a variant of `shared::Error` in
`apps/shared/src/errors.rs`. Propagate that enum across layers without defining
layer-specific error enums or returning string, boxed, or dynamic errors.
Presentation must explicitly map every variant to a status and public message;
never use a catch-all arm that hides new error variants. Unexpected failures
retain private diagnostics and return a safe generic response.
`cargo lint --errors-only` audits these contracts without running the other checks.

Follow the instructions for every affected scope:

- [src/AGENTS.md](src/AGENTS.md): root app, presentation, DI, routing, and host configuration.
- [src/api/AGENTS.md](src/api/AGENTS.md): Axum, DTOs, and HTTP error mapping.
- [src/internal/AGENTS.md](src/internal/AGENTS.md): internal protocol boundaries.
- [src/tests/AGENTS.md](src/tests/AGENTS.md): middleware and handler unit-test scope.
- [apps/AGENTS.md](apps/AGENTS.md): library dependencies and layer boundaries.
- [apps/shared/AGENTS.md](apps/shared/AGENTS.md): infrastructure interfaces and shared errors/values.
- [apps/domain/AGENTS.md](apps/domain/AGENTS.md): entities, invariants, and business behavior.
- [apps/application/AGENTS.md](apps/application/AGENTS.md): commands, queries, and use cases.
- [apps/infrastructure/AGENTS.md](apps/infrastructure/AGENTS.md): technical adapters.
- [tests/AGENTS.md](tests/AGENTS.md): host integration suites and generated gRPC clients.
- [proto/AGENTS.md](proto/AGENTS.md): Protobuf contracts and versioning.

Changes spanning layers must comply with all relevant instructions. Do not move
responsibilities to another layer merely to avoid maintaining a boundary.

Keep documentation, comments, and client-facing messages in English. Keep this
template independent of any organization or product-specific workflow. Item is
the working capability; add abstractions only when an actual use case requires them.
