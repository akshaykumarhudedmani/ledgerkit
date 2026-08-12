//! Import pipeline: adapters → normalize → dedupe → rules.

pub mod adapter;
pub mod adapters;
pub mod normalize;
pub mod raw;

pub use adapter::{AdapterError, AdapterId, BankAdapter, ParseReport};
pub use raw::{RawTransaction, RawTransactions};
