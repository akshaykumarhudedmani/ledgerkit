use anyhow::{bail, Result};
use ledgerkit_core::{EventKind, EventPayload, TransactionId};

use crate::Store;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhyStep {
    pub seq: u64,
    pub kind: &'static str,
    pub summary: String,
}

impl Store {
    /// Walk the event log for every event that references `tx_id`.
    pub fn why_transaction(&self, tx_id: TransactionId) -> Result<Vec<WhyStep>> {
        let snapshot = self.load_snapshot()?;
        let tx = snapshot.transactions.iter().find(|t| t.id == tx_id);
        let batch = tx.and_then(|t| t.import_batch_id);
        let events = self.list_events()?;
        let mut steps = Vec::new();
        for event in events {
            let summary = match &event.payload {
                EventPayload::Posted { transaction } if transaction.id == tx_id => Some(format!(
                    "posted payee={} date={} postings={}",
                    transaction.payee,
                    transaction.date,
                    transaction.postings.len()
                )),
                EventPayload::Imported {
                    batch_id,
                    source_path,
                    source_sha256,
                    row_count,
                } if batch == Some(*batch_id) => Some(format!(
                    "imported batch={batch_id} source={source_path} sha256={source_sha256} rows={row_count}"
                )),
                EventPayload::Normalized {
                    transaction_id,
                    reasons,
                } if *transaction_id == tx_id => {
                    Some(format!("normalized reasons={}", reasons.join(";")))
                }
                EventPayload::Deduped {
                    duplicate_id,
                    survivor_id,
                    strategy,
                    explanation,
                } if *duplicate_id == tx_id || *survivor_id == tx_id => Some(format!(
                    "deduped {duplicate_id} -> {survivor_id} ({strategy}) {explanation}"
                )),
                EventPayload::Categorized {
                    transaction_id,
                    category,
                    rule_id,
                    confidence,
                    reasons,
                } if *transaction_id == tx_id => Some(format!(
                    "categorized {category} rule={rule_id} conf={confidence} reasons={}",
                    reasons.join(";")
                )),
                EventPayload::ManualEdit {
                    transaction_id,
                    summary,
                } if *transaction_id == tx_id => Some(format!("manual_edit {summary}")),
                EventPayload::Reconciled { account, as_of, .. } => {
                    let in_proof = tx
                        .map(|t| {
                            let hits = t
                                .postings
                                .iter()
                                .any(|p| p.account.as_str() == account.as_str());
                            let as_of_date =
                                chrono::NaiveDate::parse_from_str(as_of, "%Y-%m-%d").ok();
                            hits && as_of_date.is_some_and(|d| t.date <= d)
                        })
                        .unwrap_or(false);
                    if in_proof {
                        Some(format!("reconciled account={account} as_of={as_of}"))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(summary) = summary {
                let kind = match event.kind {
                    EventKind::AccountUpserted => "account_upserted",
                    EventKind::Posted => "posted",
                    EventKind::Imported => "imported",
                    EventKind::Normalized => "normalized",
                    EventKind::Deduped => "deduped",
                    EventKind::Categorized => "categorized",
                    EventKind::Reconciled => "reconciled",
                    EventKind::ManualEdit => "manual_edit",
                };
                steps.push(WhyStep {
                    seq: event.seq,
                    kind,
                    summary,
                });
            }
        }
        if steps.is_empty() {
            bail!("no events reference transaction {tx_id}");
        }
        Ok(steps)
    }

    pub fn why_statement_row(&self, row_id: i64) -> Result<Vec<WhyStep>> {
        let row = self
            .get_statement_row(row_id)?
            .ok_or_else(|| anyhow::anyhow!("no statement row {row_id}"))?;
        let mut steps = vec![WhyStep {
            seq: 0,
            kind: "statement_row",
            summary: format!(
                "id={row_id} batch={} row={} status={} fingerprint={} tx={}",
                row.batch_id,
                row.row_number,
                row.parse_status,
                row.fingerprint.as_deref().unwrap_or("-"),
                row.transaction_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "-".into())
            ),
        }];
        if let Some(tx_id) = row.transaction_id {
            if let Ok(more) = self.why_transaction(tx_id) {
                steps.extend(more);
            }
        }
        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use crate::Store;
    use ledgerkit_core::{Account, AccountId, AccountType, Amount, Commodity, Transaction};
    use tempfile::tempdir;

    #[test]
    fn why_includes_posted_deduped_categorized() {
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
                AccountId::new("expenses:food").unwrap(),
                AccountType::Expense,
                Commodity::new("USD").unwrap(),
                "Food",
            ))
            .unwrap();
        let date = chrono::NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();
        let a = Transaction::transfer(
            date,
            AccountId::new("assets:bank").unwrap(),
            AccountId::new("expenses:food").unwrap(),
            Amount::parse("6.50").unwrap(),
            Commodity::new("USD").unwrap(),
            "STARBUCKS",
        )
        .unwrap();
        let b = Transaction::transfer(
            date,
            AccountId::new("assets:bank").unwrap(),
            AccountId::new("expenses:food").unwrap(),
            Amount::parse("6.50").unwrap(),
            Commodity::new("USD").unwrap(),
            "STARBUCKS",
        )
        .unwrap();
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
        let later = Transaction::transfer(
            chrono::NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
            AccountId::new("assets:bank").unwrap(),
            AccountId::new("expenses:food").unwrap(),
            Amount::parse("1.00").unwrap(),
            Commodity::new("USD").unwrap(),
            "Later",
        )
        .unwrap();
        let id_later = later.id;
        store.post_transaction(later).unwrap();
        store
            .record_reconcile(
                &ledgerkit_core::prove_reconcile(
                    &store.load_snapshot().unwrap(),
                    &ledgerkit_core::ReconcileRequest {
                        account: AccountId::new("assets:bank").unwrap(),
                        commodity: Commodity::new("USD").unwrap(),
                        as_of: date,
                        stated_ending: Amount::parse("-6.50").unwrap(),
                        starting: Amount::zero(),
                    },
                )
                .unwrap(),
                Some("reports/reconcile-assets_bank-2026-01-02.md".into()),
            )
            .unwrap();

        let why_a = store.why_transaction(id_a).unwrap();
        assert!(why_a.iter().any(|s| s.kind == "posted"));
        assert!(why_a.iter().any(|s| s.kind == "categorized"));
        assert!(why_a.iter().any(|s| s.kind == "deduped"));
        assert!(why_a.iter().any(|s| s.kind == "reconciled"));

        let why_b = store.why_transaction(id_b).unwrap();
        assert!(why_b.iter().any(|s| s.kind == "deduped"));
        assert!(why_b.iter().any(|s| s.summary.contains(&id_a.to_string())));
        assert!(why_b.iter().any(|s| s.kind == "reconciled"));

        let why_later = store.why_transaction(id_later).unwrap();
        assert!(why_later.iter().any(|s| s.kind == "posted"));
        assert!(
            why_later.iter().all(|s| s.kind != "reconciled"),
            "txs after as_of must not inherit the reconcile event: {why_later:?}"
        );
    }
}
