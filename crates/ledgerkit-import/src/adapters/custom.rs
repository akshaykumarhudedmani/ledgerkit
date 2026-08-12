use serde::{Deserialize, Serialize};

use super::hdfc::parse_simple_csv;
use crate::adapter::{AdapterError, AdapterId, BankAdapter, ParseReport};
use crate::raw::RawTransaction;

/// User-defined column mapping adapter (YAML/JSON config later).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMappingAdapter {
    pub date_column: String,
    pub description_column: String,
    pub amount_column: String,
    pub currency_column: Option<String>,
    pub balance_column: Option<String>,
}

impl Default for CustomMappingAdapter {
    fn default() -> Self {
        Self {
            date_column: "date".into(),
            description_column: "description".into(),
            amount_column: "amount".into(),
            currency_column: Some("currency".into()),
            balance_column: None,
        }
    }
}

impl BankAdapter for CustomMappingAdapter {
    fn id(&self) -> &AdapterId {
        "custom"
    }

    fn name(&self) -> &str {
        "Custom Mapping CSV"
    }

    fn parse(
        &self,
        bytes: &[u8],
    ) -> Result<(crate::raw::RawTransactions, ParseReport), AdapterError> {
        let required_owned = [
            self.date_column.clone(),
            self.description_column.clone(),
            self.amount_column.clone(),
        ];
        let required: Vec<&str> = required_owned.iter().map(String::as_str).collect();
        let mapping = self.clone();

        parse_simple_csv(self.id(), bytes, &required, move |row, headers, record| {
            let get_col = |name: &str| -> Result<String, AdapterError> {
                let idx = headers
                    .iter()
                    .position(|h| h.eq_ignore_ascii_case(name))
                    .ok_or_else(|| AdapterError::Schema(format!("column {name} not found")))?;
                record
                    .get(idx)
                    .map(str::to_string)
                    .ok_or_else(|| AdapterError::Row {
                        row,
                        message: format!("missing field {name}"),
                    })
            };

            Ok(RawTransaction {
                row_number: row,
                date_raw: get_col(&mapping.date_column)?,
                amount_raw: get_col(&mapping.amount_column)?,
                currency_raw: mapping
                    .currency_column
                    .as_ref()
                    .and_then(|c| get_col(c).ok()),
                description_raw: get_col(&mapping.description_column)?,
                balance_raw: mapping
                    .balance_column
                    .as_ref()
                    .and_then(|c| get_col(c).ok()),
                source_refs: vec![format!("custom:row:{row}")],
            })
        })
    }
}
