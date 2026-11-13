# Root App

Applies to the root `app` package, `src/`, and its host dependencies in `Cargo.toml`.

## Responsibility and rationale

This package is the composition root and executable host. It may depend on all
four layer libraries and shared. Keeping deployment choices here makes the
presentation library reusable without selecting a database, routes, or dev tools.

- `main.rs` enters the Tokio runtime and invokes bootstrap.
- `bootstrap.rs` loads configuration, initializes tracing and infrastructure,
  selects implementations, and coordinates server/database lifecycle.
- `di.rs` constructs use cases with shared interfaces and returns injected handlers.
- `routes.rs` chooses HTTP endpoints and gRPC services, then applies host middleware.
- `openapi.rs` selects documented operations and builds the host's API document.
- `server.rs` owns listeners, HTTP/1 + HTTP/2 serving, signals, and connection draining.
- `tests/` verifies route composition, dev-tool gating, and HTTP/gRPC behavior.

## Boundaries

- Do not put business rules, persistence queries, DTO mapping, or provider adapter
  implementations in the host; delegate to the appropriate library.
- Select providers through shared ports before constructing application handlers.
- Mount Swagger UI and project-specific development libraries here.
- Presentation may define endpoint metadata but must not select exposed routes,
  start listeners, read deployment configuration, or initialize infrastructure.
- Keep `main.rs` small. Group bootstrap and registration code by module as needed.
- Keep source and documentation in English and follow `<module>.rs` conventions.

Run the workspace checks in `apps/AGENTS.md`. Plain `cargo test` selects root app;
use `--workspace` to include all layer libraries and verify the entire service.
