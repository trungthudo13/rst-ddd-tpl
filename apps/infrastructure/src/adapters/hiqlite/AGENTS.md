# HiQLite Adapter Rules

Applies to `apps/infrastructure/src/adapters/hiqlite/` and its descendants.

## Raw SQL management

- Centralize all raw SQL queries, statements, and reusable fragments in `.sql`
  files under `apps/infrastructure/src/adapters/hiqlite/sqls/`. This directory is
  the single source of truth for the adapter's SQL.
- Do not embed raw SQL literals in Rust source or duplicate SQL definitions in
  repositories or mappers.
- Organize SQL files by capability and use descriptive operation names. Reuse
  existing scripts when multiple callers need the same SQL.
- Load SQL files with `include_str!` using relative paths. Keep execution,
  parameter binding, transaction coordination, and row mapping in their owning
  Rust adapters and mappers.
- Bind runtime data as SQL parameters. Any required dynamic SQL structure must
  use trusted, code-controlled fragments from `sqls/`.
