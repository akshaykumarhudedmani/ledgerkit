//! Import pipeline: adapters → normalize → dedupe → rules.

pub mod adapter;
pub mod adapters;
pub mod convert;
pub mod dates;
pub mod normalize;
pub mod raw;

pub use adapter::{AdapterError, AdapterId, BankAdapter, ParseReport};
pub use convert::{convert_raw, ConvertOptions, ConvertReport};
pub use raw::{RawTransaction, RawTransactions};
