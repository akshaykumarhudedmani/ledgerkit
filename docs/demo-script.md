# LedgerKit interview demo (~10 minutes)

Talk track for a live walkthrough. Run from the repo root. On Windows: `.\scripts\demo.ps1` covers the same pipeline unattended.

**Props:** `fixtures/csv/generic/sample.csv` (noisy Amazon / Starbucks / payroll) and this repo’s tests.

---

## 0. Setup (30s)

```powershell
cargo build -p ledgerkit-cli
.\scripts\demo.ps1
```

Or step through the commands below with `--dir .demo`.

---

## 1. Dirty merchants (45s)

Open `fixtures/csv/generic/sample.csv`. Point at:

- `AMZN MKTP US*ABC123` vs `Amazon Marketplace` (same economic payee, different strings)
- `STARBUCKS STORE 12345`

LedgerKit normalizes deterministically; it does **not** silently merge different merchant keys.

---

## 2. Import → normalize → dedupe (2 min)

```powershell
cargo run -p ledgerkit-cli -- init --dir .demo
cargo run -p ledgerkit-cli -- import fixtures/csv/generic/sample.csv `
  --account assets:bank:checking --adapter generic_csv --commodity USD --dir .demo
# same file again → idempotent (no second post)
cargo run -p ledgerkit-cli -- import fixtures/csv/generic/sample.csv `
  --account assets:bank:checking --adapter generic_csv --commodity USD --dir .demo
cargo run -p ledgerkit-cli -- tx add --date 2026-01-03 --payee "STARBUCKS STORE 12345" `
  --posting "assets:bank:checking=-6.50:USD" --posting "expenses:uncategorized=6.50:USD" --dir .demo
cargo run -p ledgerkit-cli -- dedupe --dir .demo
cargo run -p ledgerkit-cli -- rules apply --file fixtures/rules/default.yaml --dir .demo
```

Call out: duplicates get `duplicate_of` + a `Deduped` event — **never deleted**. Rules only add `category:` tags.

---

## 3. Double-entry + verify (1 min)

```powershell
cargo run -p ledgerkit-cli -- verify --dir .demo
cargo run -p ledgerkit-cli -- balance --account assets:bank:checking --commodity USD --dir .demo
```

Expect `2409.20 USD` after the duplicate is skipped. `verify` checks posting balance, event hash chain, and replay == materialized ledger hash.

Show a failing invariant (optional): `cargo test -p ledgerkit-core --lib transaction::tests::rejects_unbalanced_transaction`.

---

## 4. Reconcile proof (1 min)

```powershell
cargo run -p ledgerkit-cli -- reconcile --account assets:bank:checking `
  --balance 2409.20 --as-of 2026-01-07 --commodity USD --dir .demo
```

Open `.demo/reports/reconcile-assets_bank_checking-2026-01-07.md`. Unexplained delta must be `0`. Included postings vs skipped duplicates are listed.

---

## 5. Break a rule on purpose (45s)

`cargo test -p ledgerkit-core --lib transaction::tests::rejects_unbalanced_transaction` — construction rejects unbalanced txns. Mention `account_balance` errors on overflow (no silent wrap).

---

## 6. Export Beancount; balances match (1 min)

```powershell
cargo run -p ledgerkit-cli -- export --format beancount --out .demo/ledger.bean --dir .demo
cargo run -p ledgerkit-cli -- export --format csv --out .demo/ledger.csv --dir .demo
```

Open `ledger.bean`: `commodity` / `open` directives, `Assets:Bank:Checking` names, `id` / `category` metadata, duplicates omitted. Same checking balance as `ledgerkit balance`.

---

## 7. `why` audit chain (1 min)

Use the `posted tx=` id from the injected Starbucks `tx add`:

```powershell
cargo run -p ledgerkit-cli -- why <tx-id> --dir .demo
```

Expect `posted` → `deduped` → `reconciled` (only if `date <= as_of`).

---

## 8. Replay hash (30s)

```powershell
cargo run -p ledgerkit-cli -- replay --dir .demo
```

`ledger_hash` matches `verify`’s `replay_hash`. Same input + rules ⇒ same output; event `at` timestamps are not in the ledger content hash.

---

## Close

Local-first: no Plaid, no telemetry. Pillars: decimal money, double-entry, append-only events, no silent CSV drops, duplicates never deleted.
