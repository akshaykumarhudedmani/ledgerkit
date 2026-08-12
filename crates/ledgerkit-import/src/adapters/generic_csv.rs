use super::hdfc::{get, parse_simple_csv};
use crate::adapter::{AdapterError, AdapterId, BankAdapter, ParseReport};
use crate::raw::RawTransaction;

/// Generic US/EU-style CSV: Date, Description, Amount, [Currency], [Balance].
#[derive(Debug, Default, Clone)]
pub struct GenericCsvAdapter;

impl BankAdapter for GenericCsvAdapter {
    fn id(&self) -> &AdapterId {
        "generic_csv"
    }

    fn name(&self) -> &str {
        "Generic CSV"
    }

    fn parse(
        &self,
        bytes: &[u8],
    ) -> Result<(crate::raw::RawTransactions, ParseReport), AdapterError> {
        parse_simple_csv(
            self.id(),
            bytes,
            &["Date", "Description", "Amount"],
            |row, headers, record| {
                Ok(RawTransaction {
                    row_number: row,
                    date_raw: get(headers, record, "Date")?.to_string(),
                    amount_raw: get(headers, record, "Amount")?.to_string(),
                    currency_raw: get(headers, record, "Currency").ok().map(str::to_string),
                    description_raw: get(headers, record, "Description")?.to_string(),
                    balance_raw: get(headers, record, "Balance").ok().map(str::to_string),
                    source_refs: vec![format!("generic:row:{row}")],
                })
            },
        )
    }
}
