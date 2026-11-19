# Root App

Applies to the root `app` package, `src/`, `build.rs`, and its dependencies in
`Cargo.toml`.

## Responsibility and rationale

This package owns presentation, the composition root, and the executable host.
It may depend on application, domain, infrastructure, and shared. Keep transport
mapping and deployment choices in separate source modules even though they share
one Cargo package. The four packages under `apps/` remain reusable libraries.

- `main.rs` enters Tokio and delegates to bootstrap.
- `bootstrap.rs` loads configuration, initializes tracing and infrastructure,
  selects implementations, and coordinates server/database lifecycle.
- `config.rs` loads application-owned settings into `AppConfig`. Bootstrap loads
  them after HiQLite loads `.env`, then injects settings into routes and server.
  Routers and servers must not read the process environment.
- `middlewares.rs` declares middleware modules under `middlewares/`. Keep inbound
  middleware implementations and layer configuration there. `request.rs` owns
  tracing and the five-second request timeout.
- `di.rs` constructs use cases with shared interfaces and returns injected handlers.
- `routes.rs` merges API routers under `/api`, mounts the existing Item gRPC
  service under `/internal`, and selects documentation routes and middleware.
- `openapi.rs` selects operations and assembles the host's OpenAPI document.
- `server.rs` owns listeners, HTTP/1 + HTTP/2 serving, signals, and draining.
- `api.rs` and `api/` own HTTP routers, handlers, schemas, mappers, and errors.
  Follow the [HTTP rules](api/AGENTS.md).
- `internal.rs` and `internal/` own the Item gRPC adapter and generated wrappers.
  Follow the [internal protocol rules](internal/AGENTS.md).
- `state.rs` defines `ItemHandlers`, re-exported at the crate root. Keep providers
  and dependency factories out of injected handler bundles.
- Root `build.rs` generates versioned bindings from `proto/` into `OUT_DIR`.
- `tests/` contains mapper, middleware, and handler contract unit tests. Follow
  [the unit-test rules](tests/AGENTS.md). Host integration suites live outside
  this folder, under root `tests/host/`, registered by `integration_tests.rs`.

## Boundaries

- Keep business rules and persistence/provider implementations in their owning
  libraries. Keep DTO conversions under `api/mappers/` or `internal/mappers/`.
- Select providers through shared ports before constructing application handlers.
- The host selects prefixes, documentation exposure, middleware, and listeners;
  presentation defines relative endpoints and protocol metadata.
- Presentation consumes application/domain contracts and injected state. Do not
  import shared, infrastructure, providers, or host bootstrap, config, DI, route
  composition, middleware, OpenAPI assembly, or server modules in presentation.
  This source-module boundary replaces the separate presentation Cargo package.
- Keep every service failure in `shared::Error`; HTTP and gRPC exhaustively map
  the application/domain re-export. Keep private diagnostics out of responses.
- Register layer-owned mappers through the composition root only when needed.
  Follow the [mapper ownership rules](../apps/AGENTS.md#mappers).
- Keep `main.rs` small, Rust files free of comments, and use `<module>.rs` with
  same-named child folders rather than `mod.rs`.

Run the workspace checks in `apps/AGENTS.md`, including `cargo lint`. Plain
`cargo test` selects root app; `--workspace` includes all layer libraries and lint
tooling. Keep build-tool errors separate from runtime service error contracts.
