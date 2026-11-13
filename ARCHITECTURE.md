# Architecture

**Rust with DDD template** is a general-purpose Rust service template using
Domain-Driven Design (DDD) and hexagonal architecture. Item CRUD demonstrates the
boundaries; it does not prescribe the business model of services built from the
template.

Read the [root app rules](src/AGENTS.md), [library rules](apps/AGENTS.md), and the
relevant package `AGENTS.md` before changing code. Scoped rules take precedence
over this overview.

## Workspace and dependency direction

```text
.
├── Cargo.toml                    # Root app package and workspace
├── Cargo.lock                    # Reproducible dependency resolution
├── proto/item/v1/item.proto      # Versioned gRPC contract
├── src/                         # Executable app and composition root
│   ├── main.rs                  # Tokio entry point
│   ├── bootstrap.rs             # Configuration, providers, and lifecycle
│   ├── di.rs                    # Use-case dependency registration
│   ├── routes.rs                # Exposed HTTP routes, gRPC services, middleware
│   ├── openapi.rs               # API document selected by the host
│   ├── server.rs                # Listener and graceful shutdown
│   └── tests/                   # Host-level HTTP/gRPC integration tests
└── apps/
    ├── shared/                  # Infrastructure interfaces and shared contracts
    ├── domain/                  # Entities, invariants, and business behavior
    ├── application/             # Command/query use cases
    ├── infrastructure/          # Outbound adapters, rows, and SQL migrations
    └── presentation/            # Reusable HTTP/gRPC endpoint definitions
```

The root `app` package is an executable and also owns the workspace. It is not a
virtual workspace: Cargo needs a package with `src/main.rs` to build the binary.
All four layer packages and shared are libraries. Dependencies are explicit:

```text
presentation library -> application -> domain -> shared
                    -> domain
application          -> shared
infrastructure       -> domain/shared

root app -> presentation + application + domain + infrastructure + shared
```

The root `app` package is the default workspace member.
Cargo builds root `src/main.rs` as the `app` binary by convention, so an explicit
`default-run` setting is unnecessary. Run `cargo run` from the workspace root to
start the service. Use `--workspace` for checks and tests covering every layer.

The domain depends only on `shared` and libraries for domain values.
The application depends on the domain, shared contracts, and ordinary value
libraries. Shared is the project's common interface layer: it declares the
capabilities that infrastructure adapters implement. Infrastructure depends on
shared contracts and domain entities, never application or presentation. Shared
depends on no layer crate, so adapters remain replaceable without dependency
cycles.

Presentation depends directly on application and domain only among workspace
packages. It consumes application handler types and domain values; shared
contracts remain transitive implementation details of those APIs. It has no
database, provider, Swagger UI, or host dependency. Cargo enforces this boundary
because root app and presentation have separate manifests.

## Layer responsibilities and reasons for separation

| Package | Owns | Excludes | Why it is separate |
| --- | --- | --- | --- |
| [root app](src/AGENTS.md) | Runtime startup, configuration, DI, route/service selection, OpenAPI assembly, Swagger UI, middleware, listener lifecycle | Business rules, persistence queries, endpoint mapping | Each executable chooses its deployment and exposure policy while reusing the layer libraries. |
| [shared](apps/shared/AGENTS.md) | Infrastructure interfaces, shared errors, and genuinely shared constants/enums | Entities, workflows, DTOs, adapters, framework types | Use cases and providers agree on contracts without importing one another's implementations. |
| [domain](apps/domain/AGENTS.md) | Entities, identity, invariants, and behavior | SQL, network calls, commands, DTOs, infrastructure interface definitions | Business rules remain valid regardless of transport or storage. |
| [application](apps/application/AGENTS.md) | Commands, queries, use-case orchestration, and consumption of shared ports | HTTP/gRPC, persistence rows, provider SDKs | One workflow can serve multiple entry points and be tested with a port implementation in memory. |
| [infrastructure](apps/infrastructure/AGENTS.md) | Database connections, migrations, persistence models, outbound adapters | Business policy, inbound handlers, use-case orchestration | A provider can change without changing domain behavior or transport contracts. |
| [presentation](apps/presentation/AGENTS.md) | HTTP/gRPC endpoint definitions, health handlers, DTO mapping, operation metadata, status codes | Database/providers, host routing, Swagger UI, environment configuration, listeners, business rules | Endpoint definitions can be reused by hosts with different route and development-tool policies. |

Root `src/` is the composition root. `main.rs` starts Tokio and delegates to
bootstrap. Bootstrap loads configuration and starts infrastructure; `di.rs`
injects shared interfaces into use cases; `routes.rs` mounts selected endpoints,
gRPC services, Swagger UI, and middleware. `server.rs` serves that router and
drains connections. Bootstrap shuts down HiQLite when the server exits.

## Module scope inside each package

| Module/path | Responsibility | Reason for separation |
| --- | --- | --- |
| Root `src/main.rs` | Tokio entry point that calls bootstrap | The executable entry point stays small as modules are added. |
| Root `src/bootstrap.rs` | Configuration, tracing, provider selection, and lifecycle | Deployment choices stay outside reusable API definitions. |
| Root `src/di.rs` | Construct use cases and inject their dependencies | Registrations are easy to locate and replace without changing handlers. |
| Root `src/routes.rs` | Select HTTP routes and gRPC services, attach Swagger UI and middleware | The host explicitly controls what it exposes. |
| Root `src/openapi.rs` | Assemble metadata for the host's selected operations | API publication and documentation policy stay with route selection. |
| Root `src/server.rs` | HTTP/1 and HTTP/2 listener, signals, and connection draining | Network lifecycle stays independent of endpoint definitions. |
| `shared/errors.rs` | Domain, repository, and application error contracts | Error categories stay independent of Axum, Tonic, and HiQLite. Each layer owns the meaning of its failures. |
| `shared/ports/` | Provider-independent interfaces for infrastructure capabilities; the demo defines `ItemRepository` | A project can choose adapters without changing use cases. Associated types keep domain models out of shared dependencies. |
| `domain/entities/item.rs` | Item identity, title normalization, and state changes | Only entity behavior can change protected state, so all callers observe the same invariants. |
| `application/commands/` | One state-changing input and handler per operation | Each workflow has an explicit entry point and dependencies. |
| `application/queries/` | One read-only input and handler per operation | Reads remain distinct from state changes without requiring separate storage systems. |
| `infrastructure/adapters/hiqlite/database.rs` | Node startup, embedded migrations, database lifecycle | Connection management is independent of entity-specific SQL. |
| `infrastructure/adapters/hiqlite/item_repository.rs` | SQL implementation of `ItemRepository` | Persistence details stay behind the port. |
| `infrastructure/models/item.rs` | Private `ItemRow` database representation | Database column layout does not become the domain or API contract. |
| `infrastructure/migrations/` | Embedded, ordered SQL schema changes | Schema evolution is versioned alongside its adapter. |
| `presentation/api.rs` | Export HTTP handler, schema, and error namespaces | Root app can choose individual endpoint definitions. |
| `presentation/api/handlers/item/` | Parse requests, invoke handlers, map responses | Transport validation and application orchestration have distinct owners. |
| `presentation/api/schemas/item/` | JSON request and response DTOs | Public payloads can change without changing entities. |
| `presentation/api/error.rs` | HTTP errors and mapping from application/domain failures | Transport-specific categories and statuses stay outside shared contracts. |
| `presentation/state.rs` | Public `ItemHandlers` bundle for injected HTTP/gRPC use cases | Transport adapters receive ready-to-use handlers without knowing providers or registration logic. |
| `presentation/internal/service.rs` | Tonic service state and generated server construction | gRPC wiring is separate from individual RPCs. |
| `presentation/internal/handlers/item.rs` and `item/` | Dispatch generated RPC methods and map each operation | Generated service signatures do not spread into application code. |
| `presentation/internal/mapping.rs` and `error.rs` | Protobuf output and gRPC status mapping | Protocol-specific representations remain local to Tonic. |
| `presentation/internal/pb/` | Wrappers around generated, versioned bindings | Generated wire types stay separate from handwritten logic. |
| `proto/<module>/v<major>/` | Source Protobuf contracts | Clients share an explicit versioned contract without depending on the Rust implementation. |
| Root and each library's `src/tests/` | Tests at the owning boundary | The host tests route composition; library tests isolate their behavior. |

The following infrastructure namespaces are documented extension points, not
implemented features. Add a provider only after defining a use case and its port:

| Namespace | Scope and reason for separation |
| --- | --- |
| `adapters/cache` | Cache connectivity, serialization, and keys; the calling layer owns TTL policy. Keeps cache mechanics out of business rules. |
| `clients/internal_service` | Calls to other services under your control; isolates service contracts, credentials, timeouts, and transport errors. |
| `clients/third_party` | External vendor clients; isolates vendor SDKs, authentication, rate limits, and response models. |
| `messaging/broker` | Broker connections and lifecycle; reusable independently of a particular publication. |
| `messaging/producer` | Outbound publication, routing, headers, and retries; the application owns message meaning and workflow. |
| `messaging/protocol` | Transport envelopes and encoding; wire compatibility stays separate from domain events. |
| `notifications/email` and `notifications/sms` | Provider-specific delivery; recipient, timing, and business content belong to the use case. |
| `interceptors` | Technical cross-cutting behavior for outbound integrations; domain decisions never belong in middleware. Inbound middleware stays in presentation. |

A queue consumer or scheduled listener is an inbound adapter: place it in
presentation so it can invoke application use cases. Infrastructure owns outbound
connections and producers, not listeners that drive the application.

## Entity, persistence model, DTO, and use-case input

These types are intentionally distinct:

| Type | Example | Boundary |
| --- | --- | --- |
| Domain entity | `Item` | Protected identity, title, and creation time; no serialization contract |
| Persistence model | `ItemRow` | Private database row, mapped to a valid entity by the repository |
| REST DTO | `CreateItemRequest`, `ItemView` | JSON contract only |
| Protobuf DTO | `pb::item::v1::ItemReply` | Generated wire contract only |
| Command/query | `UpdateItemCommand`, `GetItemQuery` | Application input, independent of either transport |
| Schema | Migration SQL or `.proto` | Stored or exchanged data structure; not a runtime layer |

The demo Item has a UUID, a trimmed nonblank title, and a creation timestamp in
Unix milliseconds. Changing its title preserves identity and creation time.
UUID and timestamp generation are application concerns here. Add value objects,
domain services, or domain events when actual business behavior requires them.

## Item request flow

```text
POST /items or item.v1.ItemInternal/CreateItem
  -> presentation request DTO
  -> CreateItemCommand
  -> CreateItemHandler::execute
  -> Item::new (enforce invariants)
  -> ItemRepository::save
  -> HiqliteItemRepository (SQL and ItemRow mapping)
  -> Item
  -> ItemView or ItemReply
```

Read, update, and delete follow the same direction through their own handlers.
Updates load the entity and invoke its behavior before saving. Repository update
and delete operations report missing records, including deletion between a read
and a write. Bulk deletion ignores missing IDs and accepts an empty list. List
results are ordered by creation time and UUID in the HiQLite adapter. The demo
uses last-write-wins title updates; add version checks when a workflow needs
optimistic concurrency control. Listing is unpaginated for this small example.

## Ports, errors, and transport concerns

Infrastructure interfaces (ports) are centralized in `shared/ports/`, grouped by
capability. Define a port when a real use case requires a repository, cache,
message publisher, notification sender, service client, or clock. Infrastructure
implements the contract using the provider selected for the project. Domain and
application retain ownership of business behavior and workflow policy.

Shared must not depend on domain or application. When a port exchanges domain
values, express their type through an associated type or generic parameter.
`ItemRepository` declares `type Entity: Send + Sync`; the HiQLite adapter binds
`type Entity = domain::Item`, and Item use cases require that same association.
This keeps the entity and its invariants in domain without a dependency cycle.

The composition root selects the implementation once and injects it through the
shared interface:

```rust
let repository: Arc<dyn ItemRepository<Entity = Item>> =
    Arc::new(HiqliteItemRepository::new(database.client()));
let get_items = GetItemsHandler::new(Arc::clone(&repository));
```

Use cases accept constructor-injected dependencies and support both concrete
repository types and trait objects. Calls through the trait object reach the
injected implementation; Rust does not discover or select a provider on its own.
Provider factories and registrations stay in the composition root, not shared.
Shared owns interfaces, not a global service locator. Adapters must not expose
SQL, SDK types, or provider payloads across these contracts.

Provider errors become `RepositoryError` or another port-specific error.
Application errors express validation, missing resources, and port failures.
Presentation maps them to HTTP responses or `tonic::Status`; provider details are
logged internally rather than returned to clients. The local HTTP error type
implements Axum's `IntoResponse` and consumes application/domain errors without
importing shared or bringing Axum into its contracts.

Tracing, request IDs, authentication propagation, deadlines, and technical retries
belong at transport/integration boundaries. They must not decide domain outcomes
or duplicate entity invariants.

## File and contract conventions

- Use `<module>.rs` with a same-named folder for child modules; never `mod.rs`.
- Group entities, handlers, DTOs, and contracts by business module such as `item`.
  Infrastructure may group first by technical provider, then by business module.
- Keep command/query input and its handler together. Every handler exposes
  `execute`, receives dependencies in its constructor, and does not call another
  handler. Do not aggregate unrelated workflows into an `ItemService` or a base
  handler framework.
- Namespace files declare modules and re-exports. The gRPC module dispatcher is
  the explicit exception required to implement the generated service trait.
- Avoid catch-all `utils`, `common`, or `helpers` modules. Name types by purpose,
  for example `ItemRow` for storage rather than a second `Item`.
- Keep Protobuf source at `proto/<module>/v<major>/<schema>.proto`, with matching
  package `<module>.v<major>` and Rust wrapper path. Build bindings in
  `apps/presentation/build.rs` into `OUT_DIR`; do not commit generated Rust.
- Preserve released field numbers, message semantics, and RPC paths. Use a new
  major version for breaking changes and keep the old contract during migration.
  See [Protobuf rules](proto/AGENTS.md) for compatibility details.

## Extending and verifying the template

Start with a business capability: define its entity behavior in domain and the
required infrastructure interfaces in shared, add command/query handlers,
implement adapters, and define transport handlers. Register dependencies in root
`src/di.rs`, then mount selected handlers and update OpenAPI in the root app. Add
a new namespace consistently across affected layers.
Separate independently deployed services into their own composition roots and
contracts when required; the current workspace builds one service.

Tests belong to the layer whose behavior they verify:

- Domain: constructors, invariants, state changes, and identity preservation.
- Application: workflows with an in-memory repository or another test double.
- Infrastructure: migrations, persistence, and adapter semantics with real HiQLite.
- Presentation: DTO mapping and HTTP/gRPC error behavior.
- Root app: mounted routes, serialization, Swagger gating, and generated gRPC
  client behavior through the host's listener and injected dependencies.

Run these checks from the workspace root before completing a change:

```bash
cargo fmt --all -- --check
cargo check --workspace --locked --all-targets
cargo test --workspace --locked --all-targets
cargo clippy --workspace --locked --all-targets -- -D warnings
```

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
Stop with Ctrl-C to drain connections and shut down the database. `/health`
reports process liveness, not a database readiness check.

### REST example

```bash
curl -sS http://127.0.0.1:13000/health
curl -sS -X POST http://127.0.0.1:13000/items \
  -H 'content-type: application/json' -d '{"title":"First item"}'
```

Copy the returned `id` into `ITEM_ID`, then run:

```bash
ITEM_ID="paste-created-uuid-here"
curl -sS http://127.0.0.1:13000/items
curl -sS "http://127.0.0.1:13000/items/$ITEM_ID"
curl -sS -X PATCH "http://127.0.0.1:13000/items/$ITEM_ID" \
  -H 'content-type: application/json' -d '{"title":"Updated item"}'
curl -i -X DELETE "http://127.0.0.1:13000/items/$ITEM_ID"
```

Create returns HTTP 201; reads and updates return 200; deletion returns 204.
`POST /items/bulk-delete` accepts `{"ids":["<uuid>"]}` and returns 204, ignoring
missing IDs. Valid JSON with a blank title or malformed UUID returns 400; an
unknown UUID returns 404 for get/update/delete. Repository failures return a
generic 500 response. Axum handles malformed JSON and missing fields through its
standard extractor responses.

### gRPC example

Use grpcurl with the source contract; reflection is not enabled:

```bash
grpcurl -plaintext -import-path proto -proto item/v1/item.proto \
  -d '{"title":"First gRPC item"}' 127.0.0.1:13000 item.v1.ItemInternal/CreateItem
grpcurl -plaintext -import-path proto -proto item/v1/item.proto \
  -d '{}' 127.0.0.1:13000 item.v1.ItemInternal/ListItems
```

`GetItem`, `UpdateItem`, `DeleteItem`, and `DeleteItems` use the same service and
listener. Their request fields are defined in `proto/item/v1/item.proto`.
Validation errors map to `InvalidArgument`, missing records to `NotFound`, and
repository failures to a generic `Internal` status. The integration tests exercise
the generated Tonic client over the combined HTTP/gRPC listener.

## Container image

The Dockerfile uses four stages:

1. `toolchain` installs native build dependencies and the Rust version pinned in
   `rust-toolchain.toml`.
2. `build` compiles the root `app` binary in release mode with symbols stripped.
   BuildKit cache mounts retain downloaded crates and compiled dependencies for
   subsequent builds.
3. `base` provides a Distroless C/C++ runtime, HiQLite defaults, writable storage
   at `/data`, UID/GID `65532:65532`, and the `/app/app` entrypoint. It can be built
   independently of the Rust build stages and contains no application binary.
4. `runtime` extends `base` with the compiled demo binary at `/app/app`.

Both runtime images contain no source, Cargo, compiler, or shell.

SQL migrations, generated Protobuf bindings, and Swagger assets are embedded at
build time. Local env files, database state, Git history, and host build artifacts
are excluded from the build context by `.dockerignore`.

Build with Docker and the Buildx/BuildKit component:

```bash
docker build -t app:local .
docker build --target base -t app:base .
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
`curl http://127.0.0.1:13000/health`.

Without an explicit mount, Docker creates an anonymous volume, which is removed
with the container when using `--rm`. `VOLUME ["${HQL_DATA_DIR}"]` expands to
`/data` at build time; changing `HQL_DATA_DIR` at runtime does not change this
declared volume. Mount writable storage at the configured path when overriding
the data directory.

The default build uses one Cargo job to limit memory consumption. On a larger
builder, override it with `docker build --build-arg CARGO_BUILD_JOBS=4 -t app:local .`.

### Publishing and reusing images

`.github/workflows/publish-image.yml` runs when a Git tag is pushed. It builds
both Dockerfile targets for `linux/amd64` and `linux/arm64` and publishes to
`ghcr.io/<owner>/<repository>` using the workflow's `GITHUB_TOKEN`. The repository
name follows the current GitHub repository, so forks publish to their own path.
No additional registry credentials are required.

For a tag such as `v1.0.0`, the published images are:

| Image tag | Contents and use |
| --- | --- |
| `v1.0.0` | Runnable Item demo, built from the tagged source. |
| `base-v1.0.0` | Shared runtime and configuration for downstream service binaries. |

Commit and push the workflow and Dockerfile changes before creating a release
tag. Use Docker-compatible release tag names such as `v1.0.0`:

```bash
git tag v1.0.0
git push origin v1.0.0
```

For anonymous pulls, make the GHCR package public after its first publication.
For private packages, authenticate pulls and grant consuming repositories access
to the package. See [GHCR access and visibility](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry).

After compiling a downstream service for Linux, its Dockerfile can be:

```dockerfile
FROM ghcr.io/<owner>/<repository>:base-v1.0.0
COPY --chown=65532:65532 --chmod=755 target/release/my-service /app/app
EXPOSE 8080/tcp
```

Replace the owner, repository, version, and binary name with the intended values.
The base supplies `WORKDIR`, `USER`, HiQLite defaults, `VOLUME`, `STOPSIGNAL`, and
`ENTRYPOINT`; copying the binary to `/app/app` is enough to select what runs.
Configure the binary to listen on `0.0.0.0:8080` and publish it with
`--publish 8080:8080`. For hosts retaining this template's configuration, set
`APP_ADDR=0.0.0.0:8080`. `EXPOSE` does not change the application's listen port.

The binary must match the image's Linux architecture and be compatible with
Debian 13's glibc. Building with `rust:1-slim-trixie` matches this runtime. Any
additional dynamically linked libraries required by a service must also be
provided in its runtime image. HiQLite code and migrations remain embedded in
the service binary.
