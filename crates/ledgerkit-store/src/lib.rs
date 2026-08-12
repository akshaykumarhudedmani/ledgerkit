//! SQLite-backed append-only event store and materialized ledger.

mod db;
mod events;
mod ledger;
mod replay;
mod schema;

pub use db::Store;
pub use schema::SCHEMA_VERSION;
