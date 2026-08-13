# Data Model & SQLite Schema

Schema version: **3** (see `ledgerkit-store::SCHEMA_VERSION`)

## Logical model

```text
Commodity 1---* Account
Merchant 1---* Alias
ImportBatch 1---* StatementRow
ImportBatch 1---* Transaction
Transaction 1---* Posting
Transaction *---0..1 Transaction (duplicate_of)
StatementRow *---0..1 Transaction
Event (append-only log)
```

## Invariants

1. `sum(postings.amount)` per `(transaction, commodity) == 0`
2. `len(postings) >= 2` for every transaction
3. Balances = Σ postings for account (skip `duplicate_of IS NOT NULL`)
4. Events are insert-only; `prev_hash` chains to genesis
5. Replaying events yields identical ledger content hash
6. Import identity: see [identity.md](identity.md)

## Tables (summary)

| Table | Role |
|-------|------|
| `meta` | schema version / workspace metadata |
| `accounts` | chart of accounts |
| `commodities` | currency codes + decimal scale |
| `merchants` / `merchant_aliases` | identity + aliases |
| `import_batches` | source path + sha256 + adapter |
| `statement_rows` | every parsed/failed import row |
| `transactions` / `postings` | double-entry journal (`row_fingerprint` on imports) |
| `events` | append-only audit log |

Proof markdown is written under the workspace `reports/` directory (created by `init`).

Exact SQL: `crates/ledgerkit-store/src/schema.rs`.

## Event kinds

| Kind | Meaning |
|------|---------|
| `account_upserted` | Chart of accounts change |
| `posted` | Balanced transaction accepted |
| `imported` | New batch from adapter |
| `normalized` | Merchant/date/amount normalization |
| `deduped` | Linked duplicate → survivor |
| `categorized` | Rule applied |
| `reconciled` | Statement proof produced |
| `manual_edit` | Explicit user correction (`tx add` is a `posted` event) |

## ID strategy

- **Imported** transactions: SHA-256 of row fingerprint → UUID v8-shaped.
- **Manual** `tx add`, merchants, batches, events: UUIDv7.
