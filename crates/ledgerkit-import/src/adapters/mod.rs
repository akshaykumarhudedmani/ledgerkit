//! Built-in adapters (v1 targets).

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
