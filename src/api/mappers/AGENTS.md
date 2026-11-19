# REST Presentation Mappers

Applies to this directory and every child mapper module. Read the workspace,
presentation, and architecture instructions before changing code.

## Purpose and ownership

- Presentation mappers translate API schemas to and from application commands
  and queries, including path parameters and authenticated request context.
- Presentation mappers translate API schemas to and from domain business models
  and application results. Keep entity behavior and business validation in domain.
- Presentation mappers translate API schemas to and from domain entities through
  their public domain-owned projection and validated construction APIs. Entities
  belong to domain; persistence models belong to infrastructure. Infrastructure
  model to entity conversions stay in infrastructure. Do not depend on
  infrastructure or expose its private persistence models in presentation.
- Keep conversion implementations in capability-specific child mapper files.
  Namespace files contain declarations and re-exports only. Schema files declare
  wire contracts; handlers extract input, invoke mappers and use cases, and return
  mapped responses.

## Conversion rules

- A mapper changes representation. Do not perform use-case orchestration,
  repository access, provider calls, configuration loading, identity resolution,
  timestamp or identifier allocation, or business decisions here.
- Validate representation syntax such as UUIDs, JSON, and enum spellings. Delegate
  entity construction and business invariants to the owning domain APIs.
- Preserve identifiers, exact numeric strings, optional values, Unicode content,
  order, ownership, version pins, revisions, and audit metadata. Do not silently
  invent values, drop fields, or replace malformed input with defaults.
- Implement the required reverse conversions. Require explicit context for data
  the schema does not carry, or return the existing shared error when conversion
  is not representable. Do not claim a lossy conversion can be reversed.
- Use existing public application and domain contracts and established mapper
  functions. Do not add a generic mapper framework, wrapper helpers, or models
  without an actual conversion that needs them.

## Errors

- Return `application::Error` or `domain::Error`, which re-export `shared::Error`.
  Do not define another service error type or return string or dynamic errors.
- Construct an error variant directly where a representation failure is detected.
  A helper that only constructs an error variant is not a mapper.
- HTTP error handling belongs in `../error.rs`, outside mapper modules. That
  adapter owns `ApiError`, status codes, headers, public error responses, and
  private diagnostic logging. Keep exhaustive handling of the shared error enum
  with no catch-all arm and preserve the prescribed safe public messages.

## Verification

- Audit every `.rs` file in this directory recursively, including namespace files.
  Verify HTTP error behavior separately in `../error.rs`. Trace affected handlers,
  schema declarations, and callers before editing a conversion.
- Verify both required directions, explicit missing context, malformed inputs,
  optional values, metadata preservation, and safe exhaustive HTTP errors.
- Reuse existing mapper and endpoint tests. Add a focused regression test for
  non-trivial conversion changes; do not add tests that only mirror trivial code.
- Keep Rust files free of comments and bind struct literals before passing them
  as function arguments. Run `cargo fmt --all -- --check`,
  `cargo check --workspace --locked --all-targets`,
  `cargo test --workspace --locked --all-targets`, and `cargo lint` from the root.
