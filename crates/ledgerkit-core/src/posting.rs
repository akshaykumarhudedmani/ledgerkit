use serde::{Deserialize, Serialize};

use crate::account::AccountId;
use crate::money::{Amount, Commodity};

/// One leg of a double-entry transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Posting {
    pub account: AccountId,
    pub amount: Amount,
    pub commodity: Commodity,
    pub memo: Option<String>,
}

impl Posting {
    pub fn new(account: AccountId, amount: Amount, commodity: Commodity) -> Self {
        Self {
            account,
            amount,
            commodity,
            memo: None,
        }
    }

    pub fn with_memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = Some(memo.into());
        self
    }
}
