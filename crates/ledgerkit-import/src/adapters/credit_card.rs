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
                Ok(RawTransaction {
                    row_number: row,
                    date_raw: get(headers, record, "Transaction Date")?.to_string(),
                    amount_raw: get(headers, record, "Amount")?.to_string(),
                    currency_raw: get(headers, record, "Currency").ok().map(str::to_string),
                    description_raw: get(headers, record, "Description")?.to_string(),
                    balance_raw: None,
                    source_refs: vec![format!("cc:row:{row}")],
                })
            },
        )
    }
}
