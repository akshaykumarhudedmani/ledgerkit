use anyhow::{Context, Result};
use ledgerkit_core::{ImportBatchId, TransactionId};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Store;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementRowSpec {
    pub row_number: i64,
    pub date_raw: Option<String>,
    pub amount_raw: Option<String>,
    pub currency_raw: Option<String>,
    pub description_raw: Option<String>,
    pub balance_raw: Option<String>,
    pub source_refs: Vec<String>,
    pub fingerprint: Option<String>,
    pub parse_status: String,
    pub error: Option<String>,
    pub transaction_id: Option<TransactionId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementRow {
    pub id: i64,
    pub batch_id: ImportBatchId,
    pub adapter: String,
    pub account_id: String,
    pub row_number: i64,
    pub date_raw: Option<String>,
    pub amount_raw: Option<String>,
    pub currency_raw: Option<String>,
    pub description_raw: Option<String>,
    pub balance_raw: Option<String>,
    pub source_refs: Vec<String>,
    pub fingerprint: Option<String>,
    pub parse_status: String,
    pub error: Option<String>,
    pub transaction_id: Option<TransactionId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowReconcileReport {
    pub account: String,
    pub matched: usize,
    pub unmatched_rows: Vec<i64>,
    pub convert_errors: Vec<i64>,
    pub parse_errors: Vec<i64>,
    pub unmatched_txns: Vec<String>,
}

impl RowReconcileReport {
    pub fn ok(&self) -> bool {
        self.unmatched_rows.is_empty()
            && self.convert_errors.is_empty()
            && self.parse_errors.is_empty()
            && self.unmatched_txns.is_empty()
    }

    pub fn to_markdown(&self) -> String {
        let status = if self.ok() {
            "ROWS_MATCHED"
        } else {
            "ROW_GAPS"
        };
        let mut out = String::new();
        out.push_str("# Statement-row reconciliation\n\n");
        out.push_str(&format!("- account: `{}`\n", self.account));
        out.push_str(&format!("- matched: {}\n", self.matched));
        out.push_str(&format!(
            "- unmatched_rows: {}\n",
            self.unmatched_rows.len()
        ));
        out.push_str(&format!(
            "- convert_errors: {}\n",
            self.convert_errors.len()
        ));
        out.push_str(&format!("- parse_errors: {}\n", self.parse_errors.len()));
        out.push_str(&format!(
            "- unmatched_imported_txns: {}\n",
            self.unmatched_txns.len()
        ));
        out.push_str(&format!("- status: {status}\n"));
        out
    }
}

pub(crate) fn insert_statement_rows(
    conn: &rusqlite::Connection,
    batch_id: ImportBatchId,
    adapter: &str,
    account_id: &str,
    rows: &[StatementRowSpec],
) -> Result<()> {
    for row in rows {
        let refs = serde_json::to_string(&row.source_refs)?;
        conn.execute(
            "INSERT INTO statement_rows
               (batch_id, adapter, account_id, row_number, date_raw, amount_raw, currency_raw,
                description_raw, balance_raw, source_refs, fingerprint, parse_status, error, transaction_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            (
                batch_id.to_string(),
                adapter,
                account_id,
                row.row_number,
                row.date_raw.as_deref(),
                row.amount_raw.as_deref(),
                row.currency_raw.as_deref(),
                row.description_raw.as_deref(),
                row.balance_raw.as_deref(),
                refs,
                row.fingerprint.as_deref(),
                row.parse_status.as_str(),
                row.error.as_deref(),
                row.transaction_id.map(|id| id.to_string()),
            ),
        )?;
    }
    Ok(())
}

impl Store {
    pub fn list_statement_rows(&self, account_id: &str) -> Result<Vec<StatementRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, batch_id, adapter, account_id, row_number, date_raw, amount_raw, currency_raw,
                    description_raw, balance_raw, source_refs, fingerprint, parse_status, error, transaction_id
             FROM statement_rows
             WHERE account_id = ?1
             ORDER BY batch_id, row_number",
        )?;
        let mapped = stmt.query_map([account_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
            ))
        })?;
        let mut out = Vec::new();
        for item in mapped {
            let (
                id,
                batch,
                adapter,
                account,
                row_number,
                date_raw,
                amount_raw,
                currency_raw,
                description_raw,
                balance_raw,
                refs_json,
                fingerprint,
                parse_status,
                error,
                tx_id,
            ) = item?;
            out.push(StatementRow {
                id,
                batch_id: ImportBatchId::from_uuid(Uuid::parse_str(&batch).context("batch id")?),
                adapter,
                account_id: account,
                row_number,
                date_raw,
                amount_raw,
                currency_raw,
                description_raw,
                balance_raw,
                source_refs: serde_json::from_str(&refs_json).unwrap_or_default(),
                fingerprint,
                parse_status,
                error,
                transaction_id: tx_id
                    .map(|s| Ok::<_, anyhow::Error>(TransactionId::parse(&s)?))
                    .transpose()?,
            });
        }
        Ok(out)
    }

    pub fn get_statement_row(&self, id: i64) -> Result<Option<StatementRow>> {
        let account: Option<String> = self
            .conn
            .query_row(
                "SELECT account_id FROM statement_rows WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(account) = account else {
            return Ok(None);
        };
        Ok(self
            .list_statement_rows(&account)?
            .into_iter()
            .find(|r| r.id == id))
    }

    /// Match persisted statement rows to imported transactions for one account.
    pub fn prove_row_reconcile(&self, account_id: &str) -> Result<RowReconcileReport> {
        let rows = self.list_statement_rows(account_id)?;
        let snapshot = self.load_snapshot()?;
        let mut linked: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut matched = 0usize;
        let mut unmatched_rows = Vec::new();
        let mut convert_errors = Vec::new();
        let mut parse_errors = Vec::new();
        for row in &rows {
            match row.parse_status.as_str() {
                "ok" if row.transaction_id.is_some() => {
                    matched += 1;
                    if let Some(id) = row.transaction_id {
                        linked.insert(id.to_string());
                    }
                }
                "convert_error" => convert_errors.push(row.id),
                "parse_error" => parse_errors.push(row.id),
                _ => unmatched_rows.push(row.id),
            }
        }
        let unmatched_txns = snapshot
            .transactions
            .iter()
            .filter(|t| {
                t.import_batch_id.is_some()
                    && t.postings.iter().any(|p| p.account.as_str() == account_id)
                    && !linked.contains(&t.id.to_string())
            })
            .map(|t| t.id.to_string())
            .collect();
        Ok(RowReconcileReport {
            account: account_id.to_string(),
            matched,
            unmatched_rows,
            convert_errors,
            parse_errors,
            unmatched_txns,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ImportBatchSpec, Store};
    use chrono::Utc;
    use ledgerkit_core::{
        Account, AccountId, AccountType, Amount, Commodity, ContentHash, ImportBatchId, Transaction,
    };
    use tempfile::tempdir;

    #[test]
    fn row_reconcile_matches_imported_specs() {
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
        let mut tx = Transaction::transfer(
            chrono::NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
            AccountId::new("assets:bank").unwrap(),
            AccountId::new("expenses:uncategorized").unwrap(),
            Amount::parse("10").unwrap(),
            Commodity::new("USD").unwrap(),
            "Shop",
        )
        .unwrap();
        tx.id = ledgerkit_core::TransactionId::from_fingerprint("row-recon");
        let spec = ImportBatchSpec {
            id: ImportBatchId::new(),
            adapter: "generic_csv".into(),
            account_id: "assets:bank".into(),
            source_path: "a.csv".into(),
            source_sha256: ContentHash::sha256_str("x"),
            imported_at: Utc::now(),
            row_count: 1,
        };
        let rows = vec![StatementRowSpec {
            row_number: 2,
            date_raw: Some("2026-01-02".into()),
            amount_raw: Some("10".into()),
            currency_raw: Some("USD".into()),
            description_raw: Some("Shop".into()),
            balance_raw: None,
            source_refs: vec!["generic:row:2".into()],
            fingerprint: Some("row-recon".into()),
            parse_status: "ok".into(),
            error: None,
            transaction_id: Some(tx.id),
        }];
        store
            .apply_import(spec, vec![bank, exp], vec![tx], rows)
            .unwrap();
        let report = store.prove_row_reconcile("assets:bank").unwrap();
        assert!(report.ok(), "{report:?}");
        assert_eq!(report.matched, 1);
    }
}
