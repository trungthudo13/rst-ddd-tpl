# Root Unit Tests

Applies to `src/tests/` and its child modules. Follow the
[root app rules](../AGENTS.md).

## Purpose and boundaries

- Use this folder for unit tests of middleware rules, mapper contracts, and
  handler request/response contracts.
- Use mock data and mock dependencies with controlled results. Keep tests
  deterministic and independent of environment variables, deployed services,
  databases, network listeners, and application bootstrap.
- A small in-memory Axum router is allowed to exercise a middleware or handler's
  real extractors and response serialization. Do not start the full service or
  test complete business workflows here.
- Keep domain invariants, application workflows, and infrastructure behavior in
  their owning packages' tests.

## Middleware scope

- Verify each middleware's documented rules using mock requests, headers,
  identities, provider results, and downstream handlers.
- Cover accepted and rejected requests, missing or malformed input, and dependency
  failures where relevant to that middleware.
- Assert whether the downstream handler runs, which request context is propagated,
  and the resulting status, headers, and public response.
- Verify authentication rules, request-scoped identity, safe panic recovery, and
  timeout behavior without real providers or wall-clock sleeps. Use controlled
  time when a middleware depends on time.

## Handler scope

- Test request parsing through the handler's actual extractors and request DTOs,
  using mock dependencies rather than a real database or provider.
- Check valid inputs and invalid types, malformed payloads, missing required
  fields, invalid nulls, and numeric bounds according to the request contract.
- For integer JSON fields, accept integer numbers and reject quoted numbers such
  as `"1"`, other strings, and decimal numbers such as `1.0` or `1.5`. Reject
  overflow and negative values for unsigned fields. Do not silently coerce types.
- Path and query values arrive as text; verify that they parse to the declared
  type and reject invalid values or values outside its bounds.
- Invalid requests must produce the defined rejection response without panicking
  or invoking the application use case.
- Given a controlled application result, verify that the response matches the
  declared response DTO/model, including field names, value types, optional/null
  values, nesting, status, and required headers.
- Cover defined error responses with safe public messages and no private provider
  diagnostics or internal persistence models in the payload.

## Organization and verification

- Group middleware tests under `middlewares/` and handler tests under
  `handlers/<module>/`, with one focused file per middleware or handler.
- Register test modules in `src/tests.rs` and same-named namespace files. Keep
  `#[cfg(test)] mod tests;` in `src/main.rs`; use `<module>.rs`, never `mod.rs`.
- Name tests after observable behavior and keep mock fixtures local to the scope
  that uses them. Keep integration and end-to-end suites outside this folder.
- Keep Rust files free of comments and bind struct literals before passing them
  as function arguments.
- Run `cargo fmt --all -- --check`, `cargo check --workspace --locked --all-targets`,
  `cargo test --workspace --locked --all-targets`, and `cargo lint` from the root.

Host route composition, Swagger gating, complete CRUD workflows, and generated
gRPC clients belong under `tests/host/`, registered by `src/integration_tests.rs`.
Follow [the integration-test rules](../../tests/AGENTS.md).
