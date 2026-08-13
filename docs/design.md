# LedgerKit Design Document (Phase 1)

**Status:** final product (see [final.md](final.md))  
**Stack:** Rust + SQLite + clap + serde + rust_decimal + proptest  
**Thesis:** LedgerKit is a local-first financial data engine that turns messy bank exports into an auditable, double-entry ledger with deterministic transforms, reconciliation proofs, and exports — with every decision explainable.

---

## 1. Problem

Bank exports are fragmented and lossy:

- Every institution ships a different CSV shape.
- Merchant strings are noisy (`AMZN MKTP US*ABC` vs `Amazon Marketplace`).
- Duplicate rows appear across overlapping statement downloads.
- Consumer apps hide how balances were computed.
- Self-hosted finance tools either lack reliable imports or depend on paid APIs.

LedgerKit sits **between** ugly exports and tools people already trust (Beancount, GnuCash, spreadsheets): clean, local, auditable plumbing.

---

## 2. Goals / Non-goals

### Goals

1. Deterministic CSV → canonical ledger pipeline.
2. Double-entry correctness with machine-checked invariants.
3. Append-only provenance for every mutate.
4. Reconciliation proof artifacts.
5. Stable adapter/exporter plugin interfaces.
6. Usable as library crates *and* CLI.

### Non-goals

See [final.md](final.md). No mobile, PDF/OCR, Plaid, tax, cloud sync, ML categorization, or a v2 product in this repo.

---

## 3. Architecture

```text
┌──────────────┐    ┌─────────────┐    ┌──────────────────┐
│ Bank adapters│ -> │ Normalize / │ -> │ Ledger engine    │
│ (plugins)    │    │ Dedup/Rules │    │ (double-entry)   │
└──────────────┘    └─────────────┘    └────────┬─────────┘
                                               │
                     ┌─────────────┐           ▼
                     │ Exporters   │ <- ┌──────────────┐
                     └─────────────┘    │ Event log DB │
                                        │ (SQLite)     │
                                        └──────────────┘
```

### Crates

| Crate | Responsibility |
|-------|----------------|
| `ledgerkit-core` | Money, accounts, postings, txns, events, verify |
| `ledgerkit-store` | SQLite schema, append-only events, replay |
| `ledgerkit-import` | `BankAdapter`, normalize, dedupe, rules |
| `ledgerkit-export` | CSV/JSON/Beancount exporters |
| `ledgerkit-cli` | User-facing CLI |

---

## 4. Ledger semantics

### Money

- Represented as `rust_decimal::Decimal` wrapped in `Amount`.
- Never `f32`/`f64`.
- Commodity codes are uppercase strings (`INR`, `USD`).

### Double-entry

- A `Transaction` has ≥ 2 `Posting`s.
- For each commodity, posting amounts **must sum to zero**.
- Account balances are computed **only** by summing postings (duplicates skipped via `duplicate_of`).

### Accounts

- Hierarchical ids: `assets:bank:hdfc`.
- Types: asset, liability, equity, income, expense.

### Merchants

- Raw description → deterministic `canonical_key` + confidence + reasons.
- Fuzzy merges are **suggestions** unless confidence is high and explicit.

### Dedup

- Fingerprint: date + amount + account + normalized merchant + source refs.
- Import identity: see [identity.md](identity.md). Overlapping files reuse the same transaction id.
- Strategies: exact, near-window.
- Never delete: set `duplicate_of` + explanation event.

### Categorization

- Deterministic YAML/JSON rules DSL.
- Precedence + conflict reporting.
- Every assignment emits `Categorized` with rule id, confidence, reasons.

---

## 5. Event log

Append-only table `events` with kinds:

`Imported | Normalized | Deduped | Categorized | Reconciled | ManualEdit`

Each event stores:

- monotonic `seq`
- payload JSON
- `content_hash` of canonical payload (chained with `prev_hash`, id, at, kind)
- `prev_hash` forming a hash chain

Ledger core event kinds: `account_upserted`, `posted`.

**Replay invariant:** folding `Posted` events `0..=N` always rebuilds the same ledger content hash as the materialized tables.

**`ledgerkit why <tx_id|row_id>`:** walks events referencing that transaction, or a persisted statement row.

**`ledgerkit rebuild`:** deletes projection tables and folds events back; ledger content hash must match.

---

## 6. Import idempotency

1. Hash source bytes (SHA-256); store under `artifacts/`.
2. Same `(adapter, source_sha256, account)` → no-op or explicit re-import with new batch id.
3. Row-level errors are reported; rows are never silently dropped without an error entry.

---

## 7. Reconciliation

**Ending balance:** account, as-of date, stated ending (optional starting). Output: matched postings, skipped duplicates, unexplained delta, markdown proof.

**Statement rows:** `reconcile --rows` matches persisted `statement_rows` to imported transactions (plus convert/parse errors).

Success for ending-balance means unexplained delta == 0 within commodity scale.

---

## 8. Failure modes

| Failure | Behavior |
|---------|----------|
| Malformed CSV | Row errors in `ParseReport`; import continues for good rows |
| Unbalanced txn | Rejected at construction; `verify` fails CI |
| Corrupt SQLite | Open fails loudly; no silent repair of event hashes |
| Path traversal in export paths | Rejected (`reject_parent_dir`; unit tests) |
| Float coercion attempt | Type system prevents floats in money APIs |

---

## 9. Security / privacy

- No telemetry.
- No accounts or secrets required.
- Original statements hashed and stored locally only.
- Threat model: see [threat-model.md](threat-model.md).

---

## 10. Testing strategy

1. Unit tests for parsers, money, verify.
2. Property tests: random balanced transfers always verify; replay hash stability.
3. Golden fixtures under `fixtures/`.
4. Fuzz malformed CSV (`crates/ledgerkit-import/tests/fuzz_adapters.rs`).
5. Benchmarks: 100k-row import (`scripts/bench.ps1`, ignored in default CI).

---

## 11. Evaluation metrics (thesis)

- Dedup precision/recall on labeled set
- Categorization accuracy (rules only)
- Reconciliation success rate on N statements
- Parser success rate by bank
- Import latency/memory at 100k rows

---

## 12. Closed questions

Multi-currency conversion, transfer detection, and JSON Schema for rules are **out of product**. Use Beancount (or another tool) after export if you need them.

---

## References

- Beancount ledger semantics
- Double-entry bookkeeping invariants
- Local-first software principles (Ink & Switch)
