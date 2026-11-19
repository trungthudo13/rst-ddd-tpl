# Internal Service Protocols

Applies to `src/internal.rs` and `src/internal/`. Follow the
[root app rules](../AGENTS.md).

## Purpose and scope

This namespace owns inbound Tonic gRPC definitions, operation handlers, protocol
mappers, and generated binding wrappers. Preserve the existing Item v1 demo.
Add or extend service contracts only for an explicitly requested internal caller,
receiver, and operation; do not automatically mirror new REST operations.

## Organization and boundaries

- `service.rs` owns `ItemGrpcService` and the generated server constructor.
  The host chooses mounting, prefixes, middleware, and listener lifecycle.
- `handlers/<module>.rs` implements the generated service trait and dispatches
  into operation files under `handlers/<module>/`.
- Wrap generated messages with `tonic::include_proto!` under
  `pb/<module>/v<major>.rs`. Root `build.rs` generates into `OUT_DIR`; never commit
  generated Rust or hand-code RPC paths. Follow the [Protobuf rules](../../proto/AGENTS.md).
- Keep conversions under capability-specific `internal/mappers/` modules,
  including required reverse conversions and explicit context for missing values.
  Handlers call these mappers, invoke injected use cases, and map responses.
- Mappers perform structural conversion and delegate business invariants to
  domain APIs. Consume application/domain contracts and injected state without
  importing shared, infrastructure, providers, or host setup.
- `mappers/errors.rs` exhaustively maps `application::Error` to `tonic::Status`:
  `InvalidId` and `EmptyTitle` use `InvalidArgument`, `NotFound` uses `NotFound`,
  and `Unexpected` uses `Internal` with `internal server error`. Preserve existing
  public messages, log private diagnostics, and never add a catch-all arm.
- Only protocol boundaries return `tonic::Status`; representation helpers return
  the shared error re-export. Do not define another service error enum.
- Keep `<module>.rs` namespaces limited to declarations/re-exports, except the
  generated service trait dispatcher. Keep Rust files free of comments.

Verify mapper round trips, syntax failures, safe exhaustive error mapping, and
Item v1 generated-client compatibility. Host listener tests live in `tests/host/`.
Run every workspace check, including `cargo lint`.
