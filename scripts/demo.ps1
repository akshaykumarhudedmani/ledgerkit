# LedgerKit one-command demo (Windows PowerShell)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not $Root) { $Root = Get-Location }
Set-Location $Root

$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path

Write-Host "==> building ledgerkit"
cargo build -p ledgerkit-cli

$Demo = Join-Path $Root ".demo"
if (Test-Path $Demo) { Remove-Item -Recurse -Force $Demo }

Write-Host "==> init"
cargo run -p ledgerkit-cli -- init --dir .demo

Write-Host "==> import generic CSV (writes ledger + event log)"
cargo run -p ledgerkit-cli -- import fixtures/csv/generic/sample.csv `
  --account assets:bank:checking --adapter generic_csv --commodity USD --dir .demo

Write-Host "==> import same file again (idempotent)"
cargo run -p ledgerkit-cli -- import fixtures/csv/generic/sample.csv `
  --account assets:bank:checking --adapter generic_csv --commodity USD --dir .demo

Write-Host "==> import HDFC sample"
cargo run -p ledgerkit-cli -- import fixtures/csv/hdfc/sample.csv `
  --account assets:bank:hdfc --adapter hdfc --commodity INR --dir .demo

Write-Host "==> inject exact duplicate + dedupe + rules"
$addOut = cargo run -p ledgerkit-cli -- tx add --date 2026-01-03 --payee "STARBUCKS STORE 12345" `
  --posting "assets:bank:checking=-6.50:USD" --posting "expenses:uncategorized=6.50:USD" --dir .demo | Out-String
Write-Host $addOut
if ($addOut -notmatch "posted tx=(\S+)") { throw "could not parse posted tx id" }
$TxId = $Matches[1]
cargo run -p ledgerkit-cli -- dedupe --dir .demo
cargo run -p ledgerkit-cli -- rules apply --file fixtures/rules/default.yaml --dir .demo

Write-Host "==> reconcile + why"
cargo run -p ledgerkit-cli -- reconcile --account assets:bank:checking --balance 2409.20 --as-of 2026-01-07 --commodity USD --dir .demo
cargo run -p ledgerkit-cli -- why $TxId --dir .demo

Write-Host "==> balance + verify + replay"
cargo run -p ledgerkit-cli -- balance --account assets:bank:checking --commodity USD --dir .demo
cargo run -p ledgerkit-cli -- verify --dir .demo
cargo run -p ledgerkit-cli -- replay --dir .demo

Write-Host "==> export beancount"
cargo run -p ledgerkit-cli -- export --format beancount --out .demo/ledger.bean --dir .demo

Write-Host "Demo OK. Workspace: $Demo"
