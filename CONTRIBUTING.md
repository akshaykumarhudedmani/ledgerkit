# Contributing

This repository is a **finished product**. Useful PRs: bug fixes, fixture goldens, docs that match the code. Not useful: new product surfaces listed as out of scope in [docs/final.md](docs/final.md).

## Adapter path

1. Implement `ledgerkit_import::BankAdapter` (deterministic `parse`, no wall-clock in output).
2. Prefer a new crate under `plugins/` that depends **only** on public traits (see `plugins/sample-adapter`).
3. Built-in adapters live in `crates/ledgerkit-import/src/adapters/` and must update `fixtures/golden/parse_counts.json`.
4. Never commit real bank statements. Anonymize.

## Checks

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Money and audit

- `rust_decimal` / integer amounts only.
- Unbalanced transactions must fail construction.
- Import row failures go in `ParseReport` / `statement_rows`; never drop silently.
- Duplicates get `duplicate_of`; never delete.
