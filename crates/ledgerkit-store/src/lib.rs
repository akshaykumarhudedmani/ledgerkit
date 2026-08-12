//! SQLite-backed append-only event store.
//!
//! Phase 1 ships schema + open/append/replay stubs. Full event chaining lands in Phase 2.

mod db;
mod schema;

pub use db::Store;
pub use schema::SCHEMA_VERSION;
