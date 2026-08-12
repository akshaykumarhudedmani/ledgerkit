//! Path hygiene for write destinations (Phase 7).

use std::path::{Component, Path};

use anyhow::{bail, Result};

/// Reject any path whose components include `..`.
pub fn reject_parent_dir(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        bail!("empty path rejected");
    }
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        bail!("path traversal rejected: {}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn allows_relative_and_absolute_without_dotdot() {
        reject_parent_dir(Path::new("ledger.bean")).unwrap();
        reject_parent_dir(Path::new(".demo/ledger.bean")).unwrap();
        reject_parent_dir(Path::new("reports/out.csv")).unwrap();
    }

    #[test]
    fn rejects_parent_dir_components() {
        assert!(reject_parent_dir(Path::new("../ledger.bean")).is_err());
        assert!(reject_parent_dir(Path::new("foo/../../etc/passwd")).is_err());
        assert!(reject_parent_dir(Path::new("..")).is_err());
    }
}
