# Data Model & SQLite Schema

Schema version: **1** (see `ledgerkit-store::SCHEMA_VERSION`)

## Logical model

```text
Commodity 1---* Account
Merchant 1---* Alias
ImportBatch 1---* Transaction
Transaction 1---* Posting
Transaction *---0..1 Transaction (duplicate_of)
Event (append-only log)
```

## Invariants

1. `sum(postings.amount)` per `(transaction, commodity) == 0`
2. `len(postings) >= 2` for every transaction
3. Balances = Σ postings for account (skip `duplicate_of IS NOT NULL`)
4. Events are insert-only; `prev_hash` chains to genesis
5. Replaying events yields identical ledger content hash

## Tables (summary)

| Table | Role |
|-------|------|
| `meta` | schema version / workspace metadata |
| `accounts` | chart of accounts |
| `commodities` | currency codes + decimal scale |
| `merchants` / `merchant_aliases` | identity + aliases |
| `import_batches` | source path + sha256 + adapter |
| `transactions` / `postings` | double-entry journal |
| `events` | append-only audit log |

Exact SQL: `crates/ledgerkit-store/src/schema.rs`.

## Event kinds

| Kind | Meaning |
|------|---------|
| `Imported` | New batch from adapter |
| `Normalized` | Merchant/date/amount normalization |
| `Deduped` | Linked duplicate → survivor |
| `Categorized` | Rule applied |
| `Reconciled` | Statement proof produced |
| `ManualEdit` | Explicit user correction |

## ID strategy

UUIDv7 for transactions, merchants, batches (time-sortable, opaque).
