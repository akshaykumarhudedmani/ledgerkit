use ledgerkit_core::{Amount, Transaction, TransactionId};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    #[serde(default)]
    pub priority: i32,
    pub category: String,
    #[serde(default)]
    pub merchant_regex: Option<String>,
    #[serde(default)]
    pub payee_regex: Option<String>,
    #[serde(default)]
    pub account_regex: Option<String>,
    #[serde(default)]
    pub min_amount: Option<String>,
    #[serde(default)]
    pub max_amount: Option<String>,
    #[serde(default = "default_confidence")]
    pub confidence: u8,
}

fn default_confidence() -> u8 {
    80
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMatch {
    pub transaction_id: TransactionId,
    pub category: String,
    pub rule_id: String,
    pub confidence: u8,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleReport {
    pub applied: Vec<RuleMatch>,
    pub conflicts: Vec<String>,
    pub unmatched: usize,
    pub skipped_already_categorized: usize,
}

impl RuleSet {
    pub fn from_yaml(text: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(text)
    }

    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    pub fn from_path(path: &std::path::Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("yaml")
            .to_ascii_lowercase();
        if ext == "json" {
            Ok(Self::from_json(&text)?)
        } else {
            Ok(Self::from_yaml(&text)?)
        }
    }
}

fn already_categorized(tx: &Transaction) -> bool {
    tx.tags.iter().any(|t| t.starts_with("category:"))
}

fn primary_abs_amount(tx: &Transaction) -> Option<Amount> {
    let p = tx.postings.first()?;
    Some(Amount::from_decimal(p.amount.decimal().abs()))
}

fn merchant_haystack(tx: &Transaction) -> String {
    tx.narration
        .clone()
        .unwrap_or_else(|| tx.payee.to_lowercase())
}

fn matches_rule<'a>(tx: &'a Transaction, rule: &'a Rule) -> Option<Vec<String>> {
    let has_pred = rule.merchant_regex.is_some()
        || rule.payee_regex.is_some()
        || rule.account_regex.is_some()
        || rule.min_amount.is_some()
        || rule.max_amount.is_some();
    if !has_pred {
        return None;
    }

    let mut reasons = Vec::new();

    if let Some(pat) = &rule.merchant_regex {
        let re = Regex::new(pat).ok()?;
        let hay = merchant_haystack(tx);
        if !re.is_match(&hay) {
            return None;
        }
        reasons.push(format!("merchant_regex={pat}"));
    }
    if let Some(pat) = &rule.payee_regex {
        let re = Regex::new(pat).ok()?;
        if !re.is_match(&tx.payee) {
            return None;
        }
        reasons.push(format!("payee_regex={pat}"));
    }
    if let Some(pat) = &rule.account_regex {
        let re = Regex::new(pat).ok()?;
        let ok = tx.postings.iter().any(|p| re.is_match(p.account.as_str()));
        if !ok {
            return None;
        }
        reasons.push(format!("account_regex={pat}"));
    }
    if let Some(min) = &rule.min_amount {
        let min_a = Amount::parse(min).ok()?;
        let amt = primary_abs_amount(tx)?;
        if amt < min_a {
            return None;
        }
        reasons.push(format!("min_amount={min}"));
    }
    if let Some(max) = &rule.max_amount {
        let max_a = Amount::parse(max).ok()?;
        let amt = primary_abs_amount(tx)?;
        if amt > max_a {
            return None;
        }
        reasons.push(format!("max_amount={max}"));
    }

    Some(reasons)
}

/// Apply rules. Higher `priority` wins. Same priority + different category → conflict (no apply).
pub fn apply_rules(txs: &[Transaction], set: &RuleSet) -> RuleReport {
    let mut report = RuleReport::default();
    for tx in txs {
        if tx.duplicate_of.is_some() {
            continue;
        }
        if already_categorized(tx) {
            report.skipped_already_categorized += 1;
            continue;
        }

        let mut hits: Vec<(&Rule, Vec<String>)> = Vec::new();
        for rule in &set.rules {
            if let Some(reasons) = matches_rule(tx, rule) {
                hits.push((rule, reasons));
            }
        }
        if hits.is_empty() {
            report.unmatched += 1;
            continue;
        }
        let max_p = hits.iter().map(|(r, _)| r.priority).max().unwrap();
        let top: Vec<_> = hits
            .into_iter()
            .filter(|(r, _)| r.priority == max_p)
            .collect();
        let categories: std::collections::BTreeSet<_> =
            top.iter().map(|(r, _)| r.category.as_str()).collect();
        if categories.len() > 1 {
            report.conflicts.push(format!(
                "tx {} priority {max_p} conflict: {}",
                tx.id,
                top.iter()
                    .map(|(r, _)| format!("{}=>{}", r.id, r.category))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            continue;
        }
        let (rule, reasons) = &top[0];
        report.applied.push(RuleMatch {
            transaction_id: tx.id,
            category: rule.category.clone(),
            rule_id: rule.id.clone(),
            confidence: rule.confidence,
            reasons: reasons.clone(),
        });
    }
    report
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEvalCase {
    pub payee: String,
    pub amount: String,
    pub expected_category: Option<String>,
}

pub fn evaluate_rules(cases: &[RuleEvalCase], set: &RuleSet) -> crate::dedupe::BinaryMetrics {
    use crate::dedupe::BinaryMetrics;
    let date = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let mut tp = 0;
    let mut fp = 0;
    let mut fn_ = 0;
    for case in cases {
        let amt = Amount::parse(&case.amount).unwrap();
        let tx = Transaction::new(
            date,
            &case.payee,
            vec![
                ledgerkit_core::Posting::new(
                    ledgerkit_core::AccountId::new("assets:bank").unwrap(),
                    amt,
                    ledgerkit_core::Commodity::new("USD").unwrap(),
                ),
                ledgerkit_core::Posting::new(
                    ledgerkit_core::AccountId::new("expenses:uncategorized").unwrap(),
                    amt.checked_neg().unwrap(),
                    ledgerkit_core::Commodity::new("USD").unwrap(),
                ),
            ],
        )
        .unwrap();
        let report = apply_rules(&[tx], set);
        let got = report.applied.first().map(|m| m.category.as_str());
        match (got, case.expected_category.as_deref()) {
            (Some(g), Some(e)) if g == e => tp += 1,
            (Some(_), Some(_)) => fp += 1,
            (Some(_), None) => fp += 1,
            (None, Some(_)) => fn_ += 1,
            (None, None) => {}
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
    use ledgerkit_core::{AccountId, Commodity, Posting};

    fn tx(payee: &str, amt: &str) -> Transaction {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();
        let a = Amount::parse(amt).unwrap();
        Transaction::new(
            date,
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
    fn amazon_rule_applies() {
        let set = RuleSet::from_yaml(
            r#"
rules:
  - id: amazon
    priority: 10
    category: expenses:shopping
    merchant_regex: "(?i)amzn|amazon"
"#,
        )
        .unwrap();
        let t = tx("AMZN MKTP US*ABC", "-42.15");
        let report = apply_rules(&[t], &set);
        assert_eq!(report.applied.len(), 1);
        assert_eq!(report.applied[0].category, "expenses:shopping");
        assert!(report.conflicts.is_empty());
    }

    #[test]
    fn same_priority_conflict_is_reported() {
        let set = RuleSet::from_yaml(
            r#"
rules:
  - id: a
    priority: 5
    category: expenses:a
    payee_regex: "FOO"
  - id: b
    priority: 5
    category: expenses:b
    payee_regex: "FOO"
"#,
        )
        .unwrap();
        let report = apply_rules(&[tx("FOO BAR", "-1")], &set);
        assert!(report.applied.is_empty());
        assert_eq!(report.conflicts.len(), 1);
    }

    #[test]
    fn labeled_fixture_accuracy() {
        let set = RuleSet::from_yaml(include_str!("../../../fixtures/rules/default.yaml")).unwrap();
        let cases: Vec<RuleEvalCase> =
            serde_json::from_str(include_str!("../../../fixtures/eval/rules_cases.json")).unwrap();
        let m = evaluate_rules(&cases, &set);
        assert_eq!(m.false_positive, 0, "{m:?}");
        assert!(m.precision >= 0.99);
        assert!(m.recall >= 0.99);
    }
}
