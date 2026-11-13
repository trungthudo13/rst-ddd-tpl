CREATE TABLE IF NOT EXISTS items (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_items_created_at
    ON items (created_at, id);
