use regex::Regex;
use serde::{Deserialize, Serialize};

/// Deterministic merchant cleaning. Fuzzy merge is suggestion-only in v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedMerchant {
    pub canonical_key: String,
    pub display_name: String,
    pub confidence: u8,
    pub reasons: Vec<String>,
}

pub fn normalize_merchant(raw: &str) -> NormalizedMerchant {
    let mut reasons = Vec::new();
    let upper = raw.trim().to_uppercase();
    reasons.push("trim+uppercase".into());

    // Strip common payment processor noise tokens.
    let collapsed = Regex::new(r"\s+")
        .unwrap()
        .replace_all(&upper, " ")
        .to_string();
    if collapsed != upper {
        reasons.push("collapse_whitespace".into());
    }

    // Only strip processor-style refs (`*ABC123`), never a whole merchant name.
    let stripped = Regex::new(r"(\*|\#)[A-Z0-9]{4,}$")
        .unwrap()
        .replace(&collapsed, "")
        .trim()
        .to_string();
    let without_refs = if stripped.is_empty() {
        collapsed.clone()
    } else {
        if stripped != collapsed {
            reasons.push("strip_trailing_ref".into());
        }
        stripped
    };

    let key = without_refs
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
        .to_lowercase();

    reasons.push("alnum_underscore_key".into());

    NormalizedMerchant {
        display_name: without_refs,
        canonical_key: key,
        confidence: 90,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn cleans_amazon_marketplace_noise() {
        let n = normalize_merchant("AMZN MKTP US*ABC123");
        assert_eq!(n.canonical_key, "amzn_mktp_us");
        assert!(n.confidence >= 80);
        assert_eq!(normalize_merchant("STARBUCKS").canonical_key, "starbucks");
    }
}
