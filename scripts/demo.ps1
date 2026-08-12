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

Write-Host "==> chart of accounts"
cargo run -p ledgerkit-cli -- account add --id assets:cash --type asset --commodity INR --name Cash --dir .demo
cargo run -p ledgerkit-cli -- account add --id expenses:food --type expense --commodity INR --name Food --dir .demo

Write-Host "==> post balanced transaction"
cargo run -p ledgerkit-cli -- tx add --date 2026-03-01 --payee Cafe `
  --posting "assets:cash=-250.00:INR" --posting "expenses:food=250.00:INR" --dir .demo

Write-Host "==> balance + verify + replay"
cargo run -p ledgerkit-cli -- balance --account assets:cash --commodity INR --dir .demo
cargo run -p ledgerkit-cli -- verify --dir .demo
cargo run -p ledgerkit-cli -- replay --dir .demo

Write-Host "==> export beancount"
cargo run -p ledgerkit-cli -- export --format beancount --out .demo/ledger.bean --dir .demo

Write-Host "Demo OK. Workspace: $Demo"
