use ledgerkit_core::LedgerSnapshot;

use crate::beancount::active_transactions;
use crate::{ExportError, Exporter};

/// Canonical posting CSV (duplicates omitted). Deterministic row order.
#[derive(Debug, Default, Clone)]
pub struct CsvExporter;

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

impl Exporter for CsvExporter {
    fn id(&self) -> &'static str {
        "csv"
    }

    fn export(&self, snapshot: &LedgerSnapshot) -> Result<String, ExportError> {
        let mut out = String::from("date,tx_id,payee,account,amount,commodity,memo\n");
        for tx in active_transactions(snapshot) {
            for posting in &tx.postings {
                out.push_str(&format!(
                    "{},{},{},{},{},{},{}\n",
                    tx.date.format("%Y-%m-%d"),
                    csv_field(&tx.id.to_string()),
                    csv_field(&tx.payee),
                    csv_field(posting.account.as_str()),
                    posting.amount,
                    posting.commodity,
                    csv_field(posting.memo.as_deref().unwrap_or(""))
                ));
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ledgerkit_core::{AccountId, Amount, Commodity, LedgerSnapshot, Transaction};

    #[test]
    fn csv_skips_duplicates_and_quotes_commas() {
        let mut a = Transaction::transfer(
            chrono::NaiveDate::from_ymd_opt(2026, 1, 3).unwrap(),
            AccountId::new("assets:bank").unwrap(),
            AccountId::new("expenses:food").unwrap(),
            Amount::parse("6.50").unwrap(),
            Commodity::new("USD").unwrap(),
            "Cafe, Inc",
        )
        .unwrap();
        a.postings[0] = a.postings[0].clone().with_memo("note");
        let mut b = a.clone();
        b.id = ledgerkit_core::TransactionId::new();
        b.duplicate_of = Some(a.id);
        let snap = LedgerSnapshot {
            transactions: vec![a, b],
        };
        let csv = CsvExporter.export(&snap).unwrap();
        assert!(csv.starts_with("date,tx_id,payee,account,amount,commodity,memo\n"));
        assert_eq!(csv.lines().count(), 3); // header + 2 postings
        assert!(csv.contains("\"Cafe, Inc\""));
        assert_eq!(csv, CsvExporter.export(&snap).unwrap());
    }
}
