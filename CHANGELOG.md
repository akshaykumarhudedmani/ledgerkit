# Changelog

## Unreleased (final freeze)

- Document the product as complete: [docs/final.md](docs/final.md), [docs/identity.md](docs/identity.md).
- Deterministic import transaction ids from row fingerprints; skip re-post on overlapping files.
- Persist `statement_rows`; row-level reconcile; `why` for statement row ids.
- `ledgerkit rebuild` replays events onto wiped projections.
- Import file-size cap (32 MiB).
- README how-to-use, [glossary.md](docs/glossary.md), interview brief.
- SECURITY.md, CONTRIBUTING.md, versioning notes.

## 0.1.0 (phases 1–7)

Local-first CSV → double-entry ledger: import adapters, dedupe, rules, ending-balance reconcile, `why`, Beancount/JSON/CSV export, fuzz, benches, eval fixtures.
