# Application Layer

Applies to `apps/application/`.

## Responsibility and rationale

Own use cases: coordinate domain behavior and outbound ports, define workflow and
transaction requirements, and return domain entities or explicit application
outputs. Keeping orchestration here lets REST, gRPC, and future inbound adapters
reuse the same behavior without knowing a database or provider.

## Commands and queries

- Each operation has one input and handler in the same file under `commands/` or
  `queries/`. The Item example includes create, update, delete, bulk-delete, get,
  and list operations.
- Name inputs for intent, such as `CreateItemCommand` and `GetItemQuery`. They are
  application inputs, not transport DTOs, and must not derive wire serialization.
- Handlers are named `<Verb><Entity>Handler`, receive dependencies through their
  constructors, and expose `execute` as the public use-case entry point.
- Query handlers do not change state. Commands may return an entity or a clearly
  named application result.
- Keep entity invariants in the domain; invoke entity behavior rather than
  rebuilding or directly changing protected state in application code.
- Do not call one handler from another, create an all-purpose `ItemService`, or
  introduce base handlers, a generic use-case framework, or a `services/` module.
- Extract a helper or port only when real usage requires it. Shared helpers need
  at least two actual consumers. A large workflow may use smaller files while
  preserving its `execute` entry point.
- Add input fields because the use case needs them, not because an adapter does.

## Dependencies and outbound ports

- Runtime dependencies are domain/shared and ordinary value libraries only.
- Never import infrastructure, presentation, Axum, Tonic, HiQLite, SQL, or a
  provider SDK. Do not construct concrete databases, brokers, or clients.
- Infrastructure interfaces for repositories, email, messaging, cache, service
  calls, or clocks belong in `shared/ports/`. Application defines the workflow
  requirements and consumes those contracts, not concrete adapters.
- Ports use provider-independent values and associated types or generic
  parameters for domain data; shared must not import domain/application.
- Bind `ItemRepository::Entity` to `domain::Item`. Handlers accept an injected
  `Arc<R>` where `R: ItemRepository<Entity = Item> + ?Sized`, supporting both a
  concrete implementation and `Arc<dyn ItemRepository<Entity = Item>>`.
- The default repository type is the trait object. Presentation can consume
  `CreateItemHandler`, `GetItemsHandler`, and the other handler types without
  importing infrastructure ports or choosing a provider.
- Do not resolve dependencies through a global service locator or instantiate
  providers inside use cases; the composition root supplies dependencies.
- Do not send messages or notifications directly through a provider.
- Map domain and port failures into `shared::ApplicationError`. Missing rows on a
  write remain not-found errors rather than backend failures.
- Keep `ItemRow`, REST/gRPC DTOs, HTTP statuses, endpoints, and credentials outside
  this package.

## Definition of done

Each operation has a handler and a meaningful test with an in-memory repository
or test double. Tests require neither a live database nor a transport server.
Verify validation, state preservation, missing resources, and port failures as
appropriate. Keep module declarations in `<module>.rs`, tests in `src/tests/`, and
run the workspace checks in `apps/AGENTS.md`.
