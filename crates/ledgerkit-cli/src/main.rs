use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use ledgerkit_core::{verify_ledger, LedgerSnapshot};
use ledgerkit_export::{BeancountExporter, Exporter, JsonExporter};
use ledgerkit_import::adapters::{self, list_builtin};
use ledgerkit_import::BankAdapter as _;
use ledgerkit_store::Store;

#[derive(Parser, Debug)]
#[command(name = "ledgerkit")]
#[command(about = "Local-first bank export → auditable double-entry ledger engine", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a new LedgerKit workspace in the current directory
    Init {
        #[arg(long, default_value = ".ledgerkit")]
        dir: PathBuf,
    },
    /// Import a bank statement via an adapter
    Import {
        path: PathBuf,
        #[arg(long)]
        account: String,
        #[arg(long)]
        adapter: String,
        #[arg(long, default_value = ".ledgerkit")]
        dir: PathBuf,
    },
    /// Apply categorization rules (stub)
    Rules {
        #[command(subcommand)]
        action: RulesCmd,
    },
    /// Run deduplication (stub)
    Dedupe {
        #[arg(long, default_value = ".ledgerkit")]
        dir: PathBuf,
    },
    /// Reconcile account to a statement ending balance (stub)
    Reconcile {
        #[arg(long)]
        balance: String,
        #[arg(long = "as-of")]
        as_of: String,
        #[arg(long, default_value = ".ledgerkit")]
        dir: PathBuf,
    },
    /// Explain derivation chain for a transaction (stub)
    Why {
        tx_id: String,
        #[arg(long, default_value = ".ledgerkit")]
        dir: PathBuf,
    },
    /// Export ledger to a target format
    Export {
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = ".ledgerkit")]
        dir: PathBuf,
    },
    /// Verify ledger invariants and checksums
    Verify {
        #[arg(long, default_value = ".ledgerkit")]
        dir: PathBuf,
    },
    /// List built-in adapters
    Adapters,
}

#[derive(Subcommand, Debug)]
enum RulesCmd {
    Apply {
        #[arg(long, default_value = ".ledgerkit")]
        dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { dir } => cmd_init(&dir),
        Commands::Import {
            path,
            account,
            adapter,
            dir,
        } => cmd_import(&dir, &path, &account, &adapter),
        Commands::Rules { action } => match action {
            RulesCmd::Apply { dir } => {
                ensure_workspace(&dir)?;
                println!("rules apply: not implemented yet (Phase 4)");
                Ok(())
            }
        },
        Commands::Dedupe { dir } => {
            ensure_workspace(&dir)?;
            println!("dedupe: not implemented yet (Phase 4)");
            Ok(())
        }
        Commands::Reconcile {
            balance,
            as_of,
            dir,
        } => {
            ensure_workspace(&dir)?;
            println!(
                "reconcile: balance={balance} as_of={as_of} (Phase 5 stub — workspace ok at {})",
                dir.display()
            );
            Ok(())
        }
        Commands::Why { tx_id, dir } => {
            ensure_workspace(&dir)?;
            println!("why {tx_id}: event chain not implemented yet (Phase 5)");
            Ok(())
        }
        Commands::Export { format, out, dir } => cmd_export(&dir, &format, &out),
        Commands::Verify { dir } => cmd_verify(&dir),
        Commands::Adapters => {
            for id in list_builtin() {
                println!("{id}");
            }
            Ok(())
        }
    }
}

fn db_path(dir: &Path) -> PathBuf {
    dir.join("ledger.sqlite")
}

fn ensure_workspace(dir: &Path) -> Result<()> {
    if !db_path(dir).exists() {
        bail!(
            "no LedgerKit workspace at {} — run `ledgerkit init` first",
            dir.display()
        );
    }
    Ok(())
}

fn cmd_init(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    fs::create_dir_all(dir.join("artifacts"))?;
    fs::create_dir_all(dir.join("reports"))?;
    let store = Store::open(db_path(dir))?;
    let config = dir.join("config.json");
    if !config.exists() {
        fs::write(
            &config,
            serde_json::json!({
                "version": 1,
                "default_commodity": "INR",
                "telemetry": false
            })
            .to_string(),
        )?;
    }
    println!(
        "Initialized LedgerKit workspace at {} (schema v{})",
        dir.display(),
        store.schema_version()?
    );
    Ok(())
}

fn cmd_import(dir: &Path, path: &Path, account: &str, adapter_id: &str) -> Result<()> {
    ensure_workspace(dir)?;
    let _store = Store::open(db_path(dir))?;
    let adapter = adapters::builtin(adapter_id)
        .with_context(|| format!("unknown adapter '{adapter_id}' — try `ledgerkit adapters`"))?;
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let (raw, report) = adapter.parse(&bytes)?;

    // Preserve original artifact with checksum (security + audit).
    let hash = ledgerkit_core::ContentHash::sha256_bytes(&bytes);
    let artifact =
        dir.join("artifacts")
            .join(format!("{}-{}.bin", adapter.id(), &hash.as_str()[..16]));
    fs::write(&artifact, &bytes)?;

    println!(
        "imported {} via {} into account {account}",
        path.display(),
        adapter.name()
    );
    println!(
        "  ok_rows={} error_rows={}",
        report.ok_rows, report.error_rows
    );
    println!("  source_sha256={hash}");
    println!("  artifact={}", artifact.display());
    if !report.errors.is_empty() {
        println!("  row errors:");
        for err in report.errors.iter().take(20) {
            println!("    - {err}");
        }
    }
    println!(
        "  parsed {} raw transactions (ledger write: Phase 3)",
        raw.transactions.len()
    );
    Ok(())
}

fn cmd_export(dir: &Path, format: &str, out: &Path) -> Result<()> {
    ensure_workspace(dir)?;
    let _store = Store::open(db_path(dir))?;
    // Phase 2 wires store → snapshot; empty snapshot is valid today.
    let snapshot = LedgerSnapshot::default();
    let body = match format {
        "json" => JsonExporter.export(&snapshot)?,
        "beancount" | "bean" => BeancountExporter.export(&snapshot)?,
        other => bail!("unsupported format '{other}' (json|beancount)"),
    };
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out, body)?;
    println!("wrote {}", out.display());
    Ok(())
}

fn cmd_verify(dir: &Path) -> Result<()> {
    ensure_workspace(dir)?;
    let store = Store::open(db_path(dir))?;
    let snapshot = LedgerSnapshot::default();
    let report = verify_ledger(&snapshot);
    println!("workspace={}", dir.display());
    println!("schema_version={}", store.schema_version()?);
    println!("events={}", store.event_count()?);
    println!("transactions={}", report.transaction_count);
    println!("postings={}", report.posting_count);
    println!("ledger_hash={}", report.ledger_hash);
    if report.ok {
        println!("verify: OK");
        Ok(())
    } else {
        for u in &report.unbalanced {
            println!("  unbalanced: {u}");
        }
        bail!("verify: FAILED");
    }
}
