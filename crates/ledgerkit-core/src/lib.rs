//! LedgerKit core: money-safe types, double-entry model, and invariants.
//!
//! # Non-negotiable rules
//! - Never use floating-point for money.
//! - Every transaction must balance (sum of postings == 0 per commodity).
//! - Account balances are derived from postings only.

pub mod account;
pub mod error;
pub mod event;
pub mod hash;
pub mod ids;
pub mod money;
pub mod posting;
pub mod transaction;
pub mod verify;

pub use account::{Account, AccountId, AccountType};
pub use error::{CoreError, Result};
pub use event::{Event, EventKind, EventPayload};
pub use hash::ContentHash;
pub use ids::{CategoryId, ImportBatchId, MerchantId, TransactionId};
pub use money::{Amount, Commodity, Money};
pub use posting::Posting;
pub use transaction::Transaction;
pub use verify::{
    account_balance, verify_ledger, verify_transaction, LedgerSnapshot, VerifyReport,
};
