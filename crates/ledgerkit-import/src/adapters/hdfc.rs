use crate::adapter::{AdapterError, AdapterId, BankAdapter, ParseReport};
use crate::raw::{RawTransaction, RawTransactions};

/// HDFC Bank account statement CSV (India) — Phase 3 fills real column mapping.
#[derive(Debug, Default, Clone)]
pub struct HdfcCsvAdapter;

impl BankAdapter for HdfcCsvAdapter {
    fn id(&self) -> &AdapterId {
        "hdfc"
    }

    fn name(&self) -> &str {
        "HDFC Bank CSV"
    }

    fn parse(&self, bytes: &[u8]) -> Result<(RawTransactions, ParseReport), AdapterError> {
        parse_simple_csv(
            self.id(),
            bytes,
            &["Date", "Narration", "Withdrawal Amt.", "Deposit Amt."],
            |row, headers, record| {
                let date = get(headers, record, "Date")?;
                let narration = get(headers, record, "Narration")?;
                let withdrawal = get(headers, record, "Withdrawal Amt.").unwrap_or("");
                let deposit = get(headers, record, "Deposit Amt.").unwrap_or("");
                let amount = if !deposit.trim().is_empty() {
                    deposit.trim().to_string()
                } else if !withdrawal.trim().is_empty() {
                    format!("-{}", withdrawal.trim())
                } else {
                    return Err(AdapterError::Row {
                        row,
                        message: "missing amount".into(),
                    });
                };
                Ok(RawTransaction {
                    row_number: row,
                    date_raw: date.to_string(),
                    amount_raw: amount,
                    currency_raw: Some("INR".into()),
                    description_raw: narration.to_string(),
                    balance_raw: get(headers, record, "Closing Balance")
                        .ok()
                        .map(str::to_string),
                    source_refs: vec![format!("hdfc:row:{row}")],
                })
            },
        )
    }
}

pub(crate) fn parse_simple_csv<F>(
    adapter_id: &str,
    bytes: &[u8],
    required: &[&str],
    mut map_row: F,
) -> Result<(RawTransactions, ParseReport), AdapterError>
where
    F: FnMut(usize, &[String], &csv::StringRecord) -> Result<RawTransaction, AdapterError>,
{
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(bytes);

    let headers: Vec<String> = reader
        .headers()?
        .iter()
        .map(|h| h.trim().to_string())
        .collect();

    for col in required {
        if !headers.iter().any(|h| h.eq_ignore_ascii_case(col)) {
            return Err(AdapterError::Schema(format!(
                "missing required column '{col}' (have: {})",
                headers.join(", ")
            )));
        }
    }

    let mut transactions = Vec::new();
    let mut errors = Vec::new();
    for (idx, record) in reader.records().enumerate() {
        let row = idx + 2; // header is row 1
        match record {
            Ok(rec) => match map_row(row, &headers, &rec) {
                Ok(tx) => transactions.push(tx),
                Err(err) => errors.push(err.to_string()),
            },
            Err(err) => errors.push(format!("row {row}: {err}")),
        }
    }

    let report = ParseReport {
        ok_rows: transactions.len(),
        error_rows: errors.len(),
        errors,
    };

    Ok((
        RawTransactions {
            adapter_id: adapter_id.to_string(),
            transactions,
        },
        report,
    ))
}

pub(crate) fn get<'a>(
    headers: &[String],
    record: &'a csv::StringRecord,
    name: &str,
) -> Result<&'a str, AdapterError> {
    let idx = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(name))
        .ok_or_else(|| AdapterError::Schema(format!("column {name} not found")))?;
    record.get(idx).ok_or_else(|| AdapterError::Row {
        row: 0,
        message: format!("missing field {name}"),
    })
}
