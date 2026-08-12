//! Scale / benchmark helpers. 100k-row import is ignored in default CI.

use std::time::Instant;

use ledgerkit_core::{AccountId, Commodity, ImportBatchId};
use ledgerkit_import::adapters::GenericCsvAdapter;
use ledgerkit_import::{convert_raw, BankAdapter, ConvertOptions};

fn generic_csv(n: usize) -> String {
    let mut out = String::from("Date,Description,Amount,Currency\n");
    for i in 0..n {
        let day = (i % 28) + 1;
        out.push_str(&format!(
            "2026-01-{day:02},MERCHANT {i},-{}.{:02},USD\n",
            1 + (i % 90),
            i % 100
        ));
    }
    out
}

#[test]
fn import_1k_rows_parses_and_converts() {
    let csv = generic_csv(1_000);
    let (raw, report) = GenericCsvAdapter.parse(csv.as_bytes()).unwrap();
    assert_eq!(report.error_rows, 0);
    assert_eq!(raw.transactions.len(), 1_000);
    let converted = convert_raw(
        &raw,
        &ConvertOptions {
            bank_account: AccountId::new("assets:bank").unwrap(),
            expense_account: AccountId::new("expenses:uncategorized").unwrap(),
            income_account: AccountId::new("income:uncategorized").unwrap(),
            default_commodity: Commodity::new("USD").unwrap(),
            import_batch_id: ImportBatchId::new(),
        },
    );
    assert!(converted.errors.is_empty());
    assert_eq!(converted.transactions.len(), 1_000);
}

/// Run with: `cargo test -p ledgerkit-import --test scale -- --ignored --nocapture`
#[test]
#[ignore]
fn import_100k_rows_bench() {
    let csv = generic_csv(100_000);
    let t0 = Instant::now();
    let (raw, report) = GenericCsvAdapter.parse(csv.as_bytes()).unwrap();
    let parse_ms = t0.elapsed().as_millis();
    assert_eq!(report.error_rows, 0);
    assert_eq!(raw.transactions.len(), 100_000);

    let t1 = Instant::now();
    let converted = convert_raw(
        &raw,
        &ConvertOptions {
            bank_account: AccountId::new("assets:bank").unwrap(),
            expense_account: AccountId::new("expenses:uncategorized").unwrap(),
            income_account: AccountId::new("income:uncategorized").unwrap(),
            default_commodity: Commodity::new("USD").unwrap(),
            import_batch_id: ImportBatchId::new(),
        },
    );
    let convert_ms = t1.elapsed().as_millis();
    assert!(converted.errors.is_empty());
    assert_eq!(converted.transactions.len(), 100_000);
    eprintln!("parse_100k_ms={parse_ms} convert_100k_ms={convert_ms}");
}
