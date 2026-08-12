use ledgerkit_core::LedgerSnapshot;

use crate::{ExportError, Exporter};

#[derive(Debug, Default, Clone)]
pub struct JsonExporter;

impl Exporter for JsonExporter {
    fn id(&self) -> &'static str {
        "json"
    }

    fn export(&self, snapshot: &LedgerSnapshot) -> Result<String, ExportError> {
        Ok(serde_json::to_string_pretty(snapshot)?)
    }
}
