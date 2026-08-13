# Contributing

This repository is a **finished product**. Useful PRs: bug fixes, anonymized fixtures, docs that match the code, adapters for a CSV layout you actually have.

Not useful: mobile app, PDF/OCR, Plaid, AI categorize, cloud — see [docs/final.md](docs/final.md).

**Never commit real bank statements.** Copy them, strip names/account numbers, or keep them off git.

## How to send a change

1. Branch from `master` (or the open freeze PR if you are asked to).
2. Make the smallest change that proves the fix (a test that failed, then passes).
3. Run:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

4. Open a pull request. Describe **what broke** and **how you know it is fixed**.

## Adding a CSV adapter

1. Implement `ledgerkit_import::BankAdapter`: `parse(bytes)` must be **deterministic** (no clock in the output).
2. Prefer a crate under `plugins/` that depends only on public traits (see `plugins/sample-adapter`).
3. Built-in adapters: `crates/ledgerkit-import/src/adapters/`. Update `fixtures/golden/parse_counts.json`.
4. Put a tiny anonymized sample under `fixtures/csv/<name>/`.

## Money and audit (do not break these)

- Amounts: `rust_decimal` only — never `f32`/`f64`.
- Unbalanced transactions must fail construction.
- Import row failures go in `ParseReport` / `statement_rows`; never drop a row with no error.
- Duplicates get `duplicate_of`; never delete.

Plain-language terms: [docs/glossary.md](docs/glossary.md). How to run the demo: [README.md](README.md).
