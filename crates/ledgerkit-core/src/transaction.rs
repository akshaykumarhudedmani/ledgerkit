use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};
use crate::ids::{ImportBatchId, MerchantId, TransactionId};
use crate::money::{Amount, Commodity};
use crate::posting::Posting;
use crate::verify::verify_transaction;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub id: TransactionId,
    pub date: NaiveDate,
    pub payee: String,
    pub merchant_id: Option<MerchantId>,
    pub narration: Option<String>,
    pub postings: Vec<Posting>,
    pub import_batch_id: Option<ImportBatchId>,
    /// Soft link: never delete duplicates; point at survivor.
    pub duplicate_of: Option<TransactionId>,
    pub tags: Vec<String>,
    /// Import row fingerprint (`v1|…`). Manual `tx add` leaves this unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_fingerprint: Option<String>,
}

impl Transaction {
    pub fn new(date: NaiveDate, payee: impl Into<String>, postings: Vec<Posting>) -> Result<Self> {
        let tx = Self {
            id: TransactionId::new(),
            date,
            payee: payee.into(),
            merchant_id: None,
            narration: None,
            postings,
            import_batch_id: None,
            duplicate_of: None,
            tags: Vec::new(),
            row_fingerprint: None,
        };
        verify_transaction(&tx)?;
        Ok(tx)
    }

    /// Balanced transfer helper: debit `from`, credit `to` by `amount`.
    pub fn transfer(
        date: NaiveDate,
        from: crate::account::AccountId,
        to: crate::account::AccountId,
        amount: Amount,
        commodity: Commodity,
        payee: impl Into<String>,
    ) -> Result<Self> {
        let credit = amount
            .checked_neg()
            .ok_or_else(|| CoreError::InvalidAmount("negation overflow".into()))?;
        Self::new(
            date,
            payee,
            vec![
                Posting::new(from, credit, commodity.clone()),
                Posting::new(to, amount, commodity),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::AccountId;
    use crate::money::Commodity;
    use pretty_assertions::assert_eq;

    #[test]
    fn rejects_unbalanced_transaction() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let err = Transaction::new(
            date,
            "Bad",
            vec![Posting::new(
                AccountId::new("assets:cash").unwrap(),
                Amount::parse("10").unwrap(),
                Commodity::new("INR").unwrap(),
            )],
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::UnbalancedTransaction { .. }));
    }

    #[test]
    fn accepts_balanced_transfer() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let tx = Transaction::transfer(
            date,
            AccountId::new("assets:bank:hdfc").unwrap(),
            AccountId::new("expenses:food").unwrap(),
            Amount::parse("250.00").unwrap(),
            Commodity::new("INR").unwrap(),
            "Cafe",
        )
        .unwrap();
        assert_eq!(tx.postings.len(), 2);
    }
}
