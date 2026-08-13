use super::hdfc::{get, parse_simple_csv};
use crate::adapter::{AdapterError, AdapterId, BankAdapter, ParseReport};
use crate::raw::RawTransaction;

/// Credit-card style CSV: Transaction Date, Post Date, Description, Amount.
#[derive(Debug, Default, Clone)]
pub struct CreditCardCsvAdapter;

impl BankAdapter for CreditCardCsvAdapter {
    fn id(&self) -> &AdapterId {
        "credit_card"
    }

    fn name(&self) -> &str {
        "Credit Card CSV"
    }

    fn parse(
        &self,
        bytes: &[u8],
    ) -> Result<(crate::raw::RawTransactions, ParseReport), AdapterError> {
        parse_simple_csv(
            self.id(),
            bytes,
            &["Transaction Date", "Description", "Amount"],
            |row, headers, record| {
                let amount = get(headers, record, "Amount", row)?;
                if amount.trim().is_empty() {
                    return Err(AdapterError::Row {
                        row,
                        message: "missing amount".into(),
                    });
                }
                Ok(RawTransaction {
                    row_number: row,
                    date_raw: get(headers, record, "Transaction Date", row)?.to_string(),
                    amount_raw: amount.to_string(),
                    currency_raw: get(headers, record, "Currency", row)
                        .ok()
                        .map(str::to_string),
                    description_raw: get(headers, record, "Description", row)?.to_string(),
                    balance_raw: None,
                    source_refs: vec![format!("cc:row:{row}")],
                })
            },
        )
    }
}
