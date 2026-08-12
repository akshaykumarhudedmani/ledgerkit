use thiserror::Error;

use crate::raw::RawTransactions;

/// Stable plugin id, e.g. `hdfc`, `generic_csv`, `credit_card`, `custom`.
pub type AdapterId = str;

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("parse error at row {row}: {message}")]
    Row { row: usize, message: String },

    #[error("invalid schema: {0}")]
    Schema(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseReport {
    pub ok_rows: usize,
    pub error_rows: usize,
    pub errors: Vec<String>,
}

/// Bank statement adapter. Implementations must be deterministic.
pub trait BankAdapter: Send + Sync {
    fn id(&self) -> &AdapterId;
    fn name(&self) -> &str;
    fn parse(&self, bytes: &[u8]) -> Result<(RawTransactions, ParseReport), AdapterError>;
}
