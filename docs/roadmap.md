# Roadmap

Phases 1–7 are **done** (merged to `master`). Remaining work in this repo is the **final product freeze** documented in [final.md](final.md) — not a v2.

| Phase | Deliverable | Status |
|-------|-------------|--------|
| 1. Spec | Design + schema + skeleton | **Done** |
| 2. Ledger core | Postings, events, replay, `verify` | **Done** |
| 3. Import + adapters | HDFC + generic + credit card + custom; goldens | **Done** |
| 4. Dedupe + rules | Explainable engines + metrics | **Done** |
| 5. Reconcile + why | Ending-balance proofs + `why` | **Done** |
| 6. Export + polish | Beancount + CSV + demo script | **Done** |
| 7. Hardening | Fuzz, benches, eval chapter | **Done** |
| Final | Identity, statement rows, row recon, rebuild, hygiene | **Done** |

After final: **stop**. Bug fixes only. Do not open a Phase 8 product surface.

## Demo script (interview, ~10 min)

1. Show dirty CSV merchants that look different
2. Import → normalize → dedupe with explanations
3. Show double-entry and `verify`
4. Reconcile to statement balance; show proof report
5. Break a rule on purpose; show invariant failure in tests
6. Export to Beancount; balances match
7. `why tx_…` full audit chain
8. Replay / `rebuild`; identical content hash
