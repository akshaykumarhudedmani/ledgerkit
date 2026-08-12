# Threat Model (local application)

LedgerKit runs entirely on the user's machine. There is no cloud control plane.

## Assets

1. Bank statement files (PII + financial history)
2. Derived ledger database (`.ledgerkit/ledger.sqlite`)
3. Import artifacts and reconciliation proofs
4. User trust in balances and categorization explanations

## Adversaries / hazards

| Threat | Impact | Mitigations |
|--------|--------|-------------|
| Malware on host | Can read local ledger/statements | Out of scope to defeat OS malware; document that LedgerKit is not an encrypted vault (optional SQLCipher stretch) |
| Corrupt / malicious CSV | Wrong books, DoS via huge files | Schema validation, row caps (later), never execute CSV content, deterministic parsers |
| Silent data loss | Missing transactions | Append-only events, artifact checksums, `verify`, import reports |
| Path traversal on export/import paths | Overwrite unexpected files | Reject `..` on export `--out`; proofs stay under `reports/` |
| Accidental float rounding | Cent-level drift | `Decimal` only; proptest invariants |
| Supply-chain dependency compromise | Malicious build | Minimal deps, lockfile, CI, prefer `rusqlite` bundled |

## Non-threats (v1)

- Remote attackers over the network (no server)
- Multi-tenant isolation (single-user local tool)
- Hardware key exfiltration

## Security hygiene checklist

- [x] No telemetry in config defaults
- [x] Secrets never required
- [x] Source artifact SHA-256 on import
- [x] Path traversal unit tests
- [x] Optional read-only verify mode documentation (`ledgerkit verify` does not mutate)
- [ ] Dependency audit in CI (`cargo deny` optional)

## Privacy promise

LedgerKit does not phone home. All processing is local. Users may delete `.ledgerkit/` to wipe derived state; original CSVs remain under user control.
