# Protobuf Contract Rules

Applies to every change under `proto/`. Keep source paths, packages, generated
Rust wrappers, handlers, and contract versions synchronized.

## Responsibility and rationale

`proto/` is the only source of Protobuf contracts. Separating contracts from
transport implementation lets clients generate bindings independently and makes
compatibility changes explicit. Do not place `.proto` files under `apps/` or
commit generated Rust; Tonic/Prost builds it into `OUT_DIR`.

## Paths, packages, and generated wrappers

Use `proto/<module>/v<major>/<schema>.proto`, where the module is a business
capability in `lower_snake_case`, the version is `v1`, `v2`, and so on, and the
schema name describes its scope. Do not use transport or layer names such as
`internal`, `grpc`, `api`, or `presentation` as root contract namespaces.

```text
Source:       proto/item/v1/item.proto
Package:      item.v1
Service FQN:  item.v1.ItemInternal
Wrapper:      apps/presentation/src/internal/pb/item/v1.rs
Rust module:  presentation::internal::pb::item::v1
```

The wrapper contains `tonic::include_proto!("item.v1");`. Its parents declare
`pub mod item;` and `pub mod v1;`. Never flatten multiple business modules or
versions into one Rust module. Keep the existing service name and RPC paths
stable for consumers; version them if a breaking change is required.

## Handler and REST schema organization

- Use `presentation/src/api/handlers/<module>/` for REST operations and
  `presentation/src/internal/handlers/<module>/` for gRPC operations.
- Implement the generated service trait in `internal/handlers/<module>.rs` and
  delegate each method to its operation file. Do not hand-code RPC paths.
- Use `api/schemas/<module>/` for REST request/response DTOs, with a
  `schemas/<module>.rs` declaration/re-export file. A shared error DTO may live
  directly under `schemas/`; health may live directly under `handlers/`.
- Keep namespace files limited to declarations/re-exports, except for the gRPC
  trait dispatcher. Do not use `mod.rs`.
- New business modules get their own handler/schema namespaces in both
  transports; do not place another module's operations under `item/`.

## Versioning and wire compatibility

The path version is the contract's major version, independent of the Cargo
package version. New contracts start at `v1`; patch forms such as `v1_1` are not
allowed. Keep old and new major versions together while consumers migrate.

Compatible additions usually stay in the existing version: new optional fields
with new numbers, new messages, or new RPCs. Evaluate enum additions and client
response handling before assuming compatibility.

Breaking changes require a new major version: removing or renumbering released
fields, changing their types/cardinality/meaning, removing or changing RPC
semantics, renaming services, or changing package names and full RPC paths.
Moving a source file without changing its package is an implementation change,
not by itself a wire-version change.

- Never reuse a field number or a removed field name.
- Keep released numbers stable. Deprecate a field while it remains present; when
  removing it in an appropriate contract evolution, reserve its number and name.
- Enums must have a zero-valued `*_UNSPECIFIED` member.
- Assume clients can send or receive unknown fields and future enum values.
- Remove old versions only after all consumers have migrated.

```proto
message ExampleReply {
  reserved 4, 7 to 9;
  reserved "legacy_title";
  string id = 1;
}
```

## Change workflow and definition of done

1. Identify the owning business module and choose the current or a new major
   version based on compatibility.
2. Keep the source path, `package`, and generated wrapper synchronized.
3. Update `apps/presentation/build.rs`, including compile inputs and
   `cargo:rerun-if-changed` declarations.
4. Update RPC handlers, DTO mapping, tests, and the gRPC usage documentation.
5. Verify wire compatibility, generated client/server behavior, and every
   workspace check listed in `apps/AGENTS.md`.

A compile pass alone does not prove that path/package/version alignment or
consumer compatibility is correct.
