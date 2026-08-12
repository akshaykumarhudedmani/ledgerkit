//! Import pipeline: adapters → normalize → dedupe → rules.

pub mod adapter;
pub mod adapters;
pub mod convert;
pub mod dates;
pub mod dedupe;
pub mod normalize;
pub mod raw;
pub mod rules;

pub use adapter::{AdapterError, AdapterId, BankAdapter, ParseReport, MAX_IMPORT_BYTES};
pub use convert::{convert_raw, row_fingerprint, ConvertOptions, ConvertReport};
pub use dedupe::{plan_dedupe, DedupeOptions, DedupeReport};
pub use raw::{RawTransaction, RawTransactions};
pub use rules::{apply_rules, RuleReport, RuleSet};
