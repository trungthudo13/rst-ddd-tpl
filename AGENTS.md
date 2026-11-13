# Project Instructions

Read [ARCHITECTURE.md](ARCHITECTURE.md) before changing code.

Follow the instructions for every affected scope:

- [src/AGENTS.md](src/AGENTS.md): root app, DI, routing, and host configuration.
- [apps/AGENTS.md](apps/AGENTS.md): library dependencies and layer boundaries.
- [apps/shared/AGENTS.md](apps/shared/AGENTS.md): infrastructure interfaces and shared errors/values.
- [apps/domain/AGENTS.md](apps/domain/AGENTS.md): entities, invariants, and business behavior.
- [apps/application/AGENTS.md](apps/application/AGENTS.md): commands, queries, and use cases.
- [apps/infrastructure/AGENTS.md](apps/infrastructure/AGENTS.md): technical adapters.
- [apps/presentation/AGENTS.md](apps/presentation/AGENTS.md): Axum, Tonic, and DTOs.
- [proto/AGENTS.md](proto/AGENTS.md): Protobuf contracts and versioning.

Changes spanning layers must comply with all relevant instructions. Do not move
responsibilities to another layer merely to avoid maintaining a boundary.

Keep documentation, comments, and client-facing messages in English. Keep this
template independent of any organization or product-specific workflow. Item is
the working example; add abstractions only when an actual use case requires them.
