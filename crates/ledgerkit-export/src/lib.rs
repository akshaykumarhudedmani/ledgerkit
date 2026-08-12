//! Export plugins for LedgerKit.

mod beancount;
mod json_export;

pub use beancount::BeancountExporter;
pub use json_export::JsonExporter;

use ledgerkit_core::LedgerSnapshot;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialize error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub trait Exporter {
    fn id(&self) -> &'static str;
    fn export(&self, snapshot: &LedgerSnapshot) -> Result<String, ExportError>;
}
