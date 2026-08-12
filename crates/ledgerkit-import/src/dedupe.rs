use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::NaiveDate;
use ledgerkit_core::{Amount, Transaction, TransactionId};
use serde::{Deserialize, Serialize};

use crate::normalize::normalize_merchant;

#[derive(Debug, Clone, Copy)]
pub struct DedupeOptions {
    /// Inclusive calendar-day window for near-duplicates (0 = exact date only).
    pub window_days: i64,
}

impl Default for DedupeOptions {
    fn default() -> Self {
        Self { window_days: 1 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DedupeLink {
    pub duplicate_id: TransactionId,
    pub survivor_id: TransactionId,
    pub strategy: String,
    pub explanation: String,
    pub confidence: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DedupeReport {
    pub links: Vec<DedupeLink>,
    pub skipped_already_linked: usize,
}

#[derive(Clone)]
struct Finger {
    id: TransactionId,
    date: NaiveDate,
    key_exact: String,
    key_near: String,
}

fn primary_amount_account(tx: &Transaction) -> Option<(String, Amount, String)> {
    let p = tx.postings.first()?;
    Some((
        p.account.as_str().to_string(),
        p.amount,
        p.commodity.as_str().to_string(),
    ))
}

fn merchant_key(tx: &Transaction) -> String {
    if let Some(n) = &tx.narration {
        if !n.is_empty() {
            return n.clone();
        }
    }
    normalize_merchant(&tx.payee).canonical_key
}

fn fingers(txs: &[Transaction]) -> Vec<Finger> {
    let mut out = Vec::new();
    for tx in txs {
        if tx.duplicate_of.is_some() {
            continue;
        }
        let Some((account, amount, commodity)) = primary_amount_account(tx) else {
            continue;
        };
        let merchant = merchant_key(tx);
        let exact = format!(
            "{}|{}|{}|{}|{}",
            tx.date, account, amount, commodity, merchant
        );
        let near = format!("{account}|{amount}|{commodity}|{merchant}");
        out.push(Finger {
            id: tx.id,
            date: tx.date,
            key_exact: exact,
            key_near: near,
        });
    }
    out
}

/// Plan duplicate links. Never deletes; later links point at the earliest survivor.
pub fn plan_dedupe(txs: &[Transaction], opts: DedupeOptions) -> DedupeReport {
    let skipped_already_linked = txs.iter().filter(|t| t.duplicate_of.is_some()).count();
    let items = fingers(txs);
    let mut claimed: HashSet<String> = HashSet::new();
    let mut links = Vec::new();

    let mut exact: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, f) in items.iter().enumerate() {
        exact.entry(f.key_exact.clone()).or_default().push(i);
    }
    for group in exact.values() {
        if group.len() < 2 {
            continue;
        }
        let mut ordered = group.clone();
        ordered.sort_by(|&i, &j| {
            items[i]
                .date
                .cmp(&items[j].date)
                .then_with(|| items[i].id.to_string().cmp(&items[j].id.to_string()))
        });
        let survivor = items[ordered[0]].id;
        for &idx in &ordered[1..] {
            let dup = items[idx].id;
            claimed.insert(dup.to_string());
            claimed.insert(survivor.to_string());
            links.push(DedupeLink {
                duplicate_id: dup,
                survivor_id: survivor,
                strategy: "exact".into(),
                explanation: format!(
                    "same date+account+amount+commodity+merchant as {}",
                    survivor
                ),
                confidence: 100,
            });
        }
        claimed.insert(survivor.to_string());
    }

    if opts.window_days >= 0 {
        let mut near: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, f) in items.iter().enumerate() {
            if claimed.contains(&f.id.to_string()) {
                continue;
            }
            near.entry(f.key_near.clone()).or_default().push(i);
        }
        for group in near.values() {
            if group.len() < 2 {
                continue;
            }
            let mut ordered = group.clone();
            ordered.sort_by(|&i, &j| {
                items[i]
                    .date
                    .cmp(&items[j].date)
                    .then_with(|| items[i].id.to_string().cmp(&items[j].id.to_string()))
            });
            for a in 0..ordered.len() {
                for b in (a + 1)..ordered.len() {
                    let ia = ordered[a];
                    let ib = ordered[b];
                    let da = items[ia].date;
                    let db = items[ib].date;
                    let delta = (db - da).num_days().abs();
                    if delta > opts.window_days {
                        continue;
                    }
                    if claimed.contains(&items[ib].id.to_string()) {
                        continue;
                    }
                    let survivor = items[ia].id;
                    let duplicate = items[ib].id;
                    claimed.insert(duplicate.to_string());
                    links.push(DedupeLink {
                        duplicate_id: duplicate,
                        survivor_id: survivor,
                        strategy: "near_window".into(),
                        explanation: format!(
                            "same account+amount+merchant; dates {da} and {db} within {} day(s)",
                            opts.window_days
                        ),
                        confidence: 85,
                    });
                }
            }
        }
    }

    DedupeReport {
        links,
        skipped_already_linked,
    }
}

/// Precision/recall against labeled pairs of payee strings that should be duplicates
/// when converted with identical date/amount/account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupEvalCase {
    pub payee_a: String,
    pub payee_b: String,
    pub same_date: bool,
    pub same_amount: bool,
    pub should_link: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryMetrics {
    pub precision: f64,
    pub recall: f64,
    pub true_positive: usize,
    pub false_positive: usize,
    pub false_negative: usize,
}

/// Evaluate planner on synthetic pairs. Uses f64 only for the metric ratio, never money.
pub fn evaluate_dedup_cases(cases: &[DedupEvalCase], opts: DedupeOptions) -> BinaryMetrics {
    let date_a = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
    let date_b_same = date_a;
    let date_b_near = date_a
        .checked_add_signed(chrono::Duration::days(1))
        .unwrap();
    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut fn_ = 0usize;

    for (i, case) in cases.iter().enumerate() {
        let amount_a = "12.00";
        let amount_b = if case.same_amount { "12.00" } else { "12.01" };
        let db = if case.same_date {
            date_b_same
        } else {
            date_b_near
        };
        let mk = |date, payee: &str, salt: &str, amount: &str| {
            ledgerkit_core::Transaction::new(
                date,
                payee,
                vec![
                    ledgerkit_core::Posting::new(
                        ledgerkit_core::AccountId::new("assets:bank").unwrap(),
                        ledgerkit_core::Amount::parse(amount).unwrap(),
                        ledgerkit_core::Commodity::new("USD").unwrap(),
                    )
                    .with_memo(salt),
                    ledgerkit_core::Posting::new(
                        ledgerkit_core::AccountId::new("expenses:uncategorized").unwrap(),
                        ledgerkit_core::Amount::parse(amount)
                            .unwrap()
                            .checked_neg()
                            .unwrap(),
                        ledgerkit_core::Commodity::new("USD").unwrap(),
                    ),
                ],
            )
            .unwrap()
        };
        let a = mk(date_a, &case.payee_a, &format!("a{i}"), amount_a);
        let b = mk(db, &case.payee_b, &format!("b{i}"), amount_b);
        let report = plan_dedupe(&[a, b], opts);
        let linked = !report.links.is_empty();
        match (linked, case.should_link) {
            (true, true) => tp += 1,
            (true, false) => fp += 1,
            (false, true) => fn_ += 1,
            (false, false) => {}
        }
    }

    let precision = if tp + fp == 0 {
        1.0
    } else {
        tp as f64 / (tp + fp) as f64
    };
    let recall = if tp + fn_ == 0 {
        1.0
    } else {
        tp as f64 / (tp + fn_) as f64
    };
    BinaryMetrics {
        precision,
        recall,
        true_positive: tp,
        false_positive: fp,
        false_negative: fn_,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ledgerkit_core::{AccountId, Amount, Commodity, Posting, Transaction};

    fn tx(date: &str, payee: &str, amt: &str) -> Transaction {
        let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
        let a = Amount::parse(amt).unwrap();
        Transaction::new(
            d,
            payee,
            vec![
                Posting::new(
                    AccountId::new("assets:bank").unwrap(),
                    a,
                    Commodity::new("USD").unwrap(),
                ),
                Posting::new(
                    AccountId::new("expenses:uncategorized").unwrap(),
                    a.checked_neg().unwrap(),
                    Commodity::new("USD").unwrap(),
                ),
            ],
        )
        .unwrap()
    }

    #[test]
    fn exact_duplicate_amazon_spellings() {
        let a = tx("2026-01-02", "AMZN MKTP US*ABC123", "-42.15");
        let b = tx("2026-01-02", "Amazon Marketplace", "-42.15");
        // Different raw payee but same canonical merchant after normalize via narration unset:
        // payee normalize: AMZN MKTP US vs AMAZON MARKETPLACE — not exact.
        // Same date/amount/account but different merchant keys should NOT exact-match.
        let report = plan_dedupe(&[a, b], DedupeOptions { window_days: 1 });
        assert!(
            report.links.is_empty(),
            "conservative: different merchant keys must not auto-merge"
        );
    }

    #[test]
    fn exact_same_fingerprint_links() {
        let a = tx("2026-01-02", "STARBUCKS", "-6.50");
        let b = tx("2026-01-02", "STARBUCKS", "-6.50");
        let report = plan_dedupe(&[a, b], DedupeOptions::default());
        assert_eq!(report.links.len(), 1);
        assert_eq!(report.links[0].strategy, "exact");
    }

    #[test]
    fn near_window_same_merchant() {
        let a = tx("2026-01-02", "NETFLIX.COM", "-15.99");
        let b = tx("2026-01-03", "NETFLIX.COM", "-15.99");
        let report = plan_dedupe(&[a, b], DedupeOptions { window_days: 1 });
        assert_eq!(report.links.len(), 1);
        assert_eq!(report.links[0].strategy, "near_window");
    }

    #[test]
    fn labeled_eval_has_perfect_precision_on_fixture() {
        let cases = vec![
            DedupEvalCase {
                payee_a: "STARBUCKS".into(),
                payee_b: "STARBUCKS".into(),
                same_date: true,
                same_amount: true,
                should_link: true,
            },
            DedupEvalCase {
                payee_a: "STARBUCKS".into(),
                payee_b: "UBER".into(),
                same_date: true,
                same_amount: true,
                should_link: false,
            },
        ];
        let m = evaluate_dedup_cases(&cases, DedupeOptions::default());
        assert_eq!(m.false_positive, 0);
        assert!(m.precision >= 0.99);
        assert!(m.recall >= 0.99);
    }

    #[test]
    fn labeled_fixture_precision() {
        let cases: Vec<DedupEvalCase> =
            serde_json::from_str(include_str!("../../../fixtures/eval/dedup_cases.json")).unwrap();
        let m = evaluate_dedup_cases(&cases, DedupeOptions::default());
        assert_eq!(m.false_positive, 0, "{m:?}");
        assert!(m.precision >= 0.99);
        assert!(m.recall >= 0.99);
    }
}
