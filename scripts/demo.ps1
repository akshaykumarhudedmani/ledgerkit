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

Write-Host "==> import generic sample"
cargo run -p ledgerkit-cli -- import fixtures/csv/generic/sample.csv `
  --account assets:bank:checking --adapter generic_csv --dir .demo

Write-Host "==> verify"
cargo run -p ledgerkit-cli -- verify --dir .demo

Write-Host "==> export beancount"
cargo run -p ledgerkit-cli -- export --format beancount --out .demo/ledger.bean --dir .demo

Write-Host "Demo OK. Workspace: $Demo"
