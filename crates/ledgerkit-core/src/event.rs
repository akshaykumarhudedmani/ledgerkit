use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::hash::ContentHash;
use crate::ids::{ImportBatchId, TransactionId};

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
