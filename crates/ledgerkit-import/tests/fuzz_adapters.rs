//! Property / fuzz tests: adapters must not panic on garbage input.

use ledgerkit_import::adapters::{
    CreditCardCsvAdapter, CustomMappingAdapter, GenericCsvAdapter, HdfcCsvAdapter,
};
use ledgerkit_import::BankAdapter;
use proptest::prelude::*;

fn parse_all(bytes: &[u8]) {
    let _ = GenericCsvAdapter.parse(bytes);
    let _ = HdfcCsvAdapter.parse(bytes);
    let _ = CreditCardCsvAdapter.parse(bytes);
    let _ = CustomMappingAdapter::default().parse(bytes);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn adapters_do_not_panic_on_random_bytes(bytes in prop::collection::vec(any::<u8>(), 0..2048)) {
        parse_all(&bytes);
    }

    #[test]
    fn adapters_do_not_panic_on_random_utf8(s in ".{0,512}") {
        parse_all(s.as_bytes());
    }

    #[test]
    fn generic_csv_never_silently_drops_utf8_rows(body in "[\x20-\x7e\n,]{0,400}") {
        let mut csv = String::from("Date,Description,Amount,Currency\n");
        csv.push_str(&body);
        let result = GenericCsvAdapter.parse(csv.as_bytes());
        if let Ok((raw, report)) = result {
            prop_assert_eq!(
                raw.transactions.len(),
                report.ok_rows,
                "ok_rows must equal converted raw rows"
            );
            prop_assert_eq!(
                report.ok_rows + report.error_rows,
                raw.transactions.len() + report.errors.len()
            );
        }
    }
}
