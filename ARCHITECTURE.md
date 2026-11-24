# Architecture

**Rust with DDD template** is a general-purpose Rust service template using
Domain-Driven Design (DDD) and hexagonal architecture. Item CRUD demonstrates the
boundaries without prescribing the business model of a downstream service.

Read the [root app rules](src/AGENTS.md), [library rules](apps/AGENTS.md), and every
affected scoped `AGENTS.md` before changing code.

## Workspace and dependency direction

```text
.
├── Cargo.toml                    # Root app package and workspace
├── Cargo.lock                    # Reproducible dependency resolution
├── build.rs                      # Versioned Protobuf generation into OUT_DIR
├── tools/rslint/                 # Repository rules and workspace Clippy
├── proto/item/v1/item.proto      # Existing Item gRPC contract
├── src/                         # Executable, presentation, and composition root
│   ├── main.rs                  # Tokio entry point and source namespaces
│   ├── bootstrap.rs             # Configuration, providers, and lifecycle
│   ├── config.rs                # Application environment settings
│   ├── middlewares/             # Inbound tracing and timeout configuration
│   ├── di.rs                    # Use-case dependency registration
│   ├── routes.rs                # Host prefixes, middleware, and exposure
│   ├── openapi.rs               # Host-selected OpenAPI document
│   ├── server.rs                # Listener and graceful shutdown
│   ├── api/                     # HTTP handlers, schemas, mappers, and errors
│   ├── internal/                # Item gRPC handlers, mappers, and bindings
│   ├── state.rs                 # Injected ItemHandlers bundle
│   ├── tests/                   # Mapper, middleware, and handler unit tests
│   └── integration_tests.rs     # Registers private-host integration suites
├── tests/host/                  # Host routes, Swagger, CRUD, and gRPC clients
└── apps/
    ├── shared/                  # Infrastructure interfaces, errors, enums, constants
    ├── domain/                  # Entities, invariants, models, and domain mappers
    ├── application/             # Commands, queries, use cases, and input mappers
    └── infrastructure/          # Outbound adapters, rows, mappers, and migrations
```

The root `app` package owns the executable and presentation. The four packages
under `apps/` are libraries; there is no separate presentation package. The lint
tool is a development workspace member. Dependencies remain explicit and acyclic:

```text
application          -> domain -> shared
application          -> shared
infrastructure       -> domain/shared
root app             -> application + domain + infrastructure + shared
```

Root `app` is the default workspace member. `cargo run` builds `src/main.rs` as
binary `app`; use `--workspace` to check or test every library and the lint tool.

Domain depends only on shared and value libraries. Application consumes domain
behavior and shared infrastructure ports. Infrastructure implements those ports
and may depend on domain/shared, never application or presentation. Shared has
no layer dependencies. Associated types keep domain entities out of shared.

Presentation consumes application/domain contracts and injected state. Keep
shared, infrastructure, providers, and host configuration/registration imports
out of `src/api/`, `src/internal/`, and `src/state.rs`. These are source-module
boundaries within the root package, governed by the scoped instructions; Cargo
still enforces the dependency direction between the four libraries.

## Layer responsibilities

| Package or source boundary | Owns | Excludes |
| --- | --- | --- |
| [Root host modules](src/AGENTS.md) | Startup, configuration, DI, route/service selection, prefixes, OpenAPI assembly, Swagger UI, middleware, listeners | Business rules, persistence queries, endpoint conversion |
| [Shared](apps/shared/AGENTS.md) | Provider-independent ports, one service error enum, every enum and named constant | Entities, workflows, DTOs, adapters, framework types |
| [Domain](apps/domain/AGENTS.md) | Entities, identity, invariants, behavior, domain snapshots and their mappers | SQL, network calls, commands, DTOs, provider implementations |
| [Application](apps/application/AGENTS.md) | Command/query inputs, orchestration, shared port consumption, application mappers | HTTP/gRPC, persistence rows, provider SDKs, duplicated business rules |
| [Infrastructure](apps/infrastructure/AGENTS.md) | Connections, migrations, persistence/provider models, adapters and their mappers | Business policy, inbound handlers, use-case orchestration |
| [Root presentation modules](src/api/AGENTS.md) | Relative routes, HTTP/gRPC handlers, schemas, transport mapping, operation metadata | Providers, host prefixes, Swagger UI, environment configuration, listeners, business rules |

`main.rs` starts Tokio and delegates to bootstrap. HiQLite first loads `.env`;
bootstrap loads `AppConfig`, initializes tracing and the database, then registers
use cases in `di.rs`. `routes.rs` mounts relative REST routers under `/api`, the
existing Item gRPC service under `/internal`, and optional documentation routes.
`middlewares/request.rs` applies tracing and the five-second timeout.
`server.rs` serves HTTP/1 and HTTP/2, handles signals, and drains connections;
bootstrap shuts the database down when serving exits. Routers and servers receive
application settings rather than reading the process environment.

The template retains its existing Item v1 gRPC demonstration. Add further internal
services only for explicitly requested caller-to-receiver interactions; do not
automatically mirror new REST operations.

## Module scope

| Module/path | Responsibility and reason for separation |
| --- | --- |
| `shared/errors.rs` | One `Error` enum propagated through all layers; statuses remain transport-owned. |
| `shared/ports/item_repository.rs` | Provider-independent Item persistence contract with associated `Entity`. |
| `domain/entities/item.rs` | Private entity fields, title normalization, validated construction, and rename behavior. |
| `domain/models/item_snapshot.rs` | Domain-owned public projection used by actual representation conversions. |
| `domain/mappers/item.rs` | Entity ↔ snapshot conversion; reconstruction invokes validated entity construction. |
| `application/commands/` and `queries/` | One input and constructor-injected handler per operation, exposing `execute`. |
| `application/mappers/item.rs` | Create input ↔ entity conversion with explicit caller-provided identity and timestamp. |
| `infrastructure/adapters/hiqlite/database.rs` | Node startup, embedded migrations, cleanup, and lifecycle. |
| `infrastructure/adapters/hiqlite/item_repository.rs` | Bound SQL queries, missing-write semantics, and repository operations. |
| Infrastructure provider/capability `mappers/` directories | Persistence/provider ↔ domain conversions and provider error translation. |
| `infrastructure/models/item.rs` | Private relational `ItemRow`; storage representation stays separate from entities. |
| `infrastructure/migrations/` | Ordered, embedded SQL schema changes. |
| `src/api/routes.rs` | Relative health and Item paths/methods; the host adds `/api`. |
| `src/api/handlers/item/` | Extract input, invoke mappers and use cases, and return mapped responses. |
| `src/api/schemas/item/` | One JSON request or response DTO per file; declarations and metadata only. |
| `src/api/mappers/item.rs` | REST schemas ↔ command/query inputs and domain results. |
| `src/api/error.rs` | Transparent `ApiError` wrapper, exhaustive HTTP mapping, public responses, private logging. |
| `src/state.rs` | Cloneable injected use-case bundle shared by REST and gRPC, without provider registration. |
| `src/internal/service.rs` | Tonic service state and generated server construction. |
| `src/internal/handlers/item.rs` and `item/` | Generated trait dispatcher and per-operation RPC handlers. |
| `src/internal/mappers/` | Protobuf conversions and exhaustive gRPC status mapping. |
| `src/internal/pb/item/v1.rs` | Versioned wrapper around bindings generated by root `build.rs`. |
| `proto/<module>/v<major>/` | Versioned wire contracts that clients can share independently. |
| Root and library `src/tests/` | Unit tests at the owning boundary; no full host workflows in root unit tests. |
| `tests/host/` | Host integration suites, registered through `src/integration_tests.rs` to access private host modules. |

Additional infrastructure namespaces are extension points, not implemented
integrations. Add a provider only after a real use case requires its port:

| Namespace | Scope |
| --- | --- |
| `adapters/cache` | Connections, serialization, keys, and technical retries; the caller owns TTL policy. |
| `clients/internal_service` | Outbound internal contracts, credentials, deadlines, and transport errors. |
| `clients/third_party` | Vendor clients, SDKs, authentication, rate limits, and response models. |
| `messaging/broker` | Broker connection and lifecycle. |
| `messaging/producer` | Outbound publication, routing, headers, and retries; application owns message meaning. |
| `messaging/protocol` | Envelopes, encoding, and wire compatibility. |
| `notifications/email` and `sms` | Delivery mechanisms; application owns recipients, timing, and business content. |
| `interceptors` | Technical cross-cutting outbound integration behavior. |

Queue consumers and scheduled listeners are inbound adapters that invoke use
cases, so they belong in presentation. Infrastructure owns outbound mechanisms.

## Entity, persistence model, DTO, and use-case input

| Type | Example | Boundary |
| --- | --- | --- |
| Domain entity | `Item` | Protected identity, title, creation time, and invariants |
| Domain business model | `ItemSnapshot` | Domain projection with validated reconstruction, without a wire contract |
| Persistence model | `ItemRow` | Private database row mapped through infrastructure mappers |
| REST DTO | `CreateItemRequest`, `ItemView` | JSON contract only |
| Protobuf DTO | `pb::item::v1::ItemReply` | Generated wire contract only |
| Command/query | `UpdateItemCommand`, `GetItemQuery` | Application input independent of transport |
| Schema | Migration SQL or `.proto` | Stored or exchanged structure |

Item has a UUID, a trimmed nonblank title, and a creation time in Unix
milliseconds. Rename preserves identity and creation time. UUID/time allocation
belongs to application. Add domain services, events, or value objects when actual
business behavior needs them.

## Item request flow

```text
POST /api/items or /internal/item.v1.ItemInternal/CreateItem
  -> presentation request DTO
  -> presentation mapper
  -> CreateItemCommand
  -> CreateItemHandler::execute
  -> application mapper with allocated UUID/time
  -> validated domain construction
  -> ItemRepository::save
  -> HiqliteItemRepository and persistence mapper
  -> Item
  -> presentation response mapper
  -> ItemView or ItemReply
```

Read, update, and delete use their own handlers. Update loads an entity and invokes
rename before persistence. Missing updates/deletes remain not-found outcomes,
including deletion between read and write. Bulk deletion ignores absent IDs and
accepts an empty list. List is ordered by creation time and UUID. Updates use
last-write-wins; add optimistic revisions when a real workflow requires them.

## Ports, errors, and transport concerns

Infrastructure interfaces live in `shared/ports/`, grouped by capability. Shared
uses associated types or generic parameters for domain values and never imports
domain/application. The Item adapter binds `ItemRepository::Entity = domain::Item`.
The composition root selects a provider once and injects it:

```rust
let repository: Arc<dyn ItemRepository<Entity = Item>> =
    Arc::new(HiqliteItemRepository::new(database.client()));
let get_items = GetItemsHandler::new(Arc::clone(&repository));
```

Use cases support concrete repositories and trait objects through `Arc<R>` with
`?Sized` bounds. Shared owns contracts rather than provider factories or global
service locators. SQL, provider SDKs, and persistence models stay behind ports.

Every runtime service failure is a variant of `shared::Error` in
`apps/shared/src/errors.rs`; domain and application re-export it. All layers
propagate it without layer-specific error enums or wrappers. `EmptyTitle`,
`InvalidId`, `NotFound`, and `Unexpected` describe outcomes. Unexpected failures
preserve private diagnostics. Build tooling may use its own diagnostic contracts.

Presentation exhaustively maps each variant without a catch-all arm. HTTP uses a
transparent response newtype in `src/api/error.rs`; gRPC maps statuses under
`src/internal/mappers/errors.rs`. Validation uses HTTP 400 / `InvalidArgument`,
missing records use 404 / `NotFound`, and unexpected failures use 500 / `Internal`
with `internal server error`. Provider details are logged privately.
`cargo lint --errors-only` audits these contracts independently.

## File and mapper conventions

- Define every enum and named constant under `apps/shared/src/`, including test
  and associated constants. Use generic enum payloads for layer-owned types;
  keep entities, DTOs, workflows, and business validation in their owning layers.
- Keep `.rs` files free of line, block, and documentation comments. Bind struct
  literals to local variables before passing them as function arguments.
- Follow the [mapper ownership rules](apps/AGENTS.md#mappers). Put conversions in
  capability-specific physical `mappers/` directories; `mappers.rs` only declares
  modules and re-exports. Handlers and adapters delegate representation changes.
- Place mappers in a layer allowed to depend on both types. Implement required
  reverse conversions; require explicit context for missing data or return a
  defined error. Do not silently invent values or claim a lossy mapping is reversible.
- Presentation owns schemas ↔ commands/queries and response/domain conversions;
  application owns its inputs ↔ domain values; domain owns entity ↔ business-model
  projections; infrastructure owns persistence/provider ↔ domain conversions.
  Shared only maps its own contracts and foundational values without outward imports.
- Mappers only change representation. Domain owns business rules and validation;
  use cases own allocation, ports, and orchestration; adapters own provider I/O.
- Use `<module>.rs` and same-named child folders, never `mod.rs`. Namespace files
  declare modules and re-exports, except the generated service trait dispatcher.
- Keep each command/query input with its handler. Do not call handlers from other
  handlers or introduce a generic handler framework or catch-all helper modules.
- Group handlers, DTOs, entities, and contracts by business capability. Keep each
  request, response, query, and nested REST DTO in a separate file.
- Keep `.proto` source under `proto/<module>/v<major>/` with matching package and
  wrapper. Root `build.rs` generates into `OUT_DIR`. Preserve released field
  numbers, semantics, and RPC paths; see [Protobuf rules](proto/AGENTS.md).

## Extending and verifying the template

Start with business behavior in domain and required ports in shared. Add
application use cases, infrastructure adapters and mappers, and thin transport
handlers. Register dependencies in `src/di.rs`, select routes in `src/routes.rs`,
and update host OpenAPI. Add internal protocols only for an actual interaction.
Keep business invariants and provider behavior tested in their owning libraries.

Root `src/tests/` holds mapper, middleware, and handler contract unit tests with
controlled inputs/results. Root `tests/host/` holds mounted HTTP workflows,
Swagger gating, and generated gRPC clients. `src/integration_tests.rs` registers
those suites in the binary test harness without exposing private production
modules. Provider tests use isolated temporary HiQLite nodes and clean shutdown.

Run from the workspace root:

```bash
cargo fmt --all -- --check
cargo check --workspace --locked --all-targets
cargo test --workspace --locked --all-targets
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo lint
```

`cargo lint` reports Rust comments, misplaced representation mappings, inline
struct arguments, noncanonical error contracts, and enum/constant ownership,
then runs workspace Clippy. It never rewrites source. The lint tool's own tests
exercise rejected and accepted patterns, including mapper namespace placement.

## Running the Item demo

Install Rust through rustup and the native C/C++ build tools for your platform.
Cargo uses the pinned toolchain in `rust-toolchain.toml`, including rustfmt and
Clippy. The build script supplies protoc; no system Protobuf compiler is required.
HiQLite runs inside the service, so no separate database server is needed.

From the workspace root:

```bash
cp .env.tpl .env
cargo run --locked
```

HiQLite's configuration loader reads `.env` without overriding existing process
environment variables. The checked-in values describe a local single-node demo:

| Setting | Default in `.env.tpl` | Purpose |
| --- | --- | --- |
| `APP_ADDR` | `127.0.0.1:13000` | Combined REST and plaintext HTTP/2 gRPC listener |
| `DEBUG` | `true` | Enable `/docs/swagger/` and `/docs/openapi.json` HTTP routes |
| `RUST_LOG` | `info` | Tracing filter |
| `HQL_NODE_ID` | `1` | HiQLite node identity |
| `HQL_NODES` | `1 127.0.0.1:18100 127.0.0.1:18200` | Node identity, Raft address, and database API address |
| `HQL_LISTEN_ADDR_RAFT` / `HQL_LISTEN_ADDR_API` | `127.0.0.1` | Database listeners |
| `HQL_DATA_DIR` / `HQL_FILENAME_DB` | `.data` / `hiqlite.db` | Local persistence location |
| `HQL_SECRET_RAFT` / `HQL_SECRET_API` | Demo values | Database transport secrets; replace for a deployment |
| `HQL_LOG_SYNC` | `immediate_async` | HiQLite log synchronization mode |

The service embeds and applies migrations before accepting requests. `.env` and
`.data/` are ignored by Git; `Cargo.lock` is included for reproducible builds.
Stop with Ctrl-C to drain connections and shut down the database. `/api/health`
reports process liveness, not a database readiness check.

### REST example

```bash
curl -sS http://127.0.0.1:13000/api/health
curl -sS -X POST http://127.0.0.1:13000/api/items \
  -H 'content-type: application/json' -d '{"title":"First item"}'
```

Copy the returned `id` into `ITEM_ID`, then run:

```bash
ITEM_ID="paste-created-uuid-here"
curl -sS http://127.0.0.1:13000/api/items
curl -sS "http://127.0.0.1:13000/api/items/$ITEM_ID"
curl -sS -X PATCH "http://127.0.0.1:13000/api/items/$ITEM_ID" \
  -H 'content-type: application/json' -d '{"title":"Updated item"}'
curl -i -X DELETE "http://127.0.0.1:13000/api/items/$ITEM_ID"
```

Create returns HTTP 201; reads and updates return 200; deletion returns 204.
`POST /api/items/bulk-delete` accepts `{"ids":["<uuid>"]}` and returns 204, ignoring
missing IDs. Valid JSON with a blank title or malformed UUID returns 400; an
unknown UUID returns 404 for get/update/delete. Repository failures return a
generic 500 response. Axum handles malformed JSON and missing fields through its
standard extractor responses.

OpenAPI uses `/api` as its server URL and keeps operation paths relative to it.
With `DEBUG=true`, Swagger UI remains at `/docs/swagger/` and the document at
`/docs/openapi.json`.

### gRPC example

The host mounts generated gRPC services under `/internal`. Clients must include
that prefix before the generated service/method path. For a generated Tonic client,
generate bindings from `proto/item/v1/item.proto` in the client project, then
set its origin explicitly. The root service is a binary; clients share the
Protobuf contract rather than depending on a presentation crate:

```rust
mod item {
    tonic::include_proto!("item.v1");
}

use item::{
    CreateItemRequest,
    item_internal_client::ItemInternalClient,
};

let channel = tonic::transport::Endpoint::from_static("http://127.0.0.1:13000")
    .connect()
    .await?;
let origin = "http://127.0.0.1:13000/internal".parse()?;
let mut client = ItemInternalClient::with_origin(channel, origin);
let request = CreateItemRequest { title: "First gRPC item".into() };
let item = client
    .create_item(request)
    .await?
    .into_inner();
```

Setting only the connection endpoint path does not prepend it to generated RPC
requests. Direct grpcurl calls require a proxy that adds `/internal`; reflection
is not enabled.

`GetItem`, `UpdateItem`, `DeleteItem`, and `DeleteItems` use the same service and
listener. Their request fields are defined in `proto/item/v1/item.proto`.
Validation errors map to `InvalidArgument`, missing records to `NotFound`, and
repository failures to a generic `Internal` status. The integration tests exercise
the generated Tonic client over the combined HTTP/gRPC listener.

## Container image

The Dockerfile separates dependency compilation from application source:

1. `toolchain` installs native build dependencies and the Rust version pinned in
   `rust-toolchain.toml`, plus the pinned cargo-chef version.
2. `planner` extracts a cargo-chef recipe from the workspace manifests and lockfile.
3. `builder` cooks that recipe in release mode. Cargo downloads under
   `/usr/local/cargo` and compiled dependencies under `/build/target` remain in
   image layers, so downstream builds inherit them after pulling the image.
   These directories do not use BuildKit cache mounts. Cargo then removes the
   dummy app and build script, preserving dependency artifacts while ensuring
   real Protobuf generation runs even if the consumer matches the recipe's
   placeholder package version.
4. `build` copies the real source over the recipe skeleton and compiles the root
   `app` binary with the same release settings and symbols stripped.
5. `base` provides an internal Distroless C/C++ runtime stage, HiQLite defaults,
   writable storage at `/data`, UID/GID `65532:65532`, and the `/app/app` entrypoint.
   It can be built independently of the Rust build stages and contains no
   application binary.
6. `runtime` extends `base` with the compiled demo binary at `/app/app`.

The runtime contains no source, Cargo, compiler, or shell. The internal `base`
stage is not published as a separate image.

SQL migrations, generated Protobuf bindings, and Swagger assets are embedded at
build time. Local env files, database state, Git history, and host build artifacts
are excluded from the build context by `.dockerignore`.

Build with Docker and the Buildx/BuildKit component:

```bash
docker build -t app:local .
docker build --target builder -t app:builder .
```

The Dockerfile defines all HiQLite settings used by the demo as single-node
runtime defaults: node identity and addresses, listeners, `/data`, database
filename, transport secrets, and log synchronization. Override app settings and
HiQLite defaults through `--env` or your own `--env-file`, including
`HQL_SECRET_RAFT` and `HQL_SECRET_API` for the deployment.
The `CARGO_*` variables belong to the build stage only and are not inherited by
runtime.

```bash
docker run --rm --name app \
  --publish 13000:13000 \
  --mount type=volume,source=app-data,target=/data \
  --stop-timeout 30 \
  app:local
```

The base declares HiQLite TCP ports `18100` and `18200` through `EXPOSE`.
Downstream images declare their own application ports. This is image metadata;
publish ports with `--publish` as needed. REST and plaintext HTTP/2 gRPC in the
demo share the app listener on port `13000`.

In `HQL_NODES`, each entry contains the node ID, Raft address, and HiQLite API
address, in that order. Port `18100` carries Raft replication, leader elections,
and snapshot transfer between nodes. Port `18200` serves the HiQLite API used by
clients to communicate with the database leader. Both listeners bind to
`127.0.0.1` in the default single-node configuration, keeping them internal to
the container.

The image declares a volume at `/data`, writable by the runtime user. The named
volume in the example retains database and Raft state across container
replacement. Check liveness from the host with
`curl http://127.0.0.1:13000/api/health`.

Without an explicit mount, Docker creates an anonymous volume, which is removed
with the container when using `--rm`. `VOLUME ["${HQL_DATA_DIR}"]` expands to
`/data` at build time; changing `HQL_DATA_DIR` at runtime does not change this
declared volume. Mount writable storage at the configured path when overriding
the data directory.

The default build uses one Cargo job to limit memory consumption. On a larger
builder, override it with `docker build --build-arg CARGO_BUILD_JOBS=4 -t app:local .`.

### Publishing and reusing images

`.github/workflows/publish-image.yml` runs when a Git tag is pushed. It builds
the `builder` and `runtime` targets for `linux/amd64` and `linux/arm64`, publishing
to `ghcr.io/<owner>/<repository>` using the workflow's `GITHUB_TOKEN`. The repository
name follows the current GitHub repository, so forks publish to their own path.
No additional registry credentials are required.

For a tag such as `v1.0.1`, the published images are:

| Image tag | Contents and use |
| --- | --- |
| `v1.0.1` | Runnable Item demo, built from the tagged source. |
| `builder-v1.0.1` | Pinned Rust compiler, native build tools, cargo-chef, downloaded crates, and compiled template dependencies. |
| `latest` | Runnable image from the most recently published tag build. |
| `builder-latest` | Builder image from the most recently published tag build. |

Commit and push the workflow and Dockerfile changes before creating a release
tag. Use a new Docker-compatible release tag name for each release:

```bash
git tag v1.0.1
git push origin v1.0.1
```

The versioned builder tag is always `builder-<tag>` and the runnable tag is
`<tag>`. The two mutable aliases update on tag builds. New builds do not publish
`base-*` tags. After publishing the runnable image, the workflow removes existing
package versions tagged only with `base-*` names; versions with any other tag
are preserved. This uses the publishing repository's package admin access through
`GITHUB_TOKEN`; see [GitHub package deletion permissions](https://docs.github.com/en/packages/learn-github-packages/deleting-and-restoring-a-package#required-permissions-to-delete-or-restore-a-package).
Existing release tags do not automatically gain builder images when this
workflow changes.

For anonymous pulls, make the GHCR package public after its first publication.
For private packages, authenticate pulls and grant consuming repositories access
to the package. See [GHCR access and visibility](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry).

To compile a downstream service using the published dependencies, its Dockerfile
can be:

```dockerfile
FROM ghcr.io/<owner>/<repository>:builder-v1.0.1 AS build
WORKDIR /build
COPY . .
RUN cargo build --locked --release --package app --bin app \
    && install -Dm755 target/release/app /out/app

FROM ghcr.io/<owner>/<repository>:v1.0.1 AS runtime
ENV HQL_LOG_SYNC=immediate
COPY --from=build --chown=65532:65532 /out/app /app/app
EXPOSE 8080/tcp
```

Replace the owner, repository, version, package, and binary name with the intended
values. Use `builder-latest` and `latest` together to consume the latest tag build, or pin
versioned tags for repeatable builds. Exclude host `target/` directories,
credentials, local env files, and database state from the downstream build
context as this template's `.dockerignore` does.
The runnable image supplies `WORKDIR`, `USER`, HiQLite defaults, `VOLUME`,
`STOPSIGNAL`, and `ENTRYPOINT`; copying the binary to `/app/app` replaces the
template binary with the downstream service.
Configure the binary to listen on `0.0.0.0:8080` and publish it with
`--publish 8080:8080`. For hosts retaining this template's configuration, set
`APP_ADDR=0.0.0.0:8080`. `EXPOSE` does not change the application's listen port.

The binary must match the image's Linux architecture and be compatible with
Debian 13's glibc. Building with `rust:1-slim-trixie` matches this runtime. Any
additional dynamically linked libraries required by a service must also be
provided in its runtime image. HiQLite code and migrations remain embedded in
the service binary.

### Dependency reuse and build timing

The builder and consumer must use the same pinned Rust toolchain, native Linux
architecture, `/build` working directory, and release profile. The builder
inherits `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_RELEASE_DEBUG=0`, and
`CARGO_PROFILE_RELEASE_STRIP=symbols`; keep them in the consumer. Use the same
Cargo feature selections and compiler flags, including `RUSTFLAGS` and any
`.cargo/config.toml` settings. The template uses the native target without an
explicit `--target` and adds no `RUSTFLAGS`. Changing these settings invalidates
affected artifacts. Matching dependencies can be reused; new versions, new
dependencies, changed features, and workspace source still compile normally.
See the [cargo-chef cache limitations](https://github.com/LukeMathWalker/cargo-chef#benefits-vs-limitations).

Measure the same service source and build settings using two fresh, isolated
BuildKit builders: one Dockerfile starting from the compiler image and building
all dependencies, and one using `builder-<tag>` as above. Time the entire
`docker buildx build --progress=plain` invocation, starting before pulling base
images and ending after exporting the runtime image. Do not pre-pull the builder
or import external caches for the fresh comparison. Record the builder image's
compressed size and the layer download/extraction time in the plain build log.

Then change one application `.rs` file in a temporary copy of the service and
rebuild with each respective BuildKit builder. Preserve the first build's cache
and use `cargo build --locked --release --package app --bin app -v` in the timed
Dockerfiles to verify that unchanged registry dependencies are reported as
`Fresh` while the application recompiles. Record all four wall times and the
CPU architecture, job count, Rust version, image digests, and network conditions.
The download cost is part of the fresh builder measurement; a faster Cargo step
alone does not establish a faster fresh service build.
