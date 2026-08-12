# Roadmap (12–16 weeks)

| Phase | Weeks | Deliverable | Status |
|-------|-------|-------------|--------|
| 1. Spec | 1–2 | Design doc + schema + invariants + repo skeleton | **Current** |
| 2. Ledger core | 2–3 | Postings persistence, balances, event append/replay, `verify` | Planned |
| 3. Import + adapters | 2–3 | HDFC + generic + credit card + custom; golden fixtures | Stubbed |
| 4. Dedupe + rules | 2 | Explainable engines + metrics | Stubbed CLI |
| 5. Reconcile + why | 2 | Proof reports + `why` command | Stubbed CLI |
| 6. Export + polish | 1–2 | Beancount depth + README demo video script | JSON/Beancount stub |
| 7. Hardening | 2 | Fuzzing, benchmarks, thesis eval chapter | Planned |

## Demo script (interview, ~10 min)

1. Show dirty CSV merchants that look different  
2. Import → normalize → dedupe with explanations  
3. Show double-entry and `verify`  
4. Reconcile to statement balance; show proof report  
5. Break a rule on purpose; show invariant failure in tests  
6. Export to Beancount; balances match  
7. `why tx_…` full audit chain  
8. Replay event log; identical content hash  
