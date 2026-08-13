# Security

LedgerKit is a **local** CLI. It does not phone home, does not require accounts, and does not talk to banks.

## Trust model

- You run it on files you already have (CSV exports).
- SQLite and `artifacts/` live on disk you control.
- There is **no** encryption-at-rest, SQLCipher, or signed-release pipeline. OS file permissions are the boundary.

## What we harden

- Import **byte-size** cap (default 32 MiB) and CSV **row** cap (200_000).
- Export `--out` rejects `..` path components.
- Reconcile proofs write only under workspace `reports/` with a sanitized filename.
- No floats in money types.
- Events are append-only; `verify` checks the hash chain.

## What we do not claim

- Protection against a malicious user on the same machine.
- Safety of feeding untrusted huge files beyond the caps (DoS is bounded, not eliminated).
- Supply-chain attestation of GitHub Actions or crates.io.

See [docs/threat-model.md](docs/threat-model.md).
