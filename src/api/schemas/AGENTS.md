# API Schemas

Applies to `src/api/schemas/` and its child modules. Follow the
[HTTP presentation rules](../AGENTS.md).

- Group schemas by bounded context in folders such as `item/`. Keep each context's schemas in its own folder.
- Within each context, define each request, response, query, and nested DTO in an
  independent file named after its type in snake case. For example,
  `item/create_item_request.rs` defines `CreateItemRequest`, and
  `item/item_view.rs` defines `ItemView`.
- Do not collect multiple DTOs in generic `request.rs` or `response.rs` files.
- Use `<context>.rs` namespace files for module declarations and re-exports;
  never use `mod.rs`. Preserve context-level imports when reorganizing schemas.
- Shared transport schemas used across contexts may remain directly under
  `schemas/`, with one DTO per file.
- Keep schemas limited to declarations, serialization attributes, and schema
  metadata. Keep conversions in `api/mappers/` and business rules in their owning
  layers. Preserve JSON field names, types, defaults, and validation attributes
  during file refactors.
