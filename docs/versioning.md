# Versioning

LedgerKit is **complete**. The crate version stays `0.1.x` for bug-fix releases only.

| What | Compatibility |
|------|----------------|
| SQLite `schema_version` | Integer; migrate forward only (never silent rewrite of `events`) |
| Fingerprint string | Prefixed `v1\|`; a future `v2` would be a new prefix, not a silent change |
| Event payload JSON | Additive fields with `serde` defaults; do not rename existing tags |
| CLI | Additive flags/commands; do not change meaning of existing flags |

There is no public crates.io API promise beyond this repo.
