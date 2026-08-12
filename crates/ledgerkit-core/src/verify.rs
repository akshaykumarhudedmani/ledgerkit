use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::account::AccountId;
use crate::error::{CoreError, Result};
use crate::hash::ContentHash;
use crate::money::{Amount, Commodity};
use crate::transaction::Transaction;

/// Point-in-time ledger view derived from postings (single source of truth).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerSnapshot {
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyReport {
    pub ok: bool,
    pub transaction_count: usize,
    pub posting_count: usize,
    pub unbalanced: Vec<String>,
    pub ledger_hash: ContentHash,
}

/// Invariant: for each commodity in a transaction, posting amounts sum to zero.
pub fn verify_transaction(tx: &Transaction) -> Result<()> {
    let mut sums: BTreeMap<&str, Amount> = BTreeMap::new();
    for posting in &tx.postings {
        let entry = sums
            .entry(posting.commodity.as_str())
            .or_insert_with(Amount::zero);
        *entry = entry
            .checked_add(posting.amount)
            .ok_or_else(|| CoreError::InvalidAmount("amount overflow".into()))?;
    }

    let mut bad = Vec::new();
    for (commodity, sum) in sums {
        if !sum.is_zero() {
            bad.push(format!("{commodity} residual={sum}"));
        }
    }

    if !bad.is_empty() {
        return Err(CoreError::UnbalancedTransaction {
            tx: tx.id.to_string(),
            detail: bad.join("; "),
        });
    }

    if tx.postings.len() < 2 {
        return Err(CoreError::UnbalancedTransaction {
            tx: tx.id.to_string(),
            detail: "transaction requires at least two postings".into(),
        });
    }

    Ok(())
}

pub fn verify_ledger(snapshot: &LedgerSnapshot) -> VerifyReport {
    let mut unbalanced = Vec::new();
    let mut posting_count = 0usize;

    for tx in &snapshot.transactions {
        posting_count += tx.postings.len();
        if let Err(err) = verify_transaction(tx) {
            unbalanced.push(err.to_string());
        }
    }

    let canonical = serde_json::to_vec(&snapshot.transactions).unwrap_or_default();
    let ledger_hash = ContentHash::sha256_bytes(&canonical);

    VerifyReport {
        ok: unbalanced.is_empty(),
        transaction_count: snapshot.transactions.len(),
        posting_count,
        unbalanced,
        ledger_hash,
    }
}

/// Sum postings for one account (single source of truth for balances).
pub fn account_balance(
    snapshot: &LedgerSnapshot,
    account: &AccountId,
    commodity: &Commodity,
) -> Amount {
    let mut total = Amount::zero();
    for tx in &snapshot.transactions {
        if tx.duplicate_of.is_some() {
            continue;
        }
        for posting in &tx.postings {
            if &posting.account == account && &posting.commodity == commodity {
                if let Some(next) = total.checked_add(posting.amount) {
                    total = next;
                }
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::AccountId;
    use crate::money::{Amount, Commodity};
    use crate::posting::Posting;
    use chrono::NaiveDate;
    use proptest::prelude::*;

    #[test]
    fn balanced_ledger_verifies() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let tx = Transaction::transfer(
            date,
            AccountId::new("assets:cash").unwrap(),
            AccountId::new("expenses:misc").unwrap(),
            Amount::parse("100").unwrap(),
            Commodity::new("USD").unwrap(),
            "Shop",
        )
        .unwrap();
        let snap = LedgerSnapshot {
            transactions: vec![tx],
        };
        let report = verify_ledger(&snap);
        assert!(report.ok);
        assert_eq!(report.transaction_count, 1);
    }

    proptest! {
        #[test]
        fn transfer_amounts_always_balance(raw in -1_000_000i64..1_000_000i64) {
            prop_assume!(raw != 0);
            let amount = Amount::from_decimal(rust_decimal::Decimal::new(raw, 2));
            let date = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
            let tx = Transaction::new(
                date,
                "prop",
                vec![
                    Posting::new(
                        AccountId::new("assets:a").unwrap(),
                        amount.checked_neg().unwrap(),
                        Commodity::new("INR").unwrap(),
                    ),
                    Posting::new(
                        AccountId::new("expenses:b").unwrap(),
                        amount,
                        Commodity::new("INR").unwrap(),
                    ),
                ],
            );
            prop_assert!(tx.is_ok());
        }
    }
}
