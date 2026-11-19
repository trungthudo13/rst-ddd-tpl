# HTTP Presentation

Applies to `src/api.rs` and `src/api/` in the root `app` package. Follow the
[root app rules](../AGENTS.md).

## Responsibility and boundaries

Define Axum module routers, HTTP and health handlers, JSON schemas, operation
metadata, and HTTP error mapping. Keep provider selection, configuration,
Swagger UI, middleware configuration, and listeners in the root host modules.

- Consume application/domain contracts and injected `ItemHandlers`. Root package
  dependencies do not permit importing shared, infrastructure, providers, or
  host modules into presentation.
- Define relative routes; the host mounts selected module routers under `/api`.
- Keep business invariants, orchestration, SQL, persistence models, environment
  loading, provider registration, and listeners outside presentation.
- `api.rs` declares handler, route, schema, error, and mapper namespaces.
- Keep operations under `handlers/<module>/` and one DTO per file under
  `schemas/<module>/`. Follow the [handler rules](handlers/AGENTS.md) and
  [schema rules](schemas/AGENTS.md).
- Keep namespaces limited to declarations and re-exports; never use `mod.rs`.

## Mappers and errors

- Follow the [REST mapper rules](mappers/AGENTS.md). Keep representation
  conversions in capability-specific files under `api/mappers/`.
- Map schemas ↔ commands/queries and domain/application results ↔ response
  schemas. Implement required reverse conversions with explicit context for
  values absent from the wire contract, such as an update's path identifier.
- Handlers extract requests, call mappers, invoke one injected use case, and map
  responses. Mappers perform structural conversion and delegate entity
  construction and business validation to domain APIs.
- `api/error.rs` owns `ApiError`, its wrapper conversion, status codes, response
  implementation, safe public messages, and private logging. `ApiError` is a
  transparent newtype containing `application::Error`, the shared re-export.
- Conversion helpers return the same shared enum. Do not define local service
  error enums or return string, boxed, or dynamic service failures.
- Explicitly handle every error variant without wildcard or binding catch-all
  arms: `InvalidId` and `EmptyTitle` use 400, `NotFound` uses 404, and `Unexpected`
  uses 500 with `internal server error`. Preserve existing public messages and
  log unexpected diagnostics privately. Axum extraction errors may retain the
  framework's standard responses.

## Verification

Keep paths, methods, parameters, statuses, JSON field names, metadata, and host
OpenAPI selection synchronized. Root unit tests cover mapper round trips,
request parsing, response models, and safe error responses with mock data.
Host integration tests cover mounted routes and full workflows outside
`src/tests/`. Run all [workspace checks](../../apps/AGENTS.md).
