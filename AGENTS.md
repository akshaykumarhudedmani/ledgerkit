# LedgerKit

Local-first Rust engine: bank CSV → auditable double-entry ledger.

Invariants (do not break):

- No floats for money (`rust_decimal` only).
- Unbalanced transactions never enter the ledger.
- CSV row failures are reported; never dropped silently.
- Duplicates get `duplicate_of`; never deleted.
- Balances come from postings only (skip duplicates).
- Finished product: [docs/final.md](docs/final.md). Bug fixes only.

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Demo: `scripts/demo.ps1` (Windows) / `make demo` (Unix).

https://github.com/akshaykumarhudedmani/ledgerkit
