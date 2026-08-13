# LedgerKit

A program that runs **only on your computer**. You give it a **bank CSV** (a spreadsheet export). It turns that into a **balanced account book**, keeps a **diary of every change**, and can prove the numbers match a statement.

It does **not** log into your bank. It does **not** need an account. It does **not** send your data anywhere.

```text
your CSV  →  understand the columns  →  clean names / find copies / apply your rules
                                              →  account book + diary (one SQLite file)
                                              →  check against the statement
                                              →  save as CSV / JSON / Beancount
```

**Word list (plain English):** [docs/glossary.md](docs/glossary.md).

---

## What works 100% (try this)

These are checked in the repo and in CI. If they fail, that is a bug.

| Try this | What you should see |
|----------|---------------------|
| `.\scripts\demo.ps1` (Windows) or the commands below | Checking account ends at **2409.20 USD** after import + extra Starbucks + dedupe + reconcile |
| `fixtures/csv/generic/sample.csv` + adapter `generic_csv` | 4 rows import |
| Same file imported **again** | “already imported” — not doubled |
| `fixtures/csv/hdfc/sample.csv` + adapter `hdfc` | 3 rows (INR-style columns, dates like `01/02/26`) |
| `fixtures/csv/credit_card/sample.csv` + adapter `credit_card` | 3 rows |
| `fixtures/csv/generic/malformed.csv` | 1 good row kept, **1 error printed** (not hidden) |
| `ledgerkit verify` / `rebuild` after the demo | `verify: OK` / `rebuild: OK`, same ledger hash |
| `cargo test --workspace` | Tests pass |

**Not guaranteed 100%:** a CSV you download from *your* bank tomorrow. Only the column layouts above (and `custom` if you map columns) are built-in. Real HDFC/SBI/Chase files often add extra header junk. That is an adapter bug/fix, not “the engine is fake.”

**Also not 100% “smart categorize”:** `rules apply` **tags** transactions (`category:…`). It does **not** rewrite the expense account on the posting. Export may still show `expenses:uncategorized` on the money line.

---

## Use it

### 0. Install Rust

You need **Rust 1.75+**. On Windows use the MSVC toolchain ([rustup](https://rustup.rs/)).

```bash
git clone https://github.com/akshaykumarhudedmani/ledgerkit
cd ledgerkit
cargo build -p ledgerkit-cli
```

The binary is `target/debug/ledgerkit` (or `ledgerkit.exe`). Below, `cargo run -p ledgerkit-cli --` means “run that program.”

### 1. One-command demo (Windows)

From the repo root:

```powershell
.\scripts\demo.ps1
```

That creates a folder `.demo`, imports the sample CSVs, dedupes, applies sample rules, reconciles to **2409.20 USD**, exports Beancount/CSV. Open `.demo/reports/` for the proof file.

### 2. Same demo, typed out

```bash
cargo run -p ledgerkit-cli -- init --dir .demo

cargo run -p ledgerkit-cli -- import fixtures/csv/generic/sample.csv \
  --account assets:bank:checking --adapter generic_csv --commodity USD --dir .demo

# optional: HDFC sample
cargo run -p ledgerkit-cli -- import fixtures/csv/hdfc/sample.csv \
  --account assets:bank:hdfc --adapter hdfc --commodity INR --dir .demo

cargo run -p ledgerkit-cli -- dedupe --dir .demo
cargo run -p ledgerkit-cli -- rules apply --file fixtures/rules/default.yaml --dir .demo

cargo run -p ledgerkit-cli -- reconcile --account assets:bank:checking \
  --balance 2409.20 --as-of 2026-01-07 --commodity USD --dir .demo

cargo run -p ledgerkit-cli -- verify --dir .demo
cargo run -p ledgerkit-cli -- balance --account assets:bank:checking --commodity USD --dir .demo
cargo run -p ledgerkit-cli -- export --format beancount --out .demo/ledger.bean --dir .demo
```

`--dir .demo` = “put the database in this folder.” Default if you omit it is `.ledgerkit`.

### 3. Commands you will actually use

| Command | Plain meaning |
|---------|----------------|
| `init` | Create the folder + empty database |
| `import FILE --account … --adapter …` | Read a CSV into the book |
| `adapters` | List built-in CSV layouts |
| `dedupe` | Link copies; never deletes |
| `rules apply --file …` | Tag matching payees from a YAML file |
| `balance --account …` | Add up postings for that account |
| `reconcile --account … --balance … --as-of YYYY-MM-DD` | Does the book match the statement total? |
| `reconcile --account … --rows` | Does each saved statement line have a txn? |
| `why ID` | Diary for a transaction UUID or a statement-row number |
| `verify` | Check the diary chain and that replay matches |
| `rebuild` | Wipe the book tables, rebuild from the diary |
| `export --format json\|beancount\|csv --out FILE` | Write a copy out |
| `tx add` / `account add` | Type a transaction / account by hand |

Full interview walkthrough: [docs/demo-script.md](docs/demo-script.md).

### 4. Your own CSV

1. Look at the first line (headers).
2. Pick an adapter:
   - `Date,Description,Amount` → `generic_csv`
   - HDFC-style `Date,Narration,Withdrawal Amt.,Deposit Amt.` → `hdfc`
   - `Transaction Date,…,Description,Amount` → `credit_card`
   - Something else → `custom` (column mapping) or a new adapter (see Contribute)
3. **Do not commit real statements** to git. Keep them outside the repo.

If parse errors print, that is working as designed: bad rows are listed, good rows still import.

---

## Contribute

The product is **frozen**. Useful PRs: **bugs**, **better fixtures** (anonymized), **docs that match the code**, adapters that parse a **real** layout without adding PDF/Plaid/AI.

1. Fork / branch off `master`.
2. Never commit live bank statements or secrets.
3. Before a PR:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

4. Adapters: implement `BankAdapter` (same file in → same parse out; no clock). Prefer `plugins/` like `plugins/sample-adapter`. Built-ins live in `crates/ledgerkit-import/src/adapters/` and must update `fixtures/golden/parse_counts.json`.

Details: [CONTRIBUTING.md](CONTRIBUTING.md). Out of scope: [docs/final.md](docs/final.md).

---

## Project layout

```text
crates/ledgerkit-core     money, transactions, “does it balance?”
crates/ledgerkit-store    SQLite file, diary, rebuild, why
crates/ledgerkit-import   CSV adapters, clean names, dedupe, rules
crates/ledgerkit-export   JSON / Beancount / CSV
crates/ledgerkit-cli      the `ledgerkit` command
plugins/sample-adapter    example extra adapter
fixtures/                 sample CSVs + tests
docs/                     design, glossary, interview notes
```

---

## Docs

| Doc | For |
|-----|-----|
| [docs/glossary.md](docs/glossary.md) | Every term in English |
| [docs/interview-pitch.md](docs/interview-pitch.md) | Explain the project in an interview |
| [docs/demo-script.md](docs/demo-script.md) | 10-minute live demo |
| [docs/final.md](docs/final.md) | What “done” means |
| [docs/identity.md](docs/identity.md) | How import ids are chosen |
| [docs/design.md](docs/design.md) / [docs/architecture.md](docs/architecture.md) | Design |

---

## License

MIT
