//! Built-in adapters.

mod credit_card;
mod custom;
mod generic_csv;
mod hdfc;

pub use credit_card::CreditCardCsvAdapter;
pub use custom::CustomMappingAdapter;
pub use generic_csv::GenericCsvAdapter;
pub use hdfc::HdfcCsvAdapter;

use crate::adapter::BankAdapter;

/// Resolve a built-in adapter by id.
pub fn builtin(id: &str) -> Option<Box<dyn BankAdapter>> {
    match id {
        "hdfc" => Some(Box::new(HdfcCsvAdapter)),
        "generic" | "generic_csv" => Some(Box::new(GenericCsvAdapter)),
        "credit_card" | "cc" => Some(Box::new(CreditCardCsvAdapter)),
        "custom" => Some(Box::new(CustomMappingAdapter::default())),
        _ => None,
    }
}

pub fn list_builtin() -> Vec<&'static str> {
    vec!["hdfc", "generic_csv", "credit_card", "custom"]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BankAdapter;

    #[test]
    fn generic_and_cc_fixtures_parse() {
        let (raw, report) = GenericCsvAdapter
            .parse(include_bytes!(
                "../../../../fixtures/csv/generic/sample.csv"
            ))
            .unwrap();
        assert_eq!(report.error_rows, 0);
        assert_eq!(raw.transactions.len(), 4);

        let (raw, report) = CreditCardCsvAdapter
            .parse(include_bytes!(
                "../../../../fixtures/csv/credit_card/sample.csv"
            ))
            .unwrap();
        assert_eq!(report.error_rows, 0);
        assert_eq!(raw.transactions.len(), 3);
    }

    #[test]
    fn missing_amount_is_reported() {
        let csv = b"Date,Description,Amount\n2026-01-01,Good,10.00\n2026-01-02,Bad,\n";
        let (raw, report) = GenericCsvAdapter.parse(csv).unwrap();
        assert_eq!(raw.transactions.len(), 1);
        assert_eq!(report.error_rows, 1);
        assert!(report.errors[0].contains("missing amount"));
    }

    #[test]
    fn missing_column_is_schema_error() {
        let csv = b"Date,Description\n2026-01-01,Nope\n";
        let err = GenericCsvAdapter.parse(csv).unwrap_err();
        assert!(matches!(err, crate::AdapterError::Schema(_)));
    }

    #[test]
    fn golden_parse_counts() {
        let golden: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../fixtures/golden/parse_counts.json"
        ))
        .unwrap();
        let cases: &[(&str, &[u8])] = &[
            (
                "generic_csv",
                include_bytes!("../../../../fixtures/csv/generic/sample.csv"),
            ),
            (
                "hdfc",
                include_bytes!("../../../../fixtures/csv/hdfc/sample.csv"),
            ),
            (
                "credit_card",
                include_bytes!("../../../../fixtures/csv/credit_card/sample.csv"),
            ),
            (
                "generic_malformed",
                include_bytes!("../../../../fixtures/csv/generic/malformed.csv"),
            ),
        ];
        for (name, bytes) in cases {
            let adapter_id = if *name == "generic_malformed" {
                "generic_csv"
            } else {
                *name
            };
            let adapter = builtin(adapter_id).unwrap();
            let (_raw, report) = adapter.parse(bytes).unwrap();
            let exp = &golden[*name];
            assert_eq!(
                report.ok_rows,
                exp["ok_rows"].as_u64().unwrap() as usize,
                "{name} ok_rows"
            );
            assert_eq!(
                report.error_rows,
                exp["error_rows"].as_u64().unwrap() as usize,
                "{name} error_rows"
            );
        }
    }

    #[test]
    fn custom_adapter_reads_generic_headers() {
        let (raw, report) = CustomMappingAdapter::default()
            .parse(include_bytes!(
                "../../../../fixtures/csv/generic/sample.csv"
            ))
            .unwrap();
        assert_eq!(report.error_rows, 0);
        assert_eq!(raw.transactions.len(), 4);
        assert_eq!(raw.adapter_id, "custom");
    }
}
