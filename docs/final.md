# LedgerKit — final product contract

This is the **finished** LedgerKit. There is no v2 roadmap, no “next version,” and no enterprise/standard/fintech sequel in this repository.

Software can still receive **bug fixes**. Scope does not grow.

## What it is

A **local-first Rust engine**: supported bank CSV → adapters → normalize / dedupe / rules → **double-entry SQLite ledger** + **append-only events** → ending-balance **and** statement-row reconcile → `why` → Beancount / JSON / CSV.

**Completion sentence:** given a supported statement file, LedgerKit produces the same correct ledger every time, keeps provenance, explains transforms, mismatches at **statement-row** level, rebuilds projections from events, and exports data other tools can trust.

## In scope (this is the whole product)

| Slice | Meaning |
|-------|---------|
| Identity | Imported rows get a content fingerprint and a deterministic transaction id. Manual `tx add` stays UUIDv7. |
| Statement rows | Every import persists parsed/failed rows; errors are never silent. |
| Reconcile | Ending-balance proof **and** row↔txn matching. |
| Rebuild | `ledgerkit rebuild` wipes projections, replays events, same ledger hash. |
| Adapters | `hdfc`, `generic_csv`, `credit_card`, `custom` — boringly correct for checked-in fixtures. One in-repo plugin crate on public traits. |
| Eval | Labeled fixtures + honest metrics (not a bank corpus). |
| Hygiene | Threat model, SECURITY, CONTRIBUTING, CHANGELOG, import size cap, path hygiene. |

## Forever out of this repository

PDF / OCR / email / SMS, live bank APIs, Plaid, AI categorization, Python/TS bindings, GnuCash sync, encryption vault, signed releases, mobile, cloud, tax, multi-user, “open standard,” extra crates for events/reconcile, crate reorg theater.

## Identity (summary)

See [identity.md](identity.md). Fingerprint **excludes** wall-clock and import batch id. Overlapping CSVs that describe the **same** economic row reuse the same transaction id (second file does not post twice). Distinct `source_refs` on the same day stay distinct.

## How to know we are done

1. `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
2. Demo script still balances checking at the documented ending amount.
3. `rebuild` after import+dedupe+rules yields the same `ledger_hash` as `verify`.
4. Docs in this folder describe the product as shipped — not a future manifesto.
