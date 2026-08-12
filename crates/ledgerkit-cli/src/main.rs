mod paths;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::{NaiveDate, Utc};
use clap::{Parser, Subcommand};
use ledgerkit_core::{
    account_balance, prove_reconcile, verify_ledger, Account, AccountId, AccountType, Amount,
    Commodity, ImportBatchId, Posting, ReconcileRequest, Transaction, TransactionId,
};
use ledgerkit_export::{BeancountExporter, CsvExporter, Exporter, JsonExporter};
use ledgerkit_import::adapters::{self, list_builtin};
use ledgerkit_import::{
    apply_rules, convert_raw, plan_dedupe, ConvertOptions, DedupeOptions, RuleSet, MAX_IMPORT_BYTES,
};
use ledgerkit_store::{ImportBatchSpec, ImportOutcome, StatementRowSpec, Store};

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
    /// Reconcile an account to a statement ending balance and write a proof report
    Reconcile {
        #[arg(long)]
        account: String,
        #[arg(long)]
        balance: Option<String>,
        #[arg(long = "as-of")]
        as_of: Option<String>,
        #[arg(long, default_value = "INR")]
        commodity: String,
        /// Optional opening balance (default 0; use when the ledger does not contain earlier history)
        #[arg(long)]
        starting: Option<String>,
        /// Also (or only) match persisted statement rows to imported transactions
        #[arg(long)]
        rows: bool,
        #[arg(long, default_value = ".ledgerkit")]
        dir: PathBuf,
    },
    /// Explain the event-log derivation chain for a transaction or statement-row id
    Why {
        id: String,
        #[arg(long, default_value = ".ledgerkit")]
        dir: PathBuf,
    },
    /// Export ledger to json, beancount, or csv (duplicates omitted)
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
    /// Wipe projections and replay the event log (events are never rewritten)
    Rebuild {
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
            account,
            balance,
            as_of,
            commodity,
            starting,
            rows,
            dir,
        } => cmd_reconcile(
            &dir,
            &account,
            balance.as_deref(),
            as_of.as_deref(),
            &commodity,
            starting.as_deref(),
            rows,
        ),
        Commands::Why { id, dir } => cmd_why(&dir, &id),
        Commands::Export { format, out, dir } => cmd_export(&dir, &format, &out),
        Commands::Verify { dir } => cmd_verify(&dir),
        Commands::Rebuild { dir } => cmd_rebuild(&dir),
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

fn proof_report_path(dir: &Path, filename: &str) -> Result<PathBuf> {
    if filename.is_empty()
        || filename.contains("..")
        || filename.contains('/')
        || filename.contains('\\')
        || Path::new(filename).is_absolute()
    {
        bail!("refusing to write proof outside workspace reports/");
    }
    Ok(dir.join("reports").join(filename))
}

fn cmd_reconcile(
    dir: &Path,
    account: &str,
    balance: Option<&str>,
    as_of: Option<&str>,
    commodity: &str,
    starting: Option<&str>,
    rows: bool,
) -> Result<()> {
    ensure_workspace(dir)?;
    let mut store = Store::open(db_path(dir))?;
    if rows {
        let report = store.prove_row_reconcile(account)?;
        let filename = format!("reconcile-rows-{}.md", account.replace(':', "_"));
        let path = proof_report_path(dir, &filename)?;
        fs::create_dir_all(dir.join("reports"))?;
        fs::write(&path, report.to_markdown())?;
        println!(
            "row_reconcile account={} matched={} unmatched_rows={} convert_errors={} parse_errors={} unmatched_txns={} report={}",
            report.account,
            report.matched,
            report.unmatched_rows.len(),
            report.convert_errors.len(),
            report.parse_errors.len(),
            report.unmatched_txns.len(),
            path.display()
        );
        if !report.ok() {
            bail!("statement-row gaps (see {})", path.display());
        }
        println!("row_reconcile: OK");
        if balance.is_none() {
            return Ok(());
        }
    }
    let balance = balance.context("reconcile requires --balance (or pass --rows only)")?;
    let as_of = as_of.context("reconcile requires --as-of (or pass --rows only)")?;
    let as_of_date = NaiveDate::parse_from_str(as_of, "%Y-%m-%d")
        .with_context(|| format!("as-of must be YYYY-MM-DD, got {as_of}"))?;
    let req = ReconcileRequest {
        account: AccountId::new(account)?,
        commodity: Commodity::new(commodity)?,
        as_of: as_of_date,
        stated_ending: Amount::parse(balance)?,
        starting: match starting {
            Some(s) => Amount::parse(s)?,
            None => Amount::zero(),
        },
    };
    let snapshot = store.load_snapshot()?;
    let proof = prove_reconcile(&snapshot, &req)?;
    let filename = proof.proof_filename();
    let path = proof_report_path(dir, &filename)?;
    fs::create_dir_all(dir.join("reports"))?;
    fs::write(&path, proof.to_markdown())?;
    let rel = format!("reports/{filename}");
    let unmatched = proof.skipped_duplicates.len() + proof.after_as_of.len();
    let event = store.record_reconcile(&proof, Some(rel.clone()))?;
    store.assert_replay_matches_materialized()?;
    println!(
        "reconcile account={} as_of={} computed={} stated={} delta={} matched={} unmatched={} event_seq={} report={}",
        proof.account,
        proof.as_of,
        proof.computed_ending,
        proof.stated_ending,
        proof.unexplained_delta,
        proof.matched.len(),
        unmatched,
        event.seq,
        path.display()
    );
    if !proof.ok() {
        bail!(
            "unexplained delta {} {} (see {})",
            proof.unexplained_delta,
            proof.commodity,
            rel
        );
    }
    println!("reconcile: OK");
    Ok(())
}

fn cmd_why(dir: &Path, id: &str) -> Result<()> {
    ensure_workspace(dir)?;
    let store = Store::open(db_path(dir))?;
    let steps = if let Ok(tx) = TransactionId::parse(id) {
        store.why_transaction(tx)?
    } else if let Ok(row_id) = id.parse::<i64>() {
        store.why_statement_row(row_id)?
    } else {
        bail!("invalid id {id} (want transaction UUID or statement-row integer)");
    };
    println!("why {id} steps={}", steps.len());
    for step in steps {
        println!("  seq={} {}: {}", step.seq, step.kind, step.summary);
    }
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
    if bytes.len() > MAX_IMPORT_BYTES {
        bail!(
            "refusing to import {} ({} bytes > {MAX_IMPORT_BYTES} byte cap)",
            path.display(),
            bytes.len()
        );
    }
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
    let statement_rows = statement_specs(&raw, &converted);
    match store.apply_import(spec, accounts, converted.transactions, statement_rows)? {
        ImportOutcome::Applied {
            batch_id,
            posted,
            skipped_existing,
            last_seq,
        } => {
            println!(
                "  posted={posted} skipped_existing={skipped_existing} batch={batch_id} last_event_seq={last_seq}"
            );
        }
        ImportOutcome::Duplicate { batch_id } => {
            println!(
                "  already imported (idempotent) batch={batch_id} (would have posted {posted_n})"
            );
        }
    }
    Ok(())
}

fn statement_specs(
    raw: &ledgerkit_import::RawTransactions,
    converted: &ledgerkit_import::ConvertReport,
) -> Vec<StatementRowSpec> {
    raw.transactions
        .iter()
        .zip(converted.row_outcomes.iter())
        .map(|(row, outcome)| match outcome {
            Ok(tx) => StatementRowSpec {
                row_number: row.row_number as i64,
                date_raw: Some(row.date_raw.clone()),
                amount_raw: Some(row.amount_raw.clone()),
                currency_raw: row.currency_raw.clone(),
                description_raw: Some(row.description_raw.clone()),
                balance_raw: row.balance_raw.clone(),
                source_refs: row.source_refs.clone(),
                fingerprint: tx.row_fingerprint.clone(),
                parse_status: "ok".into(),
                error: None,
                transaction_id: Some(tx.id),
            },
            Err(err) => StatementRowSpec {
                row_number: row.row_number as i64,
                date_raw: Some(row.date_raw.clone()),
                amount_raw: Some(row.amount_raw.clone()),
                currency_raw: row.currency_raw.clone(),
                description_raw: Some(row.description_raw.clone()),
                balance_raw: row.balance_raw.clone(),
                source_refs: row.source_refs.clone(),
                fingerprint: None,
                parse_status: "convert_error".into(),
                error: Some(err.clone()),
                transaction_id: None,
            },
        })
        .collect()
}

fn cmd_rebuild(dir: &Path) -> Result<()> {
    ensure_workspace(dir)?;
    let mut store = Store::open(db_path(dir))?;
    store.verify_event_chain()?;
    let (mat, rep) = store.rebuild_projections()?;
    println!(
        "rebuild transactions={} ledger_hash={} replay_hash={}",
        mat.transaction_count, mat.ledger_hash, rep.ledger_hash
    );
    if mat.ok && mat.ledger_hash == rep.ledger_hash {
        println!("rebuild: OK");
        Ok(())
    } else {
        bail!("rebuild: FAILED");
    }
}

fn cmd_export(dir: &Path, format: &str, out: &Path) -> Result<()> {
    ensure_workspace(dir)?;
    paths::reject_parent_dir(out)?;
    let store = Store::open(db_path(dir))?;
    let snapshot = store.load_snapshot()?;
    let exported = snapshot
        .transactions
        .iter()
        .filter(|t| t.duplicate_of.is_none())
        .count();
    let body = match format {
        "json" => JsonExporter.export(&snapshot)?,
        "beancount" | "bean" => BeancountExporter.export(&snapshot)?,
        "csv" => CsvExporter.export(&snapshot)?,
        other => bail!("unsupported format '{other}' (json|beancount|csv)"),
    };
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out, body)?;
    println!(
        "wrote {} ({} transactions, duplicates omitted)",
        out.display(),
        exported
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
