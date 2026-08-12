# Contributing

## Principles

1. Money correctness beats features.
2. Every mutate needs an event + explanation.
3. Adapters never silently drop rows.
4. Prefer deterministic rules over opaque ML.

## Dev setup

```bash
rustup default stable
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Adding a bank adapter

1. Implement `BankAdapter` in `crates/ledgerkit-import/src/adapters/`.
2. Register in `adapters::builtin`.
3. Add anonymized fixture under `fixtures/csv/<bank>/`.
4. Add golden expectations when Phase 3 lands.
5. Document columns in a short comment at the top of the adapter file.

## PR checklist

- [ ] Tests for new invariants / parsers
- [ ] No floats in money paths
- [ ] `cargo fmt` + clippy clean
- [ ] Fixtures anonymized (no real account numbers / names)
