# LedgerKit — personal pitch & interview brief

This is **your** walkthrough. Use it before interviews. Do not oversell.

**If a word looks like jargon, read [glossary.md](glossary.md) first.** Every term below is defined there in normal English (ledger, posting, hash, reconcile, UUID, crate, …).

Repo: https://github.com/akshaykumarhudedmani/ledgerkit  
Stack: **Rust**, **SQLite** (WAL, bundled rusqlite), **clap**, **serde**, **rust_decimal**, **SHA-256**, **UUIDv7**, **proptest**.  
License: MIT. Local-only. No cloud, no Plaid, no telemetry.

---

## 30 seconds

I built **LedgerKit**, a local-first engine that turns messy bank CSVs into a **double-entry ledger** you can audit. Same file plus same rules always produce the same books. Every change is an **append-only event**. Balances come only from postings. You can **reconcile** to a statement ending balance and to **individual statement rows**, ask **why** a transaction exists, **rebuild** the ledger from the event log, and **export** to Beancount, JSON, or CSV.

---

## 2 minutes

Banks dump CSVs with different columns, noisy merchant names, and overlapping downloads. Consumer apps hide how the number on screen was computed. Beancount and spreadsheets are great *after* the data is clean. LedgerKit is the **plumbing in the middle**.

Pipeline: **adapter parse** → **normalize merchant** → **convert to balanced postings** → **SQLite ledger + event log** → **dedupe** (never delete) → **YAML/JSON rules** (tags) → **reconcile / why / rebuild / export**.

Invariants I refuse to break:

- No floats for money (`rust_decimal` only).
- Unbalanced transactions never enter the ledger.
- CSV rows are never silently dropped (`ParseReport` + `statement_rows`).
- Duplicates get `duplicate_of` + an explanation event; they are not deleted.
- Account balances = sum of postings, skipping duplicates.
- Ledger **content hash** does not include wall-clock; event chain hashes do (integrity), replay still matches.

I shipped this as a Rust workspace (core / store / import / export / CLI), with CI, fuzz on adapters, and an honest eval chapter: precision 1.00 on **small synthetic fixtures**, not a bank corpus.

---

## What it is / is not

| It is | It is not |
|-------|-----------|
| A **CSV → auditable ledger** engine | A budgeting app or mobile product |
| A **library + CLI** | A SaaS or bank aggregator |
| Provenance (`why`, events, proofs) | Tax software or GnuCash replacement |
| Deterministic transforms | AI categorization |
| Finished **kernel** (`docs/final.md`) | The best personal-finance OSS vs Beancount/Firefly |

**Positioning sentence:** LedgerKit sits between ugly bank exports and tools people already trust.

---

## Problem I chose (say this)

1. Every bank CSV is a different shape.
2. `AMZN MKTP US*ABC` vs `Amazon Marketplace` look like two merchants.
3. Overlapping statement downloads create duplicates.
4. Apps will not show you *how* a balance was derived.
5. Self-hosted tools either skip reliable import or pull paid APIs.

---

## Architecture (draw this)

```text
  Bank CSV
      │
      ▼
  BankAdapter::parse(bytes)     → RawTransactions + ParseReport
      │
      ▼
  convert_raw                   → balanced Transaction (2+ postings)
      │                           import id = SHA-256(fingerprint) → UUID
      ▼
  Store.apply_import            → import_batches, statement_rows, Posted events
      │
      ├── dedupe                → duplicate_of + Deduped event
      ├── rules apply           → category: tag + Categorized event
      ├── reconcile             → markdown proof + Reconciled event
      ├── why                   → walk events for tx id or statement-row id
      ├── rebuild               → wipe projections, fold events, same ledger_hash
      └── export                → json | beancount | csv  (duplicates omitted)
```

### Crates (why split)

| Crate | Job | Interview one-liner |
|-------|-----|---------------------|
| `ledgerkit-core` | Money, accounts, postings, txns, events, verify, ending-balance recon | Domain types; no SQLite |
| `ledgerkit-store` | Schema v3, events, replay, rebuild, statement rows, `why` | Persistence + projections |
| `ledgerkit-import` | Adapters, dates, normalize, convert, dedupe, rules | All CSV → txn logic |
| `ledgerkit-export` | `Exporter` trait: JSON, Beancount, CSV | Snapshot → text |
| `ledgerkit-cli` | clap commands, path/size hygiene | Thin orchestration |
| `plugins/sample-adapter` | Depends only on public `BankAdapter` | Proves extension without forking core |

SQLite is **one file** under `.ledgerkit/ledger.sqlite` (WAL, foreign keys). Original bytes go to `artifacts/` named by adapter + SHA-256 prefix. Proofs go to `reports/`.

---

## How money works

- `Amount` wraps `rust_decimal::Decimal`. Construction from `&str`. Display/canonical fingerprint uses `normalize()` so `10.00` and `10` match.
- `Commodity` is an uppercase code (`INR`, `USD`), not a float FX engine. **No multi-currency conversion** in product.
- A `Transaction` has ≥ 2 `Posting`s. For **each commodity**, posting amounts must sum to **zero** or `Transaction::new` fails (`UnbalancedTransaction`).
- `account_balance` walks postings; if `duplicate_of` is set, that txn is **skipped**.

If they ask “why not f64?”: binary floats cannot represent 0.1; books drift by cents; that is unacceptable for a ledger.

---

## How import works (end to end)

Command:

```text
ledgerkit import statement.csv --account assets:bank:checking --adapter generic_csv --commodity USD
```

1. **Cap:** `stat` then reject files **> 32 MiB**. Parsers also cap **200_000** data rows.
2. **Adapter** (`hdfc` | `generic_csv` | `credit_card` | `custom`): deterministic parse. Failed rows become `ParseReport.errors`; good rows become `RawTransaction` (date/amount/desc/source_refs like `generic:row:2`).
3. **Dates:** ISO first, then day-first (`01/02/26` → 1 Feb 2026), then US. Documented in `dates.rs`.
4. **Convert:** parse amount; zero amounts error (reported, not dropped). Sign: negative → expense offset, positive → income offset. Bank posting memo = joined source refs. Narration = canonical merchant key.
5. **Identity:** fingerprint  
   `v1|{adapter}|{account}|{date}|{canonical_amount}|{commodity}|{sorted source_refs}|{merchant}`  
   **Not** in the fingerprint: wall-clock, batch id.  
   Tx id = first 16 bytes of SHA-256(fingerprint), UUID version nibble `8`.  
   Manual `tx add` stays **UUIDv7**.
6. **Idempotency (same file):** unique `(adapter, source_sha256, account)` → `ImportOutcome::Duplicate`, no second post.
7. **Overlap (different file, same economic row):** new batch + new `statement_rows`; if tx id already exists, **skip Posted**.
8. **statement_rows:** every ok row, convert failure, and parse failure is persisted (`parse_status`: `ok` | `convert_error` | `parse_error`). Import does not bail before write just because convert produced zero txns, as long as there are rows to store.

**Normalize merchant (deterministic, not ML):** trim, uppercase, collapse space, strip trailing `*ABC123`-style refs, alnum → `canonical_key` (`amzn_mktp_us`). Conservative: different keys are **not** auto-merged.

---

## How dedupe differs from identity

| | Identity (import) | Dedupe (`ledgerkit dedupe`) |
|--|-------------------|-----------------------------|
| When | Convert time | After txns exist |
| Same overlapping CSV row | Same tx **id**, skip second post | N/A |
| Same Starbucks twice (manual + import) | Different ids (manual is v7) | Exact or near-window **link** |
| Action | Skip insert | Set `duplicate_of`, never delete |
| Exact key | Includes date, account, amount, commodity, merchant, **source_refs** | Same idea |
| Near-window | — | Same merchant/amount/account within `--window-days` (default 1) |
| Same day, different `generic:row:N` | Distinct ids | **Must not** exact-merge |

If they ask “why not delete duplicates?”: audit trail. The second row happened; the books should show it was linked, not vanished.

---

## How rules work (be honest)

YAML/JSON list of rules: `id`, `priority`, `category`, optional regexes on merchant/payee/account, min/max amount, confidence.

- Highest priority unique match → `category:<account>` **tag** + `Categorized` event.
- Equal-priority conflict → reported, **not** applied.
- Skips duplicates and already-tagged txns.

**Honest limitation:** rules **do not rewrite postings**. The offset account from convert stays `expenses:uncategorized` (or whatever you passed). Beancount export can still show uncategorized legs. Tags are the categorization proof for v1.

---

## How the event log works

Table `events` is **insert-only**. Each event: `seq`, UUIDv7 `id`, `at` (RFC3339), `kind`, `payload_json`, `content_hash`, `prev_hash`.

Hash input (chain integrity):

```text
v1
prev=...
id=...
at=...
kind=...
payload=...
```

Kinds you should name: `account_upserted`, `posted`, `imported`, `deduped`, `categorized`, `reconciled` (plus reserved `normalized`, `manual_edit`).

**Two hashes, do not confuse them:**

1. **Event chain hash** — includes `at` and event id. Two runs produce different event hashes. `verify` checks the chain is not torn.
2. **Ledger content hash** — canonical JSON of transactions sorted by date+id (`verify_ledger`). Same books ⇒ same hash. Used for replay/rebuild equality.

`replay --through N` folds events `seq <= N` (Posted, then Deduped/Categorized overlays).

`rebuild` **deletes** projection tables (accounts, txns, postings, …), **keeps** `events` (and statement rows / import batches), folds events back, asserts hashes match. Events are never rewritten.

`why <uuid>`: events that mention that txn (posted, imported for its batch, deduped, categorized, reconcile only if txn date ≤ as_of and posting hits that account).  
`why <integer>`: statement row metadata, then the linked txn chain.

---

## How reconcile works

**Ending balance** (proof of “does the ledger match the number on the PDF/CSV footer?”):

```text
ledgerkit reconcile --account assets:bank:checking --balance 2409.20 --as-of 2026-01-07 --commodity USD
```

- Sum postings on that account/commodity with `date <= as_of`, skip `duplicate_of`.
- `computed = starting + activity` (starting default 0).
- `unexplained_delta = computed - stated`.
- Writes `reports/reconcile-*.md` (filename sanitized; no `..`, no slashes).
- Demo number after import + extra Starbucks + dedupe: **2409.20 USD**, delta **0**.

**Row-level:**

```text
ledgerkit reconcile --account assets:bank:checking --rows
```

Matches persisted `statement_rows` to imported txns; reports convert/parse errors and imported txns with no row. Success means no gaps — not the same as ending-balance success.

---

## How export works

`Exporter::export(&LedgerSnapshot) -> String`. Duplicates omitted.

- **JSON:** snapshot dump.
- **CSV:** postings-oriented, amounts escaped.
- **Beancount:** `commodity`, `open`, metadata; account names capitalized (`Assets:Bank:Checking`); operating currency = **most-used** commodity, not alphabetical.

Export `--out` rejects any `..` path component.

---

## Schema (SQLite v3)

Tables: `meta`, `accounts`, `commodities`, `merchants`, `merchant_aliases`, `import_batches`, `transactions` (+ `row_fingerprint`), `postings`, `statement_rows`, `events`.

Unique import: `(adapter, source_sha256, account_id)`.  
Unique statement row: `(batch_id, row_number)`.

Migrate: `CREATE IF NOT EXISTS` plus `ALTER` for `row_fingerprint` on old DBs. Events are never migrated by rewrite.

---

## CLI map (memorize)

| Command | What to say |
|---------|-------------|
| `init` | Workspace: sqlite, `artifacts/`, `reports/`, `config.json` telemetry false |
| `account add` | Chart of accounts + event |
| `tx add` | Manual balanced postings, UUIDv7 |
| `import` | Adapter → convert → persist |
| `dedupe` | Links only |
| `rules apply` | Tags only |
| `balance` | Derived from postings |
| `reconcile` | Ending balance and/or `--rows` |
| `why` | Provenance |
| `verify` | Chain + replay hash, no mutate |
| `replay` | Time-travel fold |
| `rebuild` | Re-materialize from events |
| `export` | json/beancount/csv |
| `adapters` | List builtins |

---

## Testing & eval (say the numbers, then the caveat)

- Unit: money, unbalanced reject, adapters, goldens (`fixtures/golden/parse_counts.json`).
- Property: random non-zero transfers always balance (`proptest`).
- Fuzz: adapters must not panic on random bytes/UTF-8.
- Scale: 1k in CI; 100k ignored bench (`scripts/bench.ps1`). Parse ~0.5s debug; convert was **very** slow in debug (~3 min) — cite **release** if asked.
- Dedup/rules: **1.00 precision/recall on ~50 synthetic pairs / 5 payees**. **Not** a bank-labeled corpus. Say that first if they push on ML metrics.

Demo: `scripts/demo.ps1` / `docs/demo-script.md`.

---

## Security (local threat model)

No server ⇒ no remote attacker in scope. Assets: CSVs, sqlite, artifacts, user trust in balances.

Hardening you can name: no telemetry, SHA-256 of source, 32 MiB + 200k row caps, export `..` reject, proof files only under `reports/`, bound SQL parameters, `PRAGMA table_info` only with hardcoded table names.

Not claimed: encryption at rest, defeating OS malware, multi-tenant isolation.

---

## Design decisions they will probe

**Why Rust?** Invariants in the type system; no GC pauses in a CLI; `rust_decimal`; one binary; clippy `-D warnings` in CI.

**Why SQLite not Postgres?** Single-user local-first; one file; no daemon. Events are the log; sqlite is the materialization.

**Why not content-address the event log?** Events include time and id by design (hash chain). Ledger *content* is separate and deterministic.

**Why UUIDv7 for manual txns?** Time-sortable, no fingerprint (human-entered, not a statement row).

**Why skip-on-id for overlap instead of always posting?** Two files can describe one charge; posting twice would double-count the bank.

**Why not PDF/Plaid?** Out of scope forever for this repo. Completeness of the *kernel*, not a bank OS.

**Vs Beancount?** Beancount is the destination journal. LedgerKit is import, identity, events, proofs. Export *into* Beancount.

**Vs Firefly III?** Firefly is a web app + optional bank APIs. LedgerKit is offline CLI/engine.

**Biggest technical risk I accepted?** Conservative dedupe (miss a duplicate rather than merge two real charges). Rules-as-tags rather than rewriting postings.

**What I would do if I used it on a year of my own CSVs?** Fix adapters that fail, then make rules move postings. That is usage, not a new product phase.

---

## Honest gaps (say them before they do)

- Four **fixture** adapters, not production bank coverage.
- Rules do not recategorize postings.
- No FX, no inter-account transfer matching, no OFX/QIF.
- Eval is synthetic.
- Not dogfooded on a full year of live statements.
- Install story is `cargo run`, not a packaged binary.

Do **not** call it “the best open-source finance tool.” Call it a **correct, auditable local import engine** with proofs.

---

## 10-minute live demo (order)

1. Open `fixtures/csv/generic/sample.csv` — noisy Amazon / Starbucks.
2. `init` → `import` → import **same file again** (idempotent).
3. `tx add` duplicate Starbucks → `dedupe` → show `duplicate_of`.
4. `rules apply` → tags, mention posting limitation.
5. `reconcile` 2409.20 USD → open the markdown proof.
6. `why <tx-id>`.
7. `verify` / `rebuild` — same `ledger_hash`.
8. `export --format beancount`.
9. Optional: break an unbalanced txn in a test; show it fails construction.

---

## One-page stack cheat sheet

| Layer | Library |
|-------|---------|
| CLI | clap 4 |
| JSON/YAML | serde, serde_json, serde_yaml |
| DB | rusqlite 0.32 bundled |
| Time | chrono (NaiveDate for books; Utc for events) |
| Money | rust_decimal |
| Hash | sha2 + hex |
| IDs | uuid v7 (manual/events), SHA-256→UUID (imports) |
| CSV | csv crate, flexible + trim |
| Test | pretty_assertions, proptest, tempfile |
| CI | GitHub Actions: fmt, clippy `-D warnings`, test, demo smoke (Win + Ubuntu) |

Schema version: **3**. Workspace edition 2021, MSRV 1.75.

---

## If they ask for code pointers

| Topic | Where |
|-------|--------|
| Unbalanced reject | `crates/ledgerkit-core/src/verify.rs` |
| Fingerprint / convert | `crates/ledgerkit-import/src/convert.rs` |
| Dedupe | `crates/ledgerkit-import/src/dedupe.rs` |
| Schema | `crates/ledgerkit-store/src/schema.rs` |
| Import skip existing id | `crates/ledgerkit-store/src/import.rs` |
| Rebuild | `crates/ledgerkit-store/src/replay.rs` |
| Row recon | `crates/ledgerkit-store/src/rows.rs` |
| Ending-balance proof | `crates/ledgerkit-core/src/reconcile.rs` |
| Path traversal | `crates/ledgerkit-cli/src/paths.rs` |
| Contract | `docs/final.md`, `docs/identity.md` |

Read `docs/demo-script.md` the night before. Run `.\scripts\demo.ps1` once so the numbers are in your muscle memory.
