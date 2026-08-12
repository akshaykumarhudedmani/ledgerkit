use anyhow::{bail, Result};
use ledgerkit_core::{ContentHash, Event, EventKind, EventPayload, TransactionId};
use rusqlite::OptionalExtension;

use crate::events::{append_sealed_event, content_hash_from_hex};
use crate::Store;

impl Store {
    pub fn mark_duplicate(
        &mut self,
        duplicate_id: TransactionId,
        survivor_id: TransactionId,
        strategy: &str,
        explanation: &str,
    ) -> Result<Event> {
        if duplicate_id == survivor_id {
            bail!("duplicate_id must differ from survivor_id");
        }
        let db_tx = self.conn.transaction()?;
        let current = db_tx
            .query_row(
                "SELECT duplicate_of FROM transactions WHERE id = ?1",
                [duplicate_id.to_string()],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?;
        match current {
            None => bail!("transaction {duplicate_id} not found"),
            Some(Some(_)) => {
                bail!("transaction {duplicate_id} is already linked as a duplicate")
            }
            Some(None) => {}
        }
        let exists: i64 = db_tx.query_row(
            "SELECT COUNT(*) FROM transactions WHERE id = ?1",
            [survivor_id.to_string()],
            |row| row.get(0),
        )?;
        if exists == 0 {
            bail!("survivor {survivor_id} not found");
        }
        db_tx.execute(
            "UPDATE transactions SET duplicate_of = ?1 WHERE id = ?2",
            [survivor_id.to_string(), duplicate_id.to_string()],
        )?;
        let prev = tip_hash(&db_tx)?;
        let event = Event::seal(
            EventKind::Deduped,
            EventPayload::Deduped {
                duplicate_id,
                survivor_id,
                strategy: strategy.to_string(),
                explanation: explanation.to_string(),
            },
            prev,
        );
        let stored = append_sealed_event(&db_tx, event)?;
        db_tx.commit()?;
        Ok(stored)
    }

    pub fn apply_category(
        &mut self,
        transaction_id: TransactionId,
        category: &str,
        rule_id: &str,
        confidence: u8,
        reasons: Vec<String>,
    ) -> Result<Event> {
        let db_tx = self.conn.transaction()?;
        let tags_json: String = db_tx.query_row(
            "SELECT tags_json FROM transactions WHERE id = ?1",
            [transaction_id.to_string()],
            |row| row.get(0),
        )?;
        let mut tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        tags.retain(|t| !t.starts_with("category:"));
        tags.push(format!("category:{category}"));
        let new_json = serde_json::to_string(&tags)?;
        db_tx.execute(
            "UPDATE transactions SET tags_json = ?1 WHERE id = ?2",
            [new_json, transaction_id.to_string()],
        )?;
        let prev = tip_hash(&db_tx)?;
        let event = Event::seal(
            EventKind::Categorized,
            EventPayload::Categorized {
                transaction_id,
                category: category.to_string(),
                rule_id: rule_id.to_string(),
                confidence,
                reasons,
            },
            prev,
        );
        let stored = append_sealed_event(&db_tx, event)?;
        db_tx.commit()?;
        Ok(stored)
    }

    pub fn record_reconcile(
        &mut self,
        proof: &ledgerkit_core::ReconcileProof,
        report_path: Option<String>,
    ) -> Result<Event> {
        let unmatched = (proof.skipped_duplicates.len() + proof.after_as_of.len()) as u64;
        let db_tx = self.conn.transaction()?;
        let prev = tip_hash(&db_tx)?;
        let event = Event::seal(
            EventKind::Reconciled,
            EventPayload::Reconciled {
                account: proof.account.clone(),
                as_of: proof.as_of.to_string(),
                ending_balance: proof.stated_ending.to_string(),
                matched: proof.matched.len() as u64,
                unmatched,
                unexplained_delta: proof.unexplained_delta.to_string(),
                report_path,
            },
            prev,
        );
        let stored = append_sealed_event(&db_tx, event)?;
        db_tx.commit()?;
        Ok(stored)
    }
}

fn tip_hash(conn: &rusqlite::Connection) -> Result<ContentHash> {
    match conn.query_row(
        "SELECT content_hash FROM events ORDER BY seq DESC LIMIT 1",
        [],
        |row| row.get::<_, String>(0),
    ) {
        Ok(hex) => Ok(content_hash_from_hex(hex)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(ContentHash::zero()),
        Err(err) => Err(err.into()),
    }
}

#[cfg(test)]
mod tests {
    use crate::Store;
    use ledgerkit_core::{
        account_balance, Account, AccountId, AccountType, Amount, Commodity, Transaction,
    };
    use tempfile::tempdir;

    #[test]
    fn dedupe_links_without_deleting_and_replay_matches() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path().join("l.sqlite")).unwrap();
        store
            .upsert_account(Account::new(
                AccountId::new("assets:bank").unwrap(),
                AccountType::Asset,
                Commodity::new("USD").unwrap(),
                "Bank",
            ))
            .unwrap();
        store
            .upsert_account(Account::new(
                AccountId::new("expenses:uncategorized").unwrap(),
                AccountType::Expense,
                Commodity::new("USD").unwrap(),
                "Exp",
            ))
            .unwrap();
        let date = chrono::NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();
        let mk = |payee: &str| {
            Transaction::transfer(
                date,
                AccountId::new("assets:bank").unwrap(),
                AccountId::new("expenses:uncategorized").unwrap(),
                Amount::parse("6.50").unwrap(),
                Commodity::new("USD").unwrap(),
                payee,
            )
            .unwrap()
        };
        let a = mk("STARBUCKS");
        let b = mk("STARBUCKS");
        let id_a = a.id;
        let id_b = b.id;
        store.post_transaction(a).unwrap();
        store.post_transaction(b).unwrap();
        store
            .mark_duplicate(id_b, id_a, "exact", "same fingerprint")
            .unwrap();
        store
            .apply_category(id_a, "expenses:food", "coffee", 80, vec!["payee".into()])
            .unwrap();

        let snap = store.load_snapshot().unwrap();
        assert_eq!(snap.transactions.len(), 2);
        let dup = snap.transactions.iter().find(|t| t.id == id_b).unwrap();
        assert_eq!(dup.duplicate_of, Some(id_a));
        let survivor = snap.transactions.iter().find(|t| t.id == id_a).unwrap();
        assert!(survivor.tags.iter().any(|t| t == "category:expenses:food"));

        let (mat, rep) = store.assert_replay_matches_materialized().unwrap();
        assert_eq!(mat.ledger_hash, rep.ledger_hash);

        let bal = account_balance(
            &snap,
            &AccountId::new("assets:bank").unwrap(),
            &Commodity::new("USD").unwrap(),
        )
        .unwrap();
        assert_eq!(bal.to_string(), "-6.50");
    }
}
