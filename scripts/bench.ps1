# 100k-row import bench (Phase 7)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not $Root) { $Root = Get-Location }
Set-Location $Root
$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path

Write-Host "==> 1k-row scale test (CI)"
cargo test -p ledgerkit-import --test scale import_1k_rows_parses_and_converts

Write-Host "==> 100k-row bench (release; ignored in CI)"
cargo test -p ledgerkit-import --release --test scale -- --ignored --nocapture import_100k_rows_bench
