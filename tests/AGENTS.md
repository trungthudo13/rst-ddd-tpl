# Host Integration Tests

Applies to root `tests/` and its child modules. Follow the root architecture and
[root app rules](../src/AGENTS.md).

- Keep host integration suites under `tests/host/`: route composition, Swagger
  gating, mounted REST workflows, and generated Tonic clients over the combined
  listener. Use mock repositories and isolated ephemeral listeners.
- Register these modules through `src/integration_tests.rs`, gated by
  `#[cfg(test)]` in `src/main.rs`. This lets tests exercise private host modules
  without widening the production API or creating a second app library.
- Keep fixtures in `tests/host/fixtures.rs` and mock ports in `repository.rs`.
  Do not call deployed services or depend on the process environment.
- Always stop test listeners and await server shutdown, including gRPC clients.
- Keep middleware, mapper, and handler contract unit tests in `src/tests/`;
  domain, application, and provider tests stay in their owning library.
- Keep Rust files free of comments, bind struct literal arguments to local
  variables, and use `<module>.rs` namespaces rather than `mod.rs`.
- Run `cargo test --workspace --locked --all-targets` and `cargo lint` from root.
