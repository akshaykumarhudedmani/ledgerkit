use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ledgerkit_core::{
    Account, ContentHash, Event, EventKind, EventPayload, ImportBatchId, Transaction,
};
use uuid::Uuid;

use crate::events::append_sealed_event;
use crate::ledger::{insert_account_rows, insert_transaction_rows};
use crate::rows::{insert_statement_rows, StatementRowSpec};
use crate::Store;

#[derive(Debug, Clone)]
pub struct ImportBatchSpec {
    pub id: ImportBatchId,
    pub adapter: String,
    pub account_id: String,
    pub source_path: String,
    pub source_sha256: ContentHash,
    pub imported_at: DateTime<Utc>,
    pub row_count: u64,
}

#[derive(Debug, Clone)]
pub enum ImportOutcome {
    Applied {
        batch_id: ImportBatchId,
        posted: u64,
        skipped_existing: u64,
        last_seq: u64,
    },
    Duplicate {
        batch_id: ImportBatchId,
    },
}

impl Store {
    pub fn find_import_batch(
        &self,
        adapter: &str,
        source_sha256: &ContentHash,
        account_id: &str,
    ) -> Result<Option<ImportBatchId>> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM import_batches
             WHERE adapter = ?1 AND source_sha256 = ?2 AND account_id = ?3",
        )?;
        let mut rows = stmt.query((adapter, source_sha256.as_str(), account_id))?;
        if let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            Ok(Some(ImportBatchId::from_uuid(
                Uuid::parse_str(&id).context("import batch id")?,
            )))
        } else {
            Ok(None)
        }
    }

    /// Persist a parsed import: optional new accounts, batch row, Imported event, Posted txns.
    /// Idempotent on (adapter, source_sha256, account_id).
    /// Existing transaction ids (overlapping files) are skipped, not re-posted.
    pub fn apply_import(
        &mut self,
        spec: ImportBatchSpec,
        accounts: Vec<Account>,
        transactions: Vec<Transaction>,
        statement_rows: Vec<StatementRowSpec>,
    ) -> Result<ImportOutcome> {
        if let Some(existing) =
            self.find_import_batch(&spec.adapter, &spec.source_sha256, &spec.account_id)?
        {
            return Ok(ImportOutcome::Duplicate { batch_id: existing });
        }

        let db_tx = self.conn.transaction()?;

        for account in &accounts {
            let exists: i64 = db_tx.query_row(
                "SELECT COUNT(*) FROM accounts WHERE id = ?1",
                [account.id.as_str()],
                |row| row.get(0),
            )?;
            if exists == 0 {
                insert_account_rows(&db_tx, account)?;
                let prev = last_hash(&db_tx)?;
                let event = Event::seal(
                    EventKind::AccountUpserted,
                    EventPayload::AccountUpserted {
                        account: account.clone(),
                    },
                    prev,
                );
                append_sealed_event(&db_tx, event)?;
            }
        }

        db_tx.execute(
            "INSERT INTO import_batches
               (id, adapter, account_id, source_path, source_sha256, imported_at, row_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                spec.id.to_string(),
                spec.adapter.as_str(),
                spec.account_id.as_str(),
                spec.source_path.as_str(),
                spec.source_sha256.as_str(),
                spec.imported_at.to_rfc3339(),
                spec.row_count as i64,
            ),
        )?;

        let prev = last_hash(&db_tx)?;
        let imported = Event::seal(
            EventKind::Imported,
            EventPayload::Imported {
                batch_id: spec.id,
                source_path: spec.source_path.clone(),
                source_sha256: spec.source_sha256.clone(),
                row_count: spec.row_count,
            },
            prev,
        );
        append_sealed_event(&db_tx, imported)?;

        insert_statement_rows(
            &db_tx,
            spec.id,
            &spec.adapter,
            &spec.account_id,
            &statement_rows,
        )?;

        let mut posted = 0u64;
        let mut skipped_existing = 0u64;
        let mut last_seq = 0u64;
        for transaction in transactions {
            ledgerkit_core::verify_transaction(&transaction)?;
            let exists: i64 = db_tx.query_row(
                "SELECT COUNT(*) FROM transactions WHERE id = ?1",
                [transaction.id.to_string()],
                |row| row.get(0),
            )?;
            if exists > 0 {
                skipped_existing += 1;
                continue;
            }
            insert_transaction_rows(&db_tx, &transaction)?;
            let prev = last_hash(&db_tx)?;
            let event = Event::seal(
                EventKind::Posted,
                EventPayload::Posted { transaction },
                prev,
            );
            let stored = append_sealed_event(&db_tx, event)?;
            last_seq = stored.seq;
            posted += 1;
        }

        db_tx.commit()?;
        Ok(ImportOutcome::Applied {
            batch_id: spec.id,
            posted,
            skipped_existing,
            last_seq,
        })
    }
}

fn last_hash(conn: &rusqlite::Connection) -> Result<ContentHash> {
    match conn.query_row(
        "SELECT content_hash FROM events ORDER BY seq DESC LIMIT 1",
        [],
        |row| row.get::<_, String>(0),
    ) {
        Ok(hex) => Ok(crate::events::content_hash_from_hex(hex)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(ContentHash::zero()),
        Err(err) => Err(err.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;
    use ledgerkit_core::{AccountId, AccountType, Amount, Commodity, Transaction};
    use tempfile::tempdir;

    #[test]
    fn import_is_idempotent_and_posts() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path().join("l.sqlite")).unwrap();
        let bank = Account::new(
            AccountId::new("assets:bank").unwrap(),
            AccountType::Asset,
            Commodity::new("USD").unwrap(),
            "Bank",
        );
        let exp = Account::new(
            AccountId::new("expenses:uncategorized").unwrap(),
            AccountType::Expense,
            Commodity::new("USD").unwrap(),
            "Uncat",
        );
        let date = chrono::NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();
        let tx = Transaction::transfer(
            date,
            AccountId::new("assets:bank").unwrap(),
            AccountId::new("expenses:uncategorized").unwrap(),
            Amount::parse("10").unwrap(),
            Commodity::new("USD").unwrap(),
            "Shop",
        )
        .unwrap();
        let spec = ImportBatchSpec {
            id: ImportBatchId::new(),
            adapter: "generic_csv".into(),
            account_id: "assets:bank".into(),
            source_path: "sample.csv".into(),
            source_sha256: ContentHash::sha256_str("bytes"),
            imported_at: Utc::now(),
            row_count: 1,
        };
        let first = store
            .apply_import(spec.clone(), vec![bank, exp], vec![tx.clone()], vec![])
            .unwrap();
        assert!(matches!(first, ImportOutcome::Applied { posted: 1, .. }));
        let second = store.apply_import(spec, vec![], vec![tx], vec![]).unwrap();
        assert!(matches!(second, ImportOutcome::Duplicate { .. }));
        assert_eq!(store.transaction_count().unwrap(), 1);
        store.assert_replay_matches_materialized().unwrap();
    }

    #[test]
    fn overlapping_file_skips_existing_id() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path().join("l.sqlite")).unwrap();
        let bank = Account::new(
            AccountId::new("assets:bank").unwrap(),
            AccountType::Asset,
            Commodity::new("USD").unwrap(),
            "Bank",
        );
        let exp = Account::new(
            AccountId::new("expenses:uncategorized").unwrap(),
            AccountType::Expense,
            Commodity::new("USD").unwrap(),
            "Uncat",
        );
        let date = chrono::NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();
        let mut tx = Transaction::transfer(
            date,
            AccountId::new("assets:bank").unwrap(),
            AccountId::new("expenses:uncategorized").unwrap(),
            Amount::parse("10").unwrap(),
            Commodity::new("USD").unwrap(),
            "Shop",
        )
        .unwrap();
        tx.id = ledgerkit_core::TransactionId::from_fingerprint("v1|overlap");
        tx.row_fingerprint = Some("v1|overlap".into());
        let spec1 = ImportBatchSpec {
            id: ImportBatchId::new(),
            adapter: "generic_csv".into(),
            account_id: "assets:bank".into(),
            source_path: "a.csv".into(),
            source_sha256: ContentHash::sha256_str("file-a"),
            imported_at: Utc::now(),
            row_count: 1,
        };
        let spec2 = ImportBatchSpec {
            id: ImportBatchId::new(),
            adapter: "generic_csv".into(),
            account_id: "assets:bank".into(),
            source_path: "b.csv".into(),
            source_sha256: ContentHash::sha256_str("file-b"),
            imported_at: Utc::now(),
            row_count: 1,
        };
        store
            .apply_import(spec1, vec![bank, exp], vec![tx.clone()], vec![])
            .unwrap();
        let second = store.apply_import(spec2, vec![], vec![tx], vec![]).unwrap();
        assert!(matches!(
            second,
            ImportOutcome::Applied {
                posted: 0,
                skipped_existing: 1,
                ..
            }
        ));
        assert_eq!(store.transaction_count().unwrap(), 1);
        store.assert_replay_matches_materialized().unwrap();
    }

    #[test]
    fn convert_error_only_import_persists_statement_rows() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(dir.path().join("l.sqlite")).unwrap();
        let bank = Account::new(
            AccountId::new("assets:bank").unwrap(),
            AccountType::Asset,
            Commodity::new("USD").unwrap(),
            "Bank",
        );
        let spec = ImportBatchSpec {
            id: ImportBatchId::new(),
            adapter: "generic_csv".into(),
            account_id: "assets:bank".into(),
            source_path: "bad.csv".into(),
            source_sha256: ContentHash::sha256_str("bad"),
            imported_at: Utc::now(),
            row_count: 1,
        };
        let rows = vec![StatementRowSpec {
            row_number: 2,
            date_raw: Some("not-a-date".into()),
            amount_raw: Some("10".into()),
            currency_raw: None,
            description_raw: Some("x".into()),
            balance_raw: None,
            source_refs: vec![],
            fingerprint: None,
            parse_status: "convert_error".into(),
            error: Some("unparseable date".into()),
            transaction_id: None,
        }];
        let outcome = store.apply_import(spec, vec![bank], vec![], rows).unwrap();
        assert!(matches!(
            outcome,
            ImportOutcome::Applied {
                posted: 0,
                skipped_existing: 0,
                ..
            }
        ));
        let report = store.prove_row_reconcile("assets:bank").unwrap();
        assert_eq!(report.convert_errors.len(), 1);
        assert!(!report.ok());
    }
}
