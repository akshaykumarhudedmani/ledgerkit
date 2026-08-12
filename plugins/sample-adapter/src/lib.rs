//! Example external adapter crate.
//!
//! Real third-party plugins would depend on published `ledgerkit-import`
//! traits. This in-repo sample proves the extension surface exists.

use ledgerkit_import::adapters::GenericCsvAdapter;
use ledgerkit_import::BankAdapter;

pub fn sample_adapter() -> impl BankAdapter {
    GenericCsvAdapter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_id() {
        assert_eq!(sample_adapter().id(), "generic_csv");
    }
}
