# Architecture

```text
                    ┌─────────────────────────┐
                    │      ledgerkit-cli      │
                    └───────────┬─────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌───────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ledgerkit-import│     │ ledgerkit-core  │     │ledgerkit-export │
│ BankAdapter    │────▶│ Money/Txn/Verify│◀────│ JSON/Beancount  │
│ normalize/...  │     │ Events          │     └─────────────────┘
└───────┬───────┘     └────────┬────────┘
        │                      │
        │                      ▼
        │             ┌─────────────────┐
        └────────────▶│ ledgerkit-store │
                      │ SQLite WAL      │
                      └─────────────────┘
```

## Plugin surface

- **Adapters** implement `BankAdapter::parse(bytes) -> (RawTransactions, ParseReport)`.
- **Exporters** implement `Exporter::export(&LedgerSnapshot) -> String` (`json`, `beancount`, `csv`). Duplicates are omitted. Beancount emits `commodity`/`open` directives and transaction metadata.
- In-repo proof: `plugins/sample-adapter`.

## Determinism rules

1. Parsing must not depend on wall clock (except event timestamps, excluded from ledger content hash).
2. Hash inputs are canonical JSON / raw bytes.
3. Rule application order is explicit and stable.
