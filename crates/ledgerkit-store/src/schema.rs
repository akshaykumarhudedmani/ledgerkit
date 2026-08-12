//! SQL schema for LedgerKit local store.
//!
//! Design constraints:
//! - Events are append-only (no UPDATE/DELETE on `events`).
//! - Import artifacts store sha256 of original bytes.
//! - Derived balances are never stored as source of truth.

pub const SCHEMA_VERSION: i32 = 2;

pub const SCHEMA_SQL: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS accounts (
    id           TEXT PRIMARY KEY NOT NULL,
    account_type TEXT NOT NULL,
    commodity    TEXT NOT NULL,
    name         TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS commodities (
    code     TEXT PRIMARY KEY NOT NULL,
    decimals INTEGER NOT NULL DEFAULT 2
);

CREATE TABLE IF NOT EXISTS merchants (
    id              TEXT PRIMARY KEY NOT NULL,
    canonical_key   TEXT NOT NULL UNIQUE,
    display_name    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS merchant_aliases (
    alias       TEXT PRIMARY KEY NOT NULL,
    merchant_id TEXT NOT NULL REFERENCES merchants(id)
);

CREATE TABLE IF NOT EXISTS import_batches (
    id            TEXT PRIMARY KEY NOT NULL,
    adapter       TEXT NOT NULL,
    account_id    TEXT NOT NULL,
    source_path   TEXT NOT NULL,
    source_sha256 TEXT NOT NULL,
    imported_at   TEXT NOT NULL,
    row_count     INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS transactions (
    id               TEXT PRIMARY KEY NOT NULL,
    date             TEXT NOT NULL,
    payee            TEXT NOT NULL,
    merchant_id      TEXT REFERENCES merchants(id),
    narration        TEXT,
    import_batch_id  TEXT REFERENCES import_batches(id),
    duplicate_of     TEXT REFERENCES transactions(id),
    tags_json        TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS postings (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    transaction_id TEXT NOT NULL REFERENCES transactions(id),
    account_id     TEXT NOT NULL,
    amount         TEXT NOT NULL,
    commodity      TEXT NOT NULL,
    memo           TEXT,
    ordinal        INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS events (
    seq          INTEGER PRIMARY KEY AUTOINCREMENT,
    id           TEXT NOT NULL UNIQUE,
    at           TEXT NOT NULL,
    kind         TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    prev_hash    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_postings_account ON postings(account_id);
CREATE INDEX IF NOT EXISTS idx_transactions_date ON transactions(date);
CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind);
CREATE UNIQUE INDEX IF NOT EXISTS idx_import_batches_idempotent
  ON import_batches(adapter, source_sha256, account_id);
"#;
