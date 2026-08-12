use thiserror::Error;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CoreError {
    #[error("transaction {tx} does not balance: {detail}")]
    UnbalancedTransaction { tx: String, detail: String },

    #[error("invalid money amount: {0}")]
    InvalidAmount(String),

    #[error("invalid account name: {0}")]
    InvalidAccount(String),

    #[error("invariant violated: {0}")]
    Invariant(String),
}
