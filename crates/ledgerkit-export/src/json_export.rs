use ledgerkit_core::LedgerSnapshot;

use crate::beancount::active_transactions;
use crate::{ExportError, Exporter};

#[derive(Debug, Default, Clone)]
pub struct JsonExporter;

impl Exporter for JsonExporter {
    fn id(&self) -> &'static str {
        "json"
    }

    fn export(&self, snapshot: &LedgerSnapshot) -> Result<String, ExportError> {
        let mut ordered = LedgerSnapshot {
            transactions: active_transactions(snapshot).into_iter().cloned().collect(),
        };
        // Keep duplicates out of JSON export so balances match Beancount/CSV.
        ordered.transactions.sort_by(|a, b| {
            a.date
                .cmp(&b.date)
                .then_with(|| a.id.to_string().cmp(&b.id.to_string()))
        });
        Ok(serde_json::to_string_pretty(&ordered)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ledgerkit_core::{AccountId, Amount, Commodity, LedgerSnapshot, Transaction};

    #[test]
    fn json_omits_duplicates_and_is_stable() {
        let a = Transaction::transfer(
            chrono::NaiveDate::from_ymd_opt(2026, 1, 3).unwrap(),
            AccountId::new("assets:bank").unwrap(),
            AccountId::new("expenses:food").unwrap(),
            Amount::parse("6.50").unwrap(),
            Commodity::new("USD").unwrap(),
            "Cafe",
        )
        .unwrap();
        let mut b = a.clone();
        b.id = ledgerkit_core::TransactionId::new();
        b.duplicate_of = Some(a.id);
        let dup_id = b.id;
        let snap = LedgerSnapshot {
            transactions: vec![b, a.clone()],
        };
        let json = JsonExporter.export(&snap).unwrap();
        assert!(json.contains(&a.id.to_string()));
        assert!(!json.contains(&dup_id.to_string()));
        assert_eq!(json, JsonExporter.export(&snap).unwrap());
    }
}
