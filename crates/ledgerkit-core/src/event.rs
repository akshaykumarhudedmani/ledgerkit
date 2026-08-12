use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::account::Account;
use crate::hash::ContentHash;
use crate::ids::{ImportBatchId, TransactionId};
use crate::transaction::Transaction;

/// Append-only audit event. Events are never mutated or deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub seq: u64,
    pub at: DateTime<Utc>,
    pub kind: EventKind,
    pub payload: EventPayload,
    /// Hash of this event's canonical bytes (for chain integrity).
    pub content_hash: ContentHash,
    /// Previous event hash; genesis uses zeros.
    pub prev_hash: ContentHash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// Chart-of-accounts upsert (Phase 2).
    AccountUpserted,
    /// Balanced transaction accepted into the ledger (Phase 2).
    Posted,
    Imported,
    Normalized,
    Deduped,
    Categorized,
    Reconciled,
    ManualEdit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventPayload {
    AccountUpserted {
        account: Account,
    },
    Posted {
        transaction: Transaction,
    },
    Imported {
        batch_id: ImportBatchId,
        source_path: String,
        source_sha256: ContentHash,
        row_count: u64,
    },
    Normalized {
        transaction_id: TransactionId,
        reasons: Vec<String>,
    },
    Deduped {
        duplicate_id: TransactionId,
        survivor_id: TransactionId,
        strategy: String,
        explanation: String,
    },
    Categorized {
        transaction_id: TransactionId,
        category: String,
        rule_id: String,
        confidence: u8,
        reasons: Vec<String>,
    },
    Reconciled {
        account: String,
        as_of: String,
        ending_balance: String,
        matched: u64,
        unmatched: u64,
        unexplained_delta: String,
        report_path: Option<String>,
    },
    ManualEdit {
        transaction_id: TransactionId,
        summary: String,
    },
}

impl Event {
    /// Build a sealed event linked to `prev_hash`. `seq` is assigned by the store on insert.
    pub fn seal(kind: EventKind, payload: EventPayload, prev_hash: ContentHash) -> Self {
        let id = Uuid::now_v7();
        let at = Utc::now();
        let payload_json = serde_json::to_string(&payload).expect("EventPayload always serializes");
        let content_hash = hash_event_bytes(&prev_hash, kind, &payload_json, &at, &id);
        Self {
            id,
            seq: 0,
            at,
            kind,
            payload,
            content_hash,
            prev_hash,
        }
    }

    /// Recompute content hash (used to verify chain integrity).
    pub fn expected_content_hash(&self) -> ContentHash {
        let payload_json =
            serde_json::to_string(&self.payload).expect("EventPayload always serializes");
        hash_event_bytes(
            &self.prev_hash,
            self.kind,
            &payload_json,
            &self.at,
            &self.id,
        )
    }
}

fn hash_event_bytes(
    prev_hash: &ContentHash,
    kind: EventKind,
    payload_json: &str,
    at: &DateTime<Utc>,
    id: &Uuid,
) -> ContentHash {
    let kind_str = serde_json::to_string(&kind).unwrap_or_else(|_| "\"unknown\"".into());
    let canonical = format!(
        "v1\nprev={}\nid={}\nat={}\nkind={}\npayload={}",
        prev_hash.as_str(),
        id,
        at.to_rfc3339(),
        kind_str.trim_matches('"'),
        payload_json
    );
    ContentHash::sha256_str(&canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{Account, AccountId, AccountType};
    use crate::money::Commodity;

    #[test]
    fn sealed_hash_is_stable_for_same_inputs() {
        let account = Account::new(
            AccountId::new("assets:cash").unwrap(),
            AccountType::Asset,
            Commodity::new("INR").unwrap(),
            "Cash",
        );
        let payload = EventPayload::AccountUpserted { account };
        let a = Event::seal(
            EventKind::AccountUpserted,
            payload.clone(),
            ContentHash::zero(),
        );
        // Different id/at ⇒ different hash (intentionally includes those fields).
        let b = Event::seal(EventKind::AccountUpserted, payload, ContentHash::zero());
        assert_ne!(a.id, b.id);
        assert_eq!(a.expected_content_hash(), a.content_hash);
        assert_eq!(b.expected_content_hash(), b.content_hash);
    }
}
