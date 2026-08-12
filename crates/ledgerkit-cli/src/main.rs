use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::{NaiveDate, Utc};
use clap::{Parser, Subcommand};
use ledgerkit_core::{
    account_balance, verify_ledger, Account, AccountId, AccountType, Amount, Commodity,
    ImportBatchId, Posting, Transaction,
};
use ledgerkit_export::{BeancountExporter, Exporter, JsonExporter};
use ledgerkit_import::adapters::{self, list_builtin};
use ledgerkit_import::{
    apply_rules, convert_raw, plan_dedupe, ConvertOptions, DedupeOptions, RuleSet,
};
use ledgerkit_store::{ImportBatchSpec, ImportOutcome, Store};

#[derive(Parser, Debug)]
#[command(name = "ledgerkit")]
#[command(
    about = "Local-first bank export → auditable double-entry ledger engine",
    long_about = None
)]
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
    /// Manage chart of accounts
    Account {
        #[command(subcommand)]
        action: AccountCmd,
    },
    /// Post balanced transactions
    Tx {
        #[command(subcommand)]
        action: TxCmd,
    },
    /// Show account balance from postings
    Balance {
        #[arg(long)]
        account: String,
        #[arg(long, default_value = "INR")]
        commodity: String,
        #[arg(long, default_value = ".ledgerkit")]
        dir: PathBuf,
    },
    /// Replay event log and print ledger hash (optional --through SEQ)
    Replay {
        #[arg(long)]
        through: Option<u64>,
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
        #[arg(long, default_value = "expenses:uncategorized")]
        offset_expense: String,
        #[arg(long, default_value = "income:uncategorized")]
        offset_income: String,
        #[arg(long, default_value = "INR")]
        commodity: String,
        #[arg(long, default_value = ".ledgerkit")]
        dir: PathBuf,
    },
    /// Apply YAML/JSON categorization rules (tags only; does not rewrite postings)
    Rules {
        #[command(subcommand)]
        action: RulesCmd,
    },
    /// Run deduplication (never deletes; sets duplicate_of)
    Dedupe {
        #[arg(long, default_value_t = 1)]
        window_days: i64,
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
    /// Verify ledger invariants, event chain, and replay hash
    Verify {
        #[arg(long, default_value = ".ledgerkit")]
        dir: PathBuf,
    },
    /// List built-in adapters
    Adapters,
}

#[derive(Subcommand, Debug)]
enum AccountCmd {
    /// Upsert an account into the chart of accounts
    Add {
        #[arg(long)]
        id: String,
        #[arg(long = "type")]
        account_type: String,
        #[arg(long, default_value = "INR")]
        commodity: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, default_value = ".ledgerkit")]
        dir: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum TxCmd {
    /// Post a balanced transaction
    ///
    /// Posting format: account=amount:commodity  (amount signed; at least two required)
    Add {
        #[arg(long)]
        date: String,
        #[arg(long)]
        payee: String,
        #[arg(long = "posting", required = true)]
        postings: Vec<String>,
        #[arg(long, default_value = ".ledgerkit")]
        dir: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum RulesCmd {
    /// Tag matching transactions with `category:<account>` (skips duplicates / already tagged)
    Apply {
        #[arg(long, default_value = "fixtures/rules/default.yaml")]
        file: PathBuf,
        #[arg(long, default_value = ".ledgerkit")]
        dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { dir } => cmd_init(&dir),
        Commands::Account { action } => match action {
            AccountCmd::Add {
                id,
                account_type,
                commodity,
                name,
                dir,
            } => cmd_account_add(&dir, &id, &account_type, &commodity, name.as_deref()),
        },
        Commands::Tx { action } => match action {
            TxCmd::Add {
                date,
                payee,
                postings,
                dir,
            } => cmd_tx_add(&dir, &date, &payee, &postings),
        },
        Commands::Balance {
            account,
            commodity,
            dir,
        } => cmd_balance(&dir, &account, &commodity),
        Commands::Replay { through, dir } => cmd_replay(&dir, through),
        Commands::Import {
            path,
            account,
            adapter,
            offset_expense,
            offset_income,
            commodity,
            dir,
        } => cmd_import(
            &dir,
            &path,
            &account,
            &adapter,
            &offset_expense,
            &offset_income,
            &commodity,
        ),
        Commands::Rules { action } => match action {
            RulesCmd::Apply { dir, file } => cmd_rules_apply(&dir, &file),
        },
        Commands::Dedupe { dir, window_days } => cmd_dedupe(&dir, window_days),
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

fn parse_account_type(s: &str) -> Result<AccountType> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "asset" | "assets" => AccountType::Asset,
        "liability" | "liabilities" => AccountType::Liability,
        "equity" => AccountType::Equity,
        "income" => AccountType::Income,
        "expense" | "expenses" => AccountType::Expense,
        other => bail!("unknown account type '{other}' (asset|liability|equity|income|expense)"),
    })
}

fn cmd_account_add(
    dir: &Path,
    id: &str,
    account_type: &str,
    commodity: &str,
    name: Option<&str>,
) -> Result<()> {
    ensure_workspace(dir)?;
    let mut store = Store::open(db_path(dir))?;
    let account_id = AccountId::new(id)?;
    let account = Account::new(
        account_id.clone(),
        parse_account_type(account_type)?,
        Commodity::new(commodity)?,
        name.unwrap_or(id),
    );
    let event = store.upsert_account(account)?;
    println!(
        "account upserted id={} event_seq={} hash={}",
        account_id, event.seq, event.content_hash
    );
    Ok(())
}

fn parse_posting(spec: &str) -> Result<Posting> {
    // account=amount:commodity
    let (account, rest) = spec
        .split_once('=')
        .with_context(|| format!("invalid posting '{spec}' (want account=amount:commodity)"))?;
    let (amount, commodity) = rest
        .rsplit_once(':')
        .with_context(|| format!("invalid posting '{spec}' (want account=amount:commodity)"))?;
    Ok(Posting::new(
        AccountId::new(account.trim())?,
        Amount::parse(amount.trim())?,
        Commodity::new(commodity.trim())?,
    ))
}

fn cmd_tx_add(dir: &Path, date: &str, payee: &str, posting_specs: &[String]) -> Result<()> {
    ensure_workspace(dir)?;
    let mut store = Store::open(db_path(dir))?;
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .with_context(|| format!("invalid date '{date}' (want YYYY-MM-DD)"))?;
    let mut postings = Vec::new();
    for spec in posting_specs {
        postings.push(parse_posting(spec)?);
    }
    let tx = Transaction::new(date, payee, postings)?;
    let event = store.post_transaction(tx.clone())?;
    println!(
        "posted tx={} event_seq={} postings={} hash={}",
        tx.id,
        event.seq,
        tx.postings.len(),
        event.content_hash
    );
    Ok(())
}

fn cmd_balance(dir: &Path, account: &str, commodity: &str) -> Result<()> {
    ensure_workspace(dir)?;
    let store = Store::open(db_path(dir))?;
    let account_id = AccountId::new(account)?;
    let commodity = Commodity::new(commodity)?;
    let snapshot = store.load_snapshot()?;
    let bal = account_balance(&snapshot, &account_id, &commodity)?;
    println!("{account_id} {bal} {commodity}");
    Ok(())
}

fn cmd_replay(dir: &Path, through: Option<u64>) -> Result<()> {
    ensure_workspace(dir)?;
    let store = Store::open(db_path(dir))?;
    store.verify_event_chain()?;
    let snapshot = match through {
        Some(seq) => store.replay_through(seq)?,
        None => store.replay_all()?,
    };
    let report = verify_ledger(&snapshot);
    println!(
        "replay through={} transactions={} ledger_hash={}",
        through
            .map(|s| s.to_string())
            .unwrap_or_else(|| "all".into()),
        report.transaction_count,
        report.ledger_hash
    );
    if !report.ok {
        for u in &report.unbalanced {
            println!("  unbalanced: {u}");
        }
        bail!("replay verify failed");
    }
    Ok(())
}

fn cmd_dedupe(dir: &Path, window_days: i64) -> Result<()> {
    ensure_workspace(dir)?;
    let mut store = Store::open(db_path(dir))?;
    let snapshot = store.load_snapshot()?;
    let report = plan_dedupe(&snapshot.transactions, DedupeOptions { window_days });
    println!(
        "dedupe planned={} already_linked={}",
        report.links.len(),
        report.skipped_already_linked
    );
    for link in &report.links {
        store.mark_duplicate(
            link.duplicate_id,
            link.survivor_id,
            &link.strategy,
            &link.explanation,
        )?;
        println!(
            "  {} -> {} ({}) {}",
            link.duplicate_id, link.survivor_id, link.strategy, link.explanation
        );
    }
    store.assert_replay_matches_materialized()?;
    Ok(())
}

fn cmd_rules_apply(dir: &Path, file: &Path) -> Result<()> {
    ensure_workspace(dir)?;
    let set = RuleSet::from_path(file).with_context(|| format!("load rules {}", file.display()))?;
    let mut store = Store::open(db_path(dir))?;
    let snapshot = store.load_snapshot()?;
    let report = apply_rules(&snapshot.transactions, &set);
    println!(
        "rules file={} applied={} conflicts={} unmatched={} skipped={}",
        file.display(),
        report.applied.len(),
        report.conflicts.len(),
        report.unmatched,
        report.skipped_already_categorized
    );
    for c in &report.conflicts {
        println!("  conflict: {c}");
    }
    for m in report.applied {
        store.apply_category(
            m.transaction_id,
            &m.category,
            &m.rule_id,
            m.confidence,
            m.reasons.clone(),
        )?;
        println!(
            "  {} => {} (rule {} conf {})",
            m.transaction_id, m.category, m.rule_id, m.confidence
        );
    }
    store.assert_replay_matches_materialized()?;
    Ok(())
}

fn cmd_import(
    dir: &Path,
    path: &Path,
    account: &str,
    adapter_id: &str,
    offset_expense: &str,
    offset_income: &str,
    commodity: &str,
) -> Result<()> {
    ensure_workspace(dir)?;
    let mut store = Store::open(db_path(dir))?;
    let adapter = adapters::builtin(adapter_id)
        .with_context(|| format!("unknown adapter '{adapter_id}' — try `ledgerkit adapters`"))?;
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let (raw, report) = adapter.parse(&bytes)?;

    let hash = ledgerkit_core::ContentHash::sha256_bytes(&bytes);
    let artifact =
        dir.join("artifacts")
            .join(format!("{}-{}.bin", adapter.id(), &hash.as_str()[..16]));
    fs::write(&artifact, &bytes)?;

    println!(
        "parsed {} via {} for account {account}",
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
        println!("  parse errors:");
        for err in report.errors.iter().take(20) {
            println!("    - {err}");
        }
    }

    let bank = AccountId::new(account)?;
    let expense = AccountId::new(offset_expense)?;
    let income = AccountId::new(offset_income)?;
    let commodity = Commodity::new(commodity)?;
    let batch_id = ImportBatchId::new();
    let converted = convert_raw(
        &raw,
        &ConvertOptions {
            bank_account: bank.clone(),
            expense_account: expense.clone(),
            income_account: income.clone(),
            default_commodity: commodity.clone(),
            import_batch_id: batch_id,
        },
    );
    if !converted.errors.is_empty() {
        println!("  convert errors:");
        for err in converted.errors.iter().take(20) {
            println!("    - {err}");
        }
    }
    if converted.transactions.is_empty() {
        bail!(
            "no transactions converted (parse ok_rows={}, convert errors={})",
            report.ok_rows,
            converted.errors.len()
        );
    }

    let accounts = vec![
        Account::new(bank.clone(), AccountType::Asset, commodity.clone(), account),
        Account::new(
            expense,
            AccountType::Expense,
            commodity.clone(),
            offset_expense,
        ),
        Account::new(income, AccountType::Income, commodity, offset_income),
    ];
    let spec = ImportBatchSpec {
        id: batch_id,
        adapter: adapter.id().to_string(),
        account_id: bank.to_string(),
        source_path: path.display().to_string(),
        source_sha256: hash,
        imported_at: Utc::now(),
        row_count: converted.transactions.len() as u64,
    };
    let posted_n = converted.transactions.len();
    match store.apply_import(spec, accounts, converted.transactions)? {
        ImportOutcome::Applied {
            batch_id,
            posted,
            last_seq,
        } => {
            println!("  posted={posted} batch={batch_id} last_event_seq={last_seq}");
        }
        ImportOutcome::Duplicate { batch_id } => {
            println!(
                "  already imported (idempotent) batch={batch_id} (would have posted {posted_n})"
            );
        }
    }
    Ok(())
}

fn cmd_export(dir: &Path, format: &str, out: &Path) -> Result<()> {
    ensure_workspace(dir)?;
    let store = Store::open(db_path(dir))?;
    let snapshot = store.load_snapshot()?;
    let body = match format {
        "json" => JsonExporter.export(&snapshot)?,
        "beancount" | "bean" => BeancountExporter.export(&snapshot)?,
        other => bail!("unsupported format '{other}' (json|beancount)"),
    };
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out, body)?;
    println!(
        "wrote {} ({} transactions)",
        out.display(),
        snapshot.transactions.len()
    );
    Ok(())
}

fn cmd_verify(dir: &Path) -> Result<()> {
    ensure_workspace(dir)?;
    let store = Store::open(db_path(dir))?;
    let tip = store.verify_event_chain()?;
    let (mat, rep) = store.assert_replay_matches_materialized()?;
    println!("workspace={}", dir.display());
    println!("schema_version={}", store.schema_version()?);
    println!("events={}", store.event_count()?);
    println!("transactions={}", store.transaction_count()?);
    println!("postings={}", store.posting_count()?);
    println!("event_tip_hash={tip}");
    println!("ledger_hash={}", mat.ledger_hash);
    println!("replay_hash={}", rep.ledger_hash);
    if mat.ok && mat.ledger_hash == rep.ledger_hash {
        println!("verify: OK");
        Ok(())
    } else {
        for u in &mat.unbalanced {
            println!("  unbalanced: {u}");
        }
        bail!("verify: FAILED");
    }
}
