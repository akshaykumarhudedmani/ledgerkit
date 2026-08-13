use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::{CoreError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Income,
    Expense,
}

impl AccountType {
    /// Infer type from the first segment of an id (`assets:…` → Asset).
    pub fn from_id_prefix(id: &AccountId) -> Option<Self> {
        match id.as_str().split(':').next()? {
            "asset" | "assets" => Some(Self::Asset),
            "liability" | "liabilities" => Some(Self::Liability),
            "equity" => Some(Self::Equity),
            "income" => Some(Self::Income),
            "expense" | "expenses" => Some(Self::Expense),
            _ => None,
        }
    }
}

/// Hierarchical account id, e.g. `assets:bank:hdfc`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AccountId(String);

impl AccountId {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into().trim().to_ascii_lowercase();
        if name.is_empty()
            || name.starts_with(':')
            || name.ends_with(':')
            || name.contains("::")
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == ':' || c == '_' || c == '-')
        {
            return Err(CoreError::InvalidAccount(name));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: AccountId,
    pub account_type: AccountType,
    pub commodity: crate::money::Commodity,
    pub name: String,
}

impl Account {
    pub fn new(
        id: AccountId,
        account_type: AccountType,
        commodity: crate::money::Commodity,
        name: impl Into<String>,
    ) -> Self {
        Self {
            id,
            account_type,
            commodity,
            name: name.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_hierarchical_names() {
        assert!(AccountId::new("assets:bank:hdfc").is_ok());
    }

    #[test]
    fn rejects_bad_names() {
        assert!(AccountId::new(":assets").is_err());
        assert!(AccountId::new("assets::hdfc").is_err());
        assert!(AccountId::new("").is_err());
    }

    #[test]
    fn type_from_id_prefix() {
        let id = AccountId::new("liabilities:cc:hdfc").unwrap();
        assert_eq!(
            AccountType::from_id_prefix(&id),
            Some(AccountType::Liability)
        );
        let bank = AccountId::new("assets:bank").unwrap();
        assert_eq!(AccountType::from_id_prefix(&bank), Some(AccountType::Asset));
        let weird = AccountId::new("bank:checking").unwrap();
        assert_eq!(AccountType::from_id_prefix(&weird), None);
    }
}
