use chrono::{Datelike, NaiveDate};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("unparseable date {raw:?}")]
pub struct DateParseError {
    pub raw: String,
}

/// Deterministic date parse. Prefers ISO, then day-first (common in IN/EU), then US.
pub fn parse_date(raw: &str) -> Result<NaiveDate, DateParseError> {
    let s = raw.trim();
    const FORMATS: &[&str] = &[
        "%Y-%m-%d", "%d/%m/%Y", "%d/%m/%y", "%d-%m-%Y", "%d-%m-%y", "%Y/%m/%d", "%m/%d/%Y",
        "%m/%d/%y", "%d %b %Y", "%d-%b-%Y",
    ];
    for fmt in FORMATS {
        if let Ok(d) = NaiveDate::parse_from_str(s, fmt) {
            return Ok(pivot_year(d));
        }
    }
    Err(DateParseError {
        raw: raw.to_string(),
    })
}

fn pivot_year(d: NaiveDate) -> NaiveDate {
    if d.year() < 100 {
        NaiveDate::from_ymd_opt(d.year() + 2000, d.month(), d.day()).unwrap_or(d)
    } else {
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_and_dmy() {
        assert_eq!(
            parse_date("2026-03-01").unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()
        );
        assert_eq!(
            parse_date("01/02/26").unwrap(),
            NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()
        );
    }
}
