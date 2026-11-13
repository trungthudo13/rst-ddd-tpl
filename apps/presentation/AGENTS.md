# Presentation Layer

Applies to the `presentation` library in `apps/presentation/`.

## Responsibility and rationale

Define reusable inbound adapters: Axum module routers, HTTP and health-check
handlers, Tonic services, JSON/Protobuf DTOs, operation metadata, and transport
error mapping. Keeping endpoint definitions separate from the executable host
lets an application choose which modules, prefixes, services, middleware, and
development tools it exposes.

## Dependencies and boundaries

- The only direct workspace dependencies are `application` and `domain`.
- API frameworks, serialization, endpoint metadata, and transport logging may use
  libraries such as Axum, Tonic, Serde, Utoipa, UUID, and tracing.
- Do not depend directly on shared, infrastructure, HiQLite, or another provider.
  Consume application handler types and domain values through their public APIs.
- Do not define a binary, initialize a runtime/database, read deployment
  environment variables, start listeners, or register provider implementations.
- Define relative API routes here. Do not choose host prefixes, assemble the
  host router, install Swagger UI, or apply host middleware. These decisions
  belong in root `src/`.
- Do not put business invariants, use-case orchestration, SQL, provider clients,
  or persistence models in presentation.

## Module conventions

- `api.rs` exports HTTP handler, route, schema, and error namespaces.
- `api/routes.rs` defines module routers without host prefixes. Root app merges
  selected routers and mounts them under `/api`.
- `api/handlers/<module>/` contains callable operation handlers; a shared health
  handler may live in `api/handlers/health.rs`. Module routers mount these handlers.
- `api/schemas/<module>/` contains public request/response DTOs and schema metadata.
  Common error response schemas may live directly under `api/schemas/`.
- Handler annotations describe endpoint contracts for OpenAPI. Root app selects
  the documented operations and publishes the document and Swagger UI.
- `api/error.rs` owns HTTP errors and implements Axum `IntoResponse`. Consume
  `application::ApplicationError` and `domain::DomainError`; do not import shared.
- `state.rs` defines `ItemHandlers`, a cloneable bundle of injected use cases used
  by both HTTP and gRPC. It contains no providers or dependency factories.
- `internal/service.rs` defines the Tonic service and a server constructor. Root
  app chooses whether to mount it; presentation never starts the listener.
- `internal/handlers/<module>.rs` implements the generated service trait and
  dispatches into per-operation files under `internal/handlers/<module>/`.
- `internal/mapping.rs` and `internal/error.rs` own Protobuf and gRPC status mapping.
- `internal/pb/` wraps generated bindings using `tonic::include_proto!`.
- Use `<module>.rs` and same-named folders, never `mod.rs`. Namespace files contain
  declarations/re-exports except the generated trait dispatcher.

## Behavior and verification

- Map DTOs to commands/queries, invoke the injected use case, and map the result.
- Map validation to HTTP 400 / gRPC InvalidArgument, missing items to
  HTTP 404 / gRPC NotFound, and provider failures to generic HTTP 500 / gRPC Internal.
  Log diagnostics internally without exposing them to clients.
- Framework extraction errors may use Axum's default responses.
- Keep endpoint metadata and Protobuf contracts synchronized with handlers.
- `proto/` is the wire-contract source of truth. Build bindings into `OUT_DIR` and
  never commit generated Rust or hand-code RPC paths.
- Keep library tests under `src/tests/`. Root app tests verify mounted routes,
  Swagger gating, and generated gRPC clients over the combined listener.
- Run every workspace check listed in `apps/AGENTS.md`.
