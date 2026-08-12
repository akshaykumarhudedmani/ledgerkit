use serde::{Deserialize, Serialize};

/// Pre-canonical transaction produced by a bank adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawTransaction {
    pub row_number: usize,
    pub date_raw: String,
    pub amount_raw: String,
    pub currency_raw: Option<String>,
    pub description_raw: String,
    pub balance_raw: Option<String>,
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawTransactions {
    pub adapter_id: String,
    pub transactions: Vec<RawTransaction>,
}
