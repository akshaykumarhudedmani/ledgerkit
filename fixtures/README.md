# Fixtures

Anonymized sample CSVs and expected parse counts for CI and the demo.

- `csv/generic/sample.csv` — Date/Description/Amount (USD)
- `csv/generic/malformed.csv` — one good row, one missing amount
- `csv/hdfc/sample.csv` — HDFC-style withdrawal/deposit columns
- `csv/credit_card/sample.csv` — card Transaction Date / Amount
- `golden/parse_counts.json` — expected ok/error row counts
- `rules/default.yaml` — demo categorization rules
- `eval/` — labeled synthetic cases for metrics

**Never commit real account numbers, full legal names, or unredacted statements.**
