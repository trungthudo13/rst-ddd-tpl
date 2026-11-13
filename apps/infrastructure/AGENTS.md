# Infrastructure Layer

Applies to `apps/infrastructure/`.

## Responsibility and rationale

Implement the infrastructure interfaces declared in `shared/ports/` using
concrete providers. Own connection lifecycle,
persistence, serialization, technical retry behavior, and provider error mapping.
Keeping these mechanisms here allows databases and external services to change
without changing domain behavior or application workflows.

Depend inward on domain/shared. Never depend on application or presentation.
Declare interfaces in shared; infrastructure owns only their implementations.
Bind associated entity types to domain entities, for example `type Entity = Item`
in the HiQLite implementation of `shared::ItemRepository`.

## Module responsibilities

| Path | Scope and reason for separation |
| --- | --- |
| `adapters/hiqlite/database.rs` | Node connection, embedded migrations, and shutdown; isolates database lifecycle from entity SQL. |
| `adapters/hiqlite/item_repository.rs` | `ItemRepository` implementation; isolates provider queries and row mapping from use cases. |
| `models/item.rs` | Crate-private `ItemRow`; keeps table layout separate from domain entities and public DTOs. |
| `migrations/` | Ordered embedded SQL migrations; schema evolution stays with its adapter. |
| `adapters/cache.rs` | Future provider connections, keys, encoding, and technical retries; the calling layer owns TTL policy. |
| `clients/internal_service.rs` | Future clients for other internal services; isolates endpoints, authentication, protocol, and service contract handling. |
| `clients/third_party.rs` | Future vendor clients; isolates SDK types, rate limits, and vendor error mapping. |
| `messaging/broker.rs` | Future broker connection and lifecycle; independent of any individual publication. |
| `messaging/producer.rs` | Future publication, routing, headers, and retries; independent of message meaning. |
| `messaging/protocol.rs` | Future envelopes, encoding, and wire compatibility; independent of domain events. |
| `notifications/email.rs` and `sms.rs` | Future delivery mechanisms; independent of recipients, timing, and business content. |
| `interceptors.rs` | Future outbound tracing, metrics, credentials propagation, and deadlines; independent of domain outcomes. |

Only HiQLite persistence is implemented. The other namespaces are extension
points, not working cache, messaging, notification, or service integrations. Add
provider implementations as child modules after defining a real use case and
port. Group technical namespaces by provider/protocol and then business module.

## Boundaries

- Do not implement business rules, aggregate behavior, use-case orchestration,
  or business decisions about TTL, delivery policy, or workflow.
- Do not own inbound queue consumers, cron listeners, REST/gRPC DTOs, or handlers.
  Inbound adapters drive use cases and belong in presentation.
- Keep SQL and persistence models inside this package. Validate reconstructed
  entities through the domain and never return `ItemRow` across the boundary.
- Respect repository semantics, including missing updates/deletes. Do not turn
  an update into an insert or expose a provider failure as a business outcome.
- Bind data values as SQL parameters and map provider errors into port errors.
- Do not hard-code deployment credentials, endpoints, or secrets in adapters.
- Technical retries, serialization, rate limits, and provider error translation
  stay here; the application owns workflow and message/content decisions.
- Middleware/interceptors must not decide business outcomes. Inbound transport
  middleware belongs in presentation.
- Use `<module>.rs` and same-named child folders, never `mod.rs`. Namespace files
  contain declarations/re-exports, not provider implementations. Avoid catch-all
  `utils`, `common`, and `helpers` modules.

## Definition of done

Test adapters against the real provider or a suitable test container. The Item
adapter tests use isolated temporary HiQLite nodes and verify migrations, CRUD,
missing rows, and persistence. Shut nodes down and clean temporary state. Keep
provider details out of clients' error responses. Run the workspace checks in
`apps/AGENTS.md`.
