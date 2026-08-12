# Transaction identity

## Policy

| Origin | Id | Fingerprint |
|--------|----|-------------|
| Import convert | SHA-256 of fingerprint → UUID (RFC variant, version nibble `8`) | Stored on `transactions.row_fingerprint` |
| Manual `tx add` | UUIDv7 | None |

Same convert inputs ⇒ same fingerprint ⇒ same id. Wall-clock and `import_batch_id` are **not** in the fingerprint.

## Fingerprint v1 (canonical string)

```text
v1|{adapter}|{account}|{YYYY-MM-DD}|{normalized_amount}|{commodity}|{source_refs}|{canonical_merchant}
```

- `source_refs`: sorted, joined with U+001F.
- `normalized_amount`: `Decimal::normalize` of the parsed amount (so `10.00` and `10` match).
- `canonical_merchant`: import `normalize_merchant` key (same as transaction narration).

## Overlapping files

Two different files (different SHA-256) can contain the same economic row. That is **one** ledger transaction. The second import:

- creates a new `import_batches` row (different bytes);
- inserts `statement_rows` for the new file;
- **skips** `Posted` if the transaction id already exists;
- points the new statement row at the existing transaction.

Exact-file re-import remains idempotent on `(adapter, source_sha256, account)`.

## What identity is not

- Not “two overlapping files hash to one file id.”
- Not content-addressed **events** (events still use UUIDv7 + wall-clock in the hash chain).
- Not a global merchant graph. Merchant key is a deterministic string, not a fuzzy merge.
