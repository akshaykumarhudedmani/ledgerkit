use ledgerkit_core::{AccountId, Amount, Commodity, ImportBatchId, Posting, Transaction};

use crate::dates::parse_date;
use crate::normalize::normalize_merchant;
use crate::raw::RawTransactions;

pub struct ConvertOptions {
    pub bank_account: AccountId,
    pub expense_account: AccountId,
    pub income_account: AccountId,
    pub default_commodity: Commodity,
    pub import_batch_id: ImportBatchId,
}

#[derive(Debug, Clone, Default)]
pub struct ConvertReport {
    pub transactions: Vec<Transaction>,
    pub errors: Vec<String>,
}

/// Turn adapter output into balanced double-entry transactions.
/// Failed rows are recorded in `errors` and never silently dropped.
pub fn convert_raw(raw: &RawTransactions, opts: &ConvertOptions) -> ConvertReport {
    let mut report = ConvertReport::default();
    for row in &raw.transactions {
        match convert_one(row, opts) {
            Ok(tx) => report.transactions.push(tx),
            Err(err) => report.errors.push(format!("row {}: {err}", row.row_number)),
        }
    }
    report
}

fn convert_one(
    row: &crate::raw::RawTransaction,
    opts: &ConvertOptions,
) -> Result<Transaction, String> {
    let date = parse_date(&row.date_raw).map_err(|e| e.to_string())?;
    let amount = Amount::parse(&row.amount_raw).map_err(|e| e.to_string())?;
    if amount.is_zero() {
        return Err("amount is zero".into());
    }
    let commodity = match &row.currency_raw {
        Some(code) if !code.trim().is_empty() => {
            Commodity::new(code.trim()).map_err(|e| e.to_string())?
        }
        _ => opts.default_commodity.clone(),
    };

    let merchant = normalize_merchant(&row.description_raw);
    let offset_account = if amount.decimal().is_sign_negative() {
        opts.expense_account.clone()
    } else {
        opts.income_account.clone()
    };
    let offset = amount
        .checked_neg()
        .ok_or_else(|| "amount negation overflow".to_string())?;

    let mut tx = Transaction::new(
        date,
        row.description_raw.trim(),
        vec![
            Posting::new(opts.bank_account.clone(), amount, commodity.clone())
                .with_memo(row.source_refs.join(",")),
            Posting::new(offset_account, offset, commodity),
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.import_batch_id = Some(opts.import_batch_id);
    tx.narration = Some(merchant.canonical_key);
    Ok(tx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::RawTransaction;

    #[test]
    fn withdrawal_hits_expense_offset() {
        let raw = RawTransactions {
            adapter_id: "generic_csv".into(),
            transactions: vec![RawTransaction {
                row_number: 2,
                date_raw: "2026-01-02".into(),
                amount_raw: "-42.15".into(),
                currency_raw: Some("USD".into()),
                description_raw: "AMZN MKTP US*ABC123".into(),
                balance_raw: None,
                source_refs: vec!["generic:row:2".into()],
            }],
        };
        let opts = ConvertOptions {
            bank_account: AccountId::new("assets:bank:checking").unwrap(),
            expense_account: AccountId::new("expenses:uncategorized").unwrap(),
            income_account: AccountId::new("income:uncategorized").unwrap(),
            default_commodity: Commodity::new("USD").unwrap(),
            import_batch_id: ImportBatchId::new(),
        };
        let report = convert_raw(&raw, &opts);
        assert!(report.errors.is_empty());
        assert_eq!(report.transactions.len(), 1);
        let tx = &report.transactions[0];
        assert_eq!(tx.postings[0].account.as_str(), "assets:bank:checking");
        assert_eq!(tx.postings[0].amount.to_string(), "-42.15");
        assert_eq!(tx.postings[1].account.as_str(), "expenses:uncategorized");
        assert_eq!(tx.narration.as_deref(), Some("amzn_mktp_us"));
    }

    #[test]
    fn bad_date_is_reported_not_dropped_silently() {
        let raw = RawTransactions {
            adapter_id: "generic_csv".into(),
            transactions: vec![RawTransaction {
                row_number: 3,
                date_raw: "not-a-date".into(),
                amount_raw: "10".into(),
                currency_raw: None,
                description_raw: "x".into(),
                balance_raw: None,
                source_refs: vec![],
            }],
        };
        let opts = ConvertOptions {
            bank_account: AccountId::new("assets:bank").unwrap(),
            expense_account: AccountId::new("expenses:uncategorized").unwrap(),
            income_account: AccountId::new("income:uncategorized").unwrap(),
            default_commodity: Commodity::new("INR").unwrap(),
            import_batch_id: ImportBatchId::new(),
        };
        let report = convert_raw(&raw, &opts);
        assert_eq!(report.transactions.len(), 0);
        assert_eq!(report.errors.len(), 1);
    }
}
