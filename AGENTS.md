# LedgerKit — agent instructions

This is a **local-first Rust** financial data engine (CSV → auditable double-entry ledger).

## Always true

- Read `.cursor/rules/` (auto-applied) and `docs/roadmap.md` for phase scope.
- Never use floats for money; never delete duplicates; never silent-drop CSV rows.
- Prefer failing invariants over shipping a demo that lies about balances.

## Verify before claiming done

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Or run the demo: `scripts/demo.ps1` (Windows) / `make demo` (Unix).

## Quality tooling

- **Hooks** (auto): see `.cursor/hooks.json` — quality gate on agent stop, shell guard, prompt secret block.
- **Manual slash skills**: see `docs/agent-workflow.md` for when to run Bugbot / Security / Split PRs / Autopilot / Loop.

## Repo

https://github.com/akshaykumarhudedmani/ledgerkit
