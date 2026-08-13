# Words in normal English

If a sentence in the pitch or README sounds like nonsense, look it up here. These are the same ideas as in accounting / this repo — said simply.

## Money and the books

**Ledger** — The account book. In LedgerKit it is the list of transactions and postings stored in a file on your disk.

**Double-entry** — Every movement of money is written **twice**: money leaves one account and enters another. The two sides must cancel. Example: coffee costs ₹250 → bank −250 and expenses +250. If they do not cancel, the transaction is **unbalanced** and LedgerKit **rejects** it.

**Posting** — One line of a transaction: “this account, this amount, this currency.” A transaction needs at least two postings.

**Transaction (txn)** — One economic event (a coffee, a salary credit) made of postings that balance.

**Account** — A named bucket, like `assets:bank:checking` or `expenses:food`. Hierarchy uses colons (`assets` → `bank` → `checking`).

**Chart of accounts** — The full list of buckets you created (`account add`).

**Commodity** — The currency code: `INR`, `USD`. LedgerKit does **not** convert INR to USD for you.

**Amount** — A money number stored as a precise decimal (`rust_decimal`), never a normal computer float (`f32`/`f64`). Floats cannot store 0.10 exactly, so books would drift by paise/cents.

**Balance** — How much is in one account. LedgerKit does **not** store a running total as truth. It **adds up all postings** each time (skipping duplicates).

**Payee** — Who the bank said you paid / got money from (raw text from the CSV).

**Merchant / canonical_key** — Cleaned version of the payee, e.g. `AMZN MKTP US*ABC123` → `amzn_mktp_us`. Used for matching, not for pretty display.

**Offset account** — The other side of the bank posting. A withdrawal hits `expenses:…`; a deposit hits `income:…`. Until rules rewrite postings (they currently **don’t**), that is often `expenses:uncategorized`.

**Tag** — A label stuck on a transaction, like `category:expenses:shopping`. Rules add tags; they do not move the money lines.

---

## Files and import

**CSV** — A spreadsheet saved as text (columns separated by commas). Bank “statements” you download are often CSV.

**Statement** — One download from the bank covering some dates. May overlap the previous download.

**Adapter** — A small parser that understands **one** CSV layout (HDFC columns vs “Date,Description,Amount”). `ledgerkit adapters` lists them.

**Parse** — Read bytes and turn rows into dates/amounts/text. A bad row is a **parse error**.

**Convert** — Turn a parsed row into a balanced double-entry transaction. A bad date after parse is a **convert error**.

**ParseReport** — The scorecard after parse: how many rows OK, how many failed, the error messages. Failures must be **reported**, never thrown away quietly (**no silent drop**).

**Fixture** — A fake sample file in `fixtures/` used by tests and the demo. Not your real bank file.

**Golden** — Expected counts we freeze in JSON so a parser change cannot silently “lose” a row.

**Artifact** — A copy of the exact file you imported, saved under `.ledgerkit/artifacts/` with a hash in the name.

**SHA-256** — A fingerprint of the **file bytes**. Same file → same hash. Used so importing the same file twice is a no-op.

**Idempotent** — Doing it twice does not double the books. Same CSV + same adapter + same account → second import says “already imported.”

**Source ref** — A label like `generic:row:2` meaning “this came from row 2 of that adapter.” Two real charges the same day stay distinct if their source refs differ.

**Statement row** — One line of the original statement **kept in the database**, even if it failed. Status: `ok`, `parse_error`, or `convert_error`.

**Batch / import_batch** — One import run of one file.

**Identity / row fingerprint** — A string built from adapter, account, date, amount, currency, source refs, merchant. Same inputs → same transaction id. Clock time is **not** included.

**UUIDv7** — A unique id that includes time, used for **manual** `tx add` and for events. Imports do **not** use this for the transaction id.

**Overlapping files** — Jan 1–31 CSV and Jan 15–Feb 15 CSV both contain the same coffee. Identity makes that **one** transaction, not two posts.

---

## Dedupe and rules

**Dedupe** — After transactions exist, find likely copies and **link** them. Never delete.

**duplicate_of** — “This txn is a copy of that survivor.” Balances ignore the copy.

**Exact match** — Same date, account, amount, currency, merchant, source refs.

**Near-window** — Same merchant/amount/account, dates within N days (default 1). For “Netflix posted yesterday vs today.”

**Rules** — YAML/JSON if-then: if payee looks like Amazon, tag `category:expenses:shopping`. **Priority** = which rule wins. Two rules same priority = **conflict**, neither applied.

**YAML / JSON** — Text formats for config. YAML is the indented one (`fixtures/rules/default.yaml`).

---

## Events, hash, rebuild

**Event / event log** — A diary that only **appends**. “Account created”, “transaction posted”, “marked duplicate”, “reconciled”. Never edit an old page.

**Append-only** — Insert new rows only. No UPDATE/DELETE on `events`.

**seq** — Event number 1, 2, 3… in order.

**Hash (SHA-256 of an event)** — A checksum of this event plus the previous checksum. If someone tampers with history, `verify` notices the **chain** broke.

**Ledger content hash** — Checksum of the **books** (the transactions), not of the diary timestamps. Same books → same hash even if you imported at 3pm vs 4pm.

**Wall-clock** — Real time of day. Allowed on events. **Forbidden** inside ledger content hashes and import fingerprints.

**Projection / materialized tables** — The convenient tables (`transactions`, `postings`) built **from** events. Fast to query; events are the source of truth.

**Replay** — Re-read events from the start (or until seq N) and rebuild the books in memory. Must match the tables.

**Rebuild** — Delete those convenient tables, fold events back onto disk, check the content hash still matches.

**why** — “Show me every diary entry about this transaction (or this statement row).”

**Provenance** — Where a fact came from. `why` is provenance.

---

## Reconcile

**Reconcile** — Check the books against the bank.

**Ending balance** — The number printed at the bottom of the statement (“you have $2409.20”).

**as-of** — Only count transactions on or before this date.

**Starting balance** — If your ledger does not include history before the statement, you can pass an opening number (default 0).

**Unexplained delta** — Computed total minus the bank’s number. **0** means they match.

**Proof / proof report** — A markdown file under `reports/` listing which postings were included. You can show it to someone.

**Row reconcile (`--rows`)** — Check each saved statement line has a matching transaction (and list parse/convert failures). Different question than ending balance.

---

## Export and other tools

**Export** — Write the books out as JSON, CSV, or **Beancount**.

**Beancount** — A popular plain-text accounting format / program. LedgerKit **exports into** it; it is not Beancount internally.

**GnuCash / Firefly III / hledger** — Other finance tools. LedgerKit is not a replacement UI; it is the import/audit engine in front.

**Plaid** — A paid company API that talks to banks. LedgerKit does **not** use it. You download CSV yourself.

**CLI** — Command-line program you run in a terminal (`ledgerkit import …`).

**Crate** — One Rust package. This repo is a **workspace** of several crates (core, store, import, export, cli).

**SQLite** — A database that is **one file** (`.ledgerkit/ledger.sqlite`). No server.

**WAL** — Write-Ahead Log: SQLite crash-safety mode. You do not need to explain more than “safer writes.”

**Schema** — The table layout. Version 3 is current.

**Migrate** — Update old files to new columns without rewriting the event diary.

---

## Quality / security words

**Deterministic** — Same inputs + same rules → same outputs. No “it depends what time it is” in the books.

**Invariant** — A rule that must never break (e.g. transactions balance). Tests fail the build if they break.

**Local-first** — Runs on your PC. No login, no cloud copy of your statements.

**Telemetry** — Phone-home analytics. Off. Config says `telemetry: false`.

**Path traversal (`..`)** — A trick like `--out ../../secret`. Export rejects `..`.

**CI** — GitHub automatically runs format, clippy, tests when you push.

**clippy** — Rust linter. CI treats warnings as errors (`-D warnings`).

**proptest / property test** — Generate many random valid transfers and assert they still balance.

**Fuzz** — Throw garbage bytes at adapters; they must not crash.

**Precision / recall** — Dedup quality scores. Ours are 1.00 on **tiny fake examples**, not real banks. Do not brag without that sentence.

**Kernel / freeze** — The engine is done as a product. Bug fixes yes; new product (PDF, app, Plaid) no.

**Dogfood** — Using your own tool on your real data. We have not done a full year of live CSVs in-repo.

---

## Stack names (libraries)

**Rust** — The programming language.

**clap** — Parses `ledgerkit import --account …` flags.

**serde** — Converts structs to/from JSON/YAML.

**rusqlite** — Talks to SQLite from Rust (`bundled` = ships SQLite, no extra install).

**chrono** — Dates. Book dates have **no timezone**. Event times are UTC.

**sha2 / hex** — Hashing and printing hashes.

**csv crate** — Reads CSV.

**regex** — Pattern match for merchant cleanup and rules.
