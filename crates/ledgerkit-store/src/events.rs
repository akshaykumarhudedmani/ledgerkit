use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use ledgerkit_core::{ContentHash, Event, EventKind, EventPayload};
use uuid::Uuid;

use crate::Store;

impl Store {
    pub fn last_event_hash(&self) -> Result<ContentHash> {
        match self.conn.query_row(
            "SELECT content_hash FROM events ORDER BY seq DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        ) {
            Ok(hex) => Ok(content_hash_from_hex(hex)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(ContentHash::zero()),
            Err(err) => Err(err.into()),
        }
    }

    /// Append a sealed event. Assigns monotonic `seq`. Returns the stored event.
    pub fn append_event(&mut self, event: Event) -> Result<Event> {
        let db_tx = self.conn.transaction()?;
        let stored = append_sealed_event(&db_tx, event)?;
        db_tx.commit()?;
        Ok(stored)
    }

    pub fn list_events(&self) -> Result<Vec<Event>> {
        self.events_through(u64::MAX)
    }

    pub fn events_through(&self, max_seq: u64) -> Result<Vec<Event>> {
        let bound = if max_seq >= i64::MAX as u64 {
            i64::MAX
        } else {
            max_seq as i64
        };
        let mut stmt = self.conn.prepare(
            "SELECT seq, id, at, kind, payload_json, content_hash, prev_hash
             FROM events
             WHERE seq <= ?1
             ORDER BY seq ASC",
        )?;
        let rows = stmt.query_map([bound], |row| {
            Ok(RawEventRow {
                seq: row.get::<_, i64>(0)? as u64,
                id: row.get(1)?,
                at: row.get(2)?,
                kind: row.get(3)?,
                payload_json: row.get(4)?,
                content_hash: row.get(5)?,
                prev_hash: row.get(6)?,
            })
        })?;

        let mut out = Vec::new();
        for row in rows {
            out.push(decode_event(row.context("read event row")?)?);
        }
        Ok(out)
    }

    /// Verify prev/content hash chain. Returns Ok(tip) or error on break.
    pub fn verify_event_chain(&self) -> Result<ContentHash> {
        let events = self.list_events()?;
        let mut prev = ContentHash::zero();
        for event in &events {
            if event.prev_hash != prev {
                bail!(
                    "event chain broken at seq {}: prev_hash mismatch",
                    event.seq
                );
            }
            if event.content_hash != event.expected_content_hash() {
                bail!(
                    "event chain broken at seq {}: content_hash mismatch",
                    event.seq
                );
            }
            prev = event.content_hash.clone();
        }
        Ok(prev)
    }
}

struct RawEventRow {
    seq: u64,
    id: String,
    at: String,
    kind: String,
    payload_json: String,
    content_hash: String,
    prev_hash: String,
}

pub(crate) fn append_sealed_event(conn: &rusqlite::Connection, event: Event) -> Result<Event> {
    let expected_prev = match conn.query_row(
        "SELECT content_hash FROM events ORDER BY seq DESC LIMIT 1",
        [],
        |row| row.get::<_, String>(0),
    ) {
        Ok(hex) => content_hash_from_hex(hex),
        Err(rusqlite::Error::QueryReturnedNoRows) => ContentHash::zero(),
        Err(err) => return Err(err.into()),
    };
    if event.prev_hash != expected_prev {
        bail!(
            "event prev_hash mismatch: got {}, tip {}",
            event.prev_hash,
            expected_prev
        );
    }
    if event.content_hash != event.expected_content_hash() {
        bail!("event content_hash does not match sealed payload");
    }
    insert_event_row(conn, event)
}

fn insert_event_row(conn: &rusqlite::Connection, mut event: Event) -> Result<Event> {
    let kind = event_kind_str(event.kind);
    let payload_json = serde_json::to_string(&event.payload)?;
    conn.execute(
        "INSERT INTO events (id, at, kind, payload_json, content_hash, prev_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (
            event.id.to_string(),
            event.at.to_rfc3339(),
            kind,
            payload_json,
            event.content_hash.as_str(),
            event.prev_hash.as_str(),
        ),
    )?;
    let seq: i64 = conn.last_insert_rowid();
    event.seq = seq as u64;
    Ok(event)
}

fn decode_event(row: RawEventRow) -> Result<Event> {
    let id = Uuid::parse_str(&row.id).context("event id")?;
    let at = DateTime::parse_from_rfc3339(&row.at)
        .context("event at")?
        .with_timezone(&Utc);
    let kind = parse_event_kind(&row.kind)?;
    let payload: EventPayload =
        serde_json::from_str(&row.payload_json).context("event payload_json")?;
    Ok(Event {
        id,
        seq: row.seq,
        at,
        kind,
        payload,
        content_hash: content_hash_from_hex(row.content_hash),
        prev_hash: content_hash_from_hex(row.prev_hash),
    })
}

pub(crate) fn content_hash_from_hex(hex: String) -> ContentHash {
    serde_json::from_value(serde_json::Value::String(hex))
        .expect("ContentHash deserializes from hex string")
}

fn event_kind_str(kind: EventKind) -> &'static str {
    match kind {
        EventKind::AccountUpserted => "account_upserted",
        EventKind::Posted => "posted",
        EventKind::Imported => "imported",
        EventKind::Normalized => "normalized",
        EventKind::Deduped => "deduped",
        EventKind::Categorized => "categorized",
        EventKind::Reconciled => "reconciled",
        EventKind::ManualEdit => "manual_edit",
    }
}

fn parse_event_kind(s: &str) -> Result<EventKind> {
    Ok(match s {
        "account_upserted" => EventKind::AccountUpserted,
        "posted" => EventKind::Posted,
        "imported" => EventKind::Imported,
        "normalized" => EventKind::Normalized,
        "deduped" => EventKind::Deduped,
        "categorized" => EventKind::Categorized,
        "reconciled" => EventKind::Reconciled,
        "manual_edit" => EventKind::ManualEdit,
        other => bail!("unknown event kind: {other}"),
    })
}
