use anyhow::Result;
use ledgerkit_core::{
    account_balance, verify_ledger, AccountId, Amount, Commodity, EventPayload, LedgerSnapshot,
    VerifyReport,
};

use crate::Store;

impl Store {
    /// Rebuild ledger snapshot by folding events with `seq <= max_seq`.
    ///
    /// `Posted` events add transactions; `Deduped` / `Categorized` fold onto them.
    /// Account events are applied for completeness but do not affect the ledger content hash.
    pub fn replay_through(&self, max_seq: u64) -> Result<LedgerSnapshot> {
        let events = self.events_through(max_seq)?;
        let mut snapshot = LedgerSnapshot::default();
        for event in events {
            match event.payload {
                EventPayload::Posted { transaction } => {
                    // Replace if same id appears (shouldn't in v1 append-only).
                    if let Some(pos) = snapshot
                        .transactions
                        .iter()
                        .position(|t| t.id == transaction.id)
                    {
                        snapshot.transactions[pos] = transaction;
                    } else {
                        snapshot.transactions.push(transaction);
                    }
                }
                EventPayload::Deduped {
                    duplicate_id,
                    survivor_id,
                    ..
                } => {
                    if let Some(tx) = snapshot
                        .transactions
                        .iter_mut()
                        .find(|t| t.id == duplicate_id)
                    {
                        tx.duplicate_of = Some(survivor_id);
                    }
                }
                EventPayload::Categorized {
                    transaction_id,
                    category,
                    ..
                } => {
                    if let Some(tx) = snapshot
                        .transactions
                        .iter_mut()
                        .find(|t| t.id == transaction_id)
                    {
                        tx.tags.retain(|t| !t.starts_with("category:"));
                        tx.tags.push(format!("category:{category}"));
                    }
                }
                _ => {}
            }
        }
        Ok(snapshot)
    }

    pub fn replay_all(&self) -> Result<LedgerSnapshot> {
        self.replay_through(u64::MAX)
    }

    /// Materialized snapshot vs full event replay must share the same ledger hash.
    pub fn assert_replay_matches_materialized(&self) -> Result<(VerifyReport, VerifyReport)> {
        let materialized = self.load_snapshot()?;
        let replayed = self.replay_all()?;
        let a = verify_ledger(&materialized);
        let b = verify_ledger(&replayed);
        if a.ledger_hash != b.ledger_hash {
            anyhow::bail!(
                "replay hash mismatch: materialized={} replayed={}",
                a.ledger_hash,
                b.ledger_hash
            );
        }
        Ok((a, b))
    }

    pub fn balance_of(&self, account: &AccountId, commodity: &Commodity) -> Result<Amount> {
        let snapshot = self.load_snapshot()?;
        Ok(account_balance(&snapshot, account, commodity)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;
    use ledgerkit_core::{Account, AccountType, Transaction};
    use tempfile::tempdir;

    #[test]
    fn post_verify_and_replay_hash_match() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path().join("ledger.sqlite")).unwrap();

        store
            .upsert_account(Account::new(
                AccountId::new("assets:cash").unwrap(),
                AccountType::Asset,
                Commodity::new("INR").unwrap(),
                "Cash",
            ))
            .unwrap();
        store
            .upsert_account(Account::new(
                AccountId::new("expenses:food").unwrap(),
                AccountType::Expense,
                Commodity::new("INR").unwrap(),
                "Food",
            ))
            .unwrap();

        let date = chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let tx = Transaction::transfer(
            date,
            AccountId::new("assets:cash").unwrap(),
            AccountId::new("expenses:food").unwrap(),
            Amount::parse("120.50").unwrap(),
            Commodity::new("INR").unwrap(),
            "Cafe",
        )
        .unwrap();
        store.post_transaction(tx).unwrap();

        assert_eq!(store.event_count().unwrap(), 3);
        store.verify_event_chain().unwrap();

        let (mat, rep) = store.assert_replay_matches_materialized().unwrap();
        assert!(mat.ok);
        assert_eq!(mat.ledger_hash, rep.ledger_hash);
        assert_eq!(mat.transaction_count, 1);

        let bal = store
            .balance_of(
                &AccountId::new("assets:cash").unwrap(),
                &Commodity::new("INR").unwrap(),
            )
            .unwrap();
        assert_eq!(bal.to_string(), "-120.50");
    }

    #[test]
    fn time_travel_replay_through_stops_early() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path().join("ledger.sqlite")).unwrap();
        store
            .upsert_account(Account::new(
                AccountId::new("assets:a").unwrap(),
                AccountType::Asset,
                Commodity::new("USD").unwrap(),
                "A",
            ))
            .unwrap();
        store
            .upsert_account(Account::new(
                AccountId::new("expenses:b").unwrap(),
                AccountType::Expense,
                Commodity::new("USD").unwrap(),
                "B",
            ))
            .unwrap();

        let date = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let tx1 = Transaction::transfer(
            date,
            AccountId::new("assets:a").unwrap(),
            AccountId::new("expenses:b").unwrap(),
            Amount::parse("10").unwrap(),
            Commodity::new("USD").unwrap(),
            "one",
        )
        .unwrap();
        let tx2 = Transaction::transfer(
            date,
            AccountId::new("assets:a").unwrap(),
            AccountId::new("expenses:b").unwrap(),
            Amount::parse("5").unwrap(),
            Commodity::new("USD").unwrap(),
            "two",
        )
        .unwrap();
        let e1 = store.post_transaction(tx1).unwrap();
        store.post_transaction(tx2).unwrap();

        let mid = store.replay_through(e1.seq).unwrap();
        assert_eq!(mid.transactions.len(), 1);
        let all = store.replay_all().unwrap();
        assert_eq!(all.transactions.len(), 2);
    }
}
