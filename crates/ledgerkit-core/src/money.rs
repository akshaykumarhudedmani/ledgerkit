use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::{CoreError, Result};

/// ISO-ish commodity/currency code (e.g. INR, USD). Never empty.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Commodity(String);

impl Commodity {
    pub fn new(code: impl Into<String>) -> Result<Self> {
        let code = code.into().trim().to_uppercase();
        if code.is_empty() || code.len() > 16 {
            return Err(CoreError::InvalidAmount(format!(
                "invalid commodity code: {code:?}"
            )));
        }
        Ok(Self(code))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Commodity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Signed minor-unit-safe amount using [`Decimal`] (no floats).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Amount(Decimal);

impl Amount {
    pub fn zero() -> Self {
        Self(Decimal::ZERO)
    }

    pub fn from_decimal(value: Decimal) -> Self {
        Self(value)
    }

    pub fn parse(s: &str) -> Result<Self> {
        let cleaned = s.trim().replace(',', "");
        Decimal::from_str(&cleaned)
            .map(Self)
            .map_err(|e| CoreError::InvalidAmount(format!("{s}: {e}")))
    }

    pub fn decimal(self) -> Decimal {
        self.0
    }

    pub fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    pub fn checked_neg(self) -> Option<Self> {
        self.0.checked_mul(Decimal::NEGATIVE_ONE).map(Self)
    }

    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Amount tagged with commodity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount: Amount,
    pub commodity: Commodity,
}

impl Money {
    pub fn new(amount: Amount, commodity: Commodity) -> Self {
        Self { amount, commodity }
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.amount, self.commodity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parses_amounts_without_floats() {
        let a = Amount::parse("1,234.56").unwrap();
        assert_eq!(a.to_string(), "1234.56");
        assert!(!a.is_zero());
    }

    #[test]
    fn rejects_empty_commodity() {
        assert!(Commodity::new("").is_err());
    }
}
