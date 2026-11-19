# REST Handlers

Applies to this directory and child modules. Follow the [HTTP rules](../AGENTS.md).

- Extract requests, delegate conversions to `api/mappers/`, invoke the injected
  application handler, and return mapped results. Keep business behavior out.
- Every `#[utoipa::path(...)]` annotation declares an explicit tag matching its
  business namespace: operations under `handlers/item/` use `tag = "item"`;
  `handlers/health.rs` uses `tag = "health"`.
- Declare a nonempty English operation `description` of at most 200 characters.
  Response descriptions do not replace the operation description.
- Keep method, relative path, parameters, request body, responses, tag, and
  description consistent with the actual endpoint. The host owns prefixes,
  OpenAPI document assembly, and Swagger UI registration.
- Keep Rust files free of comments and bind struct arguments to local variables.
  Run the workspace checks, including `cargo lint`.
