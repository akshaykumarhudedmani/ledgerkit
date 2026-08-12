# LedgerKit

**Local-first financial data engine** that turns messy bank exports into an auditable, double-entry ledger with deterministic transforms, reconciliation proofs, and exports — with every decision explainable.

```text
Bank CSV  →  adapters  →  normalize / dedupe / rules  →  double-entry ledger
                                                              ↓
                                                         event log (SQLite)
                                                              ↓
                                                    CSV / JSON / Beancount
```

## Pillars

| Pillar | Meaning |
|--------|---------|
| Local-first | No cloud account, no paid bank API, no SaaS dependency |
| Deterministic | Same input + rules ⇒ same output |
| Auditable | Append-only event log; every mutate is explainable |
| Correct money math | `Decimal` only — never floats |
| Double-entry | Every transaction balances; invariants fail the build |
| Reconciliation proofs | Prove imports match a statement ending balance |
| Library + CLI | Embeddable crates and `ledgerkit` CLI |

## Quick start

**Requirements:** Rust 1.75+ (MSVC toolchain on Windows).

```bash
cargo build -p ledgerkit-cli
cargo test --workspace

# Phase 2 ledger demo
cargo run -p ledgerkit-cli -- init --dir .demo
cargo run -p ledgerkit-cli -- import fixtures/csv/generic/sample.csv \
  --account assets:bank:checking --adapter generic_csv --commodity USD --dir .demo
cargo run -p ledgerkit-cli -- dedupe --dir .demo
cargo run -p ledgerkit-cli -- rules apply --file fixtures/rules/default.yaml --dir .demo
cargo run -p ledgerkit-cli -- verify --dir .demo
cargo run -p ledgerkit-cli -- balance --account assets:bank:checking --commodity USD --dir .demo
```

On Windows PowerShell:

```powershell
.\scripts\demo.ps1
```

Interview talk track (10 min): [docs/demo-script.md](docs/demo-script.md).

## CLI (v1 surface)

```text
ledgerkit init
ledgerkit account add --id assets:bank:hdfc --type asset --commodity INR
ledgerkit tx add --date 2026-03-01 --payee Cafe \
  --posting assets:bank:hdfc=-250:INR --posting expenses:food=250:INR
ledgerkit balance --account assets:bank:hdfc
ledgerkit verify
ledgerkit replay [--through N]
ledgerkit import ./statement.csv --account assets:bank:hdfc --adapter hdfc
ledgerkit rules apply --file fixtures/rules/default.yaml
ledgerkit dedupe
ledgerkit reconcile --account assets:bank:checking --balance 2409.20 --as-of 2026-01-07 --commodity USD
ledgerkit why <tx-id>
ledgerkit export --format beancount --out ledger.bean
ledgerkit export --format csv --out ledger.csv
ledgerkit adapters
```

## Workspace layout

```text
crates/
  ledgerkit-core/     # money, accounts, postings, invariants, events
  ledgerkit-store/    # SQLite append-only event log
  ledgerkit-import/   # BankAdapter trait + built-in CSV adapters
  ledgerkit-export/   # JSON + Beancount exporters
  ledgerkit-cli/      # clap CLI binary `ledgerkit`
plugins/
  sample-adapter/     # proves external plugin SDK surface
docs/                 # design, threat model, schema, roadmap
fixtures/             # anonymized sample CSVs + golden outputs
```

## Built-in adapters (Phase 3 targets)

- `hdfc` — HDFC Bank CSV (India)
- `generic_csv` — US/EU-style Date/Description/Amount
- `credit_card` — credit-card Transaction Date/Description/Amount
- `custom` — column-mapping adapter

## Quality tooling

- Agent rules/hooks: [docs/agent-workflow.md](docs/agent-workflow.md) and `AGENTS.md`
- CI: GitHub Actions on every push

## Status

**Phase 4 (Dedupe + rules):** done — exact/near-window dedupe (`duplicate_of`, never delete) and YAML rules with conflict reporting + labeled metrics.

**Phase 5 (Reconcile + why):** done — statement proof reports under `reports/` and `ledgerkit why <tx-id>` event chains.

**Phase 6 (Export + polish):** done — Beancount `commodity`/`open`/metadata, CSV export, interview demo script.

**Phase 7 (Hardening):** in review — CSV fuzz, 100k-row bench, path-traversal tests, [eval chapter](docs/eval.md).

See [docs/roadmap.md](docs/roadmap.md), [docs/design.md](docs/design.md), and [docs/eval.md](docs/eval.md).

## Non-goals (v1)

Mobile apps, consumer budgeting UI, PDF/OCR spine, Plaid, tax filing, multi-user cloud sync, “AI categorizes everything” as the main claim.

## License

MIT
