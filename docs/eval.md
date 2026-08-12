# Evaluation (thesis chapter notes)

LedgerKit v1 claims are **measured**, not vibed. All figures below are reproducible from this repo with no cloud services.

## 1. Method

| Claim | Instrument | Input |
|-------|------------|--------|
| Dedup quality | `evaluate_dedup_cases` vs `fixtures/eval/dedup_cases.json` | Labeled payee pairs |
| Categorization | `evaluate_rules` vs `fixtures/eval/rules_cases.json` + `fixtures/rules/default.yaml` | Labeled payees |
| Parser success | Golden `fixtures/golden/parse_counts.json` | Anonymized CSVs |
| Reconcile | `prove_reconcile` unexplained delta == 0 | Demo statement ending `2409.20 USD` as-of `2026-01-07` |
| Import scale | `cargo test -p ledgerkit-import --test scale -- --ignored --nocapture` | Synthetic 100k-row generic CSV |
| Parser robustness | proptest fuzz in `tests/fuzz_adapters.rs` | Random bytes / UTF-8 |

Money amounts never use `f32`/`f64`. Metric ratios (precision/recall) are the only floats, and they are not stored in the ledger.

## 2. Dedup (labeled)

Fixture: 4 pairs (exact same merchant, near-window Netflix, different merchants, different amounts).

| Metric | Result (this repo) |
|--------|-------------------|
| Precision | **1.00** (0 false positives) |
| Recall | **1.00** (0 false negatives) |

Policy: exact fingerprint includes source refs; same-day distinct import rows are not merged; duplicates are linked with `duplicate_of`, never deleted.

Reproduce: `cargo test -p ledgerkit-import --lib dedupe::tests::labeled_fixture_precision`

## 3. Rules (labeled)

Fixture: 5 payees (Amazon, Netflix, Uber, payroll, unmatched random).

| Metric | Result (this repo) |
|--------|-------------------|
| Precision | **1.00** |
| Recall | **1.00** |
| Unmatched (expected none) | 1/5 (`RANDOM MERCHANT XYZ`) |

Conflicts at equal priority are reported and **not** applied.

Reproduce: `cargo test -p ledgerkit-import --lib rules::tests::labeled_fixture_accuracy`

## 4. Parser success by adapter

| Adapter | ok_rows | error_rows | Notes |
|---------|---------|------------|--------|
| `generic_csv` sample | 4 | 0 | |
| `hdfc` sample | 3 | 0 | |
| `credit_card` sample | 3 | 0 | |
| `generic_csv` malformed | 1 | 1 | Bad row reported, good row kept |

Hardening: `parse_simple_csv` rejects more than **200_000** data rows (`AdapterError::Schema`). Fuzz: adapters must not panic on random bytes (64 proptest cases × garbage + UTF-8 + CSV-shaped noise).

## 5. Reconciliation

Demo workspace after import + duplicate Starbucks + dedupe:

- Computed ending `assets:bank:checking` = **2409.20 USD**
- Stated ending 2409.20 as-of 2026-01-07
- Unexplained delta **0**
- Proof: `.demo/reports/reconcile-assets_bank_checking-2026-01-07.md`

Success rate on the checked-in demo statement: **1 / 1**. Broader N-statement rates need user CSVs (not committed).

## 6. Import latency (100k rows)

Default CI runs a **1_000-row** parse+convert test. The 100k bench is ignored in CI and should be timed in **release**:

```powershell
.\scripts\bench.ps1
```

Sample (debug, this workspace): parse_100k_ms ≈ **514**, convert_100k_ms ≈ **173124** (UUID + balance checks per txn). Release is the number to cite in the thesis.

## 7. Path traversal

Export `--out` rejects any `..` component (`crates/ledgerkit-cli/src/paths.rs`). Reconcile proofs already write only under `reports/` with a sanitized filename.

## 8. Limits / non-claims

- Metrics are on **small labeled fixtures**, not a bank-scale corpus.
- No ML categorization; rules only.
- No Plaid; no telemetry.
- `cargo deny` / `cargo audit` remain optional (lockfile + bundled rusqlite).
