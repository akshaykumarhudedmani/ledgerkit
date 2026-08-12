use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::account::AccountId;
use crate::error::{CoreError, Result};
use crate::ids::TransactionId;
use crate::money::{Amount, Commodity};
use crate::verify::LedgerSnapshot;

#[derive(Debug, Clone)]
pub struct ReconcileRequest {
    pub account: AccountId,
    pub commodity: Commodity,
    pub as_of: NaiveDate,
    pub stated_ending: Amount,
    pub starting: Amount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofLine {
    pub transaction_id: TransactionId,
    pub date: NaiveDate,
    pub payee: String,
    pub amount: Amount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconcileProof {
    pub account: String,
    pub commodity: String,
    pub as_of: NaiveDate,
    pub starting: Amount,
    pub computed_ending: Amount,
    pub stated_ending: Amount,
    pub unexplained_delta: Amount,
    pub matched: Vec<ProofLine>,
    pub skipped_duplicates: Vec<ProofLine>,
    pub after_as_of: Vec<ProofLine>,
}

impl ReconcileProof {
    pub fn ok(&self) -> bool {
        self.unexplained_delta.is_zero()
    }

    pub fn proof_filename(&self) -> String {
        let acct = self.account.replace(':', "_");
        format!("reconcile-{acct}-{}.md", self.as_of)
    }

    /// Deterministic markdown proof (no wall-clock).
    pub fn to_markdown(&self) -> String {
        let status = if self.ok() {
            "BALANCED"
        } else {
            "UNEXPLAINED_DELTA"
        };
        let mut out = String::new();
        out.push_str("# Reconciliation proof\n\n");
        out.push_str(&format!("- account: `{}`\n", self.account));
        out.push_str(&format!("- commodity: `{}`\n", self.commodity));
        out.push_str(&format!("- as_of: {}\n", self.as_of));
        out.push_str(&format!("- starting: {}\n", self.starting));
        out.push_str(&format!("- computed_ending: {}\n", self.computed_ending));
        out.push_str(&format!("- stated_ending: {}\n", self.stated_ending));
        out.push_str(&format!(
            "- unexplained_delta: {}\n",
            self.unexplained_delta
        ));
        out.push_str(&format!("- status: {status}\n"));
        out.push_str(&format!("- matched: {}\n", self.matched.len()));
        out.push_str(&format!(
            "- unmatched: {}\n\n",
            self.skipped_duplicates.len() + self.after_as_of.len()
        ));

        out.push_str("## Included postings (skip duplicates)\n\n");
        if self.matched.is_empty() {
            out.push_str("_none_\n\n");
        } else {
            out.push_str("| date | tx | payee | amount |\n|---|---|---|---|\n");
            for line in &self.matched {
                out.push_str(&format!(
                    "| {} | `{}` | {} | {} |\n",
                    line.date, line.transaction_id, line.payee, line.amount
                ));
            }
            out.push('\n');
        }

        out.push_str("## Skipped duplicates\n\n");
        if self.skipped_duplicates.is_empty() {
            out.push_str("_none_\n\n");
        } else {
            out.push_str("| date | tx | payee | amount |\n|---|---|---|---|\n");
            for line in &self.skipped_duplicates {
                out.push_str(&format!(
                    "| {} | `{}` | {} | {} |\n",
                    line.date, line.transaction_id, line.payee, line.amount
                ));
            }
            out.push('\n');
        }

        out.push_str("## After as_of (excluded)\n\n");
        if self.after_as_of.is_empty() {
            out.push_str("_none_\n");
        } else {
            out.push_str("| date | tx | payee | amount |\n|---|---|---|---|\n");
            for line in &self.after_as_of {
                out.push_str(&format!(
                    "| {} | `{}` | {} | {} |\n",
                    line.date, line.transaction_id, line.payee, line.amount
                ));
            }
        }
        out
    }
}

fn line_for(tx: &crate::transaction::Transaction, amount: Amount) -> ProofLine {
    ProofLine {
        transaction_id: tx.id,
        date: tx.date,
        payee: tx.payee.clone(),
        amount,
    }
}

/// Prove statement ending balance from postings (duplicates never counted).
pub fn prove_reconcile(
    snapshot: &LedgerSnapshot,
    req: &ReconcileRequest,
) -> Result<ReconcileProof> {
    let mut matched = Vec::new();
    let mut skipped_duplicates = Vec::new();
    let mut after_as_of = Vec::new();
    let mut activity = Amount::zero();

    for tx in &snapshot.transactions {
        let mut hit = Amount::zero();
        let mut any = false;
        for posting in &tx.postings {
            if posting.account == req.account && posting.commodity == req.commodity {
                any = true;
                hit = hit.checked_add(posting.amount).ok_or_else(|| {
                    CoreError::InvalidAmount(format!(
                        "overflow summing {} {}",
                        req.account, req.commodity
                    ))
                })?;
            }
        }
        if !any {
            continue;
        }
        if tx.date > req.as_of {
            after_as_of.push(line_for(tx, hit));
            continue;
        }
        if tx.duplicate_of.is_some() {
            skipped_duplicates.push(line_for(tx, hit));
            continue;
        }
        activity = activity.checked_add(hit).ok_or_else(|| {
            CoreError::InvalidAmount(format!(
                "overflow summing {} {}",
                req.account, req.commodity
            ))
        })?;
        matched.push(line_for(tx, hit));
    }

    matched.sort_by(|a, b| {
        a.date.cmp(&b.date).then_with(|| {
            a.transaction_id
                .to_string()
                .cmp(&b.transaction_id.to_string())
        })
    });
    skipped_duplicates.sort_by(|a, b| {
        a.date.cmp(&b.date).then_with(|| {
            a.transaction_id
                .to_string()
                .cmp(&b.transaction_id.to_string())
        })
    });
    after_as_of.sort_by(|a, b| {
        a.date.cmp(&b.date).then_with(|| {
            a.transaction_id
                .to_string()
                .cmp(&b.transaction_id.to_string())
        })
    });

    let computed_ending = req
        .starting
        .checked_add(activity)
        .ok_or_else(|| CoreError::InvalidAmount("overflow adding starting balance".into()))?;
    let unexplained_delta = computed_ending
        .checked_sub(req.stated_ending)
        .ok_or_else(|| CoreError::InvalidAmount("overflow computing unexplained delta".into()))?;

    Ok(ReconcileProof {
        account: req.account.to_string(),
        commodity: req.commodity.to_string(),
        as_of: req.as_of,
        starting: req.starting,
        computed_ending,
        stated_ending: req.stated_ending,
        unexplained_delta,
        matched,
        skipped_duplicates,
        after_as_of,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::AccountId;
    use crate::money::{Amount, Commodity};
    use crate::transaction::Transaction;
    use crate::verify::LedgerSnapshot;

    fn transfer(date: &str, payee: &str, amt: &str) -> Transaction {
        let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
        Transaction::transfer(
            d,
            AccountId::new("assets:bank").unwrap(),
            AccountId::new("expenses:food").unwrap(),
            Amount::parse(amt).unwrap(),
            Commodity::new("USD").unwrap(),
            payee,
        )
        .unwrap()
    }

    #[test]
    fn balanced_statement_has_zero_delta() {
        let tx = transfer("2026-01-03", "Cafe", "6.50");
        let snap = LedgerSnapshot {
            transactions: vec![tx],
        };
        let proof = prove_reconcile(
            &snap,
            &ReconcileRequest {
                account: AccountId::new("assets:bank").unwrap(),
                commodity: Commodity::new("USD").unwrap(),
                as_of: NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
                stated_ending: Amount::parse("-6.50").unwrap(),
                starting: Amount::zero(),
            },
        )
        .unwrap();
        assert!(proof.ok());
        assert_eq!(proof.matched.len(), 1);
        assert_eq!(
            proof.proof_filename(),
            "reconcile-assets_bank-2026-01-31.md"
        );
    }

    #[test]
    fn mismatch_reports_delta_and_skips_duplicates() {
        let a = transfer("2026-01-03", "Cafe", "6.50");
        let mut b = transfer("2026-01-03", "Cafe", "6.50");
        let later = transfer("2026-02-01", "Later", "1.00");
        b.duplicate_of = Some(a.id);
        let snap = LedgerSnapshot {
            transactions: vec![a, b, later],
        };
        let proof = prove_reconcile(
            &snap,
            &ReconcileRequest {
                account: AccountId::new("assets:bank").unwrap(),
                commodity: Commodity::new("USD").unwrap(),
                as_of: NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
                stated_ending: Amount::parse("0").unwrap(),
                starting: Amount::zero(),
            },
        )
        .unwrap();
        assert!(!proof.ok());
        assert_eq!(proof.unexplained_delta.to_string(), "-6.50");
        assert_eq!(proof.matched.len(), 1);
        assert_eq!(proof.skipped_duplicates.len(), 1);
        assert_eq!(proof.after_as_of.len(), 1);
    }

    #[test]
    fn starting_balance_is_included() {
        let tx = transfer("2026-01-03", "Cafe", "6.50");
        let snap = LedgerSnapshot {
            transactions: vec![tx],
        };
        let proof = prove_reconcile(
            &snap,
            &ReconcileRequest {
                account: AccountId::new("assets:bank").unwrap(),
                commodity: Commodity::new("USD").unwrap(),
                as_of: NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
                stated_ending: Amount::parse("93.50").unwrap(),
                starting: Amount::parse("100").unwrap(),
            },
        )
        .unwrap();
        assert!(proof.ok());
        assert_eq!(proof.computed_ending.to_string(), "93.50");
    }
}
