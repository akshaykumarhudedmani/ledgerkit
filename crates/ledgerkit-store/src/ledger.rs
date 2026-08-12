use anyhow::{bail, Context, Result};
use ledgerkit_core::{
    verify_transaction, Account, AccountId, AccountType, Amount, Commodity, Event, EventKind,
    EventPayload, LedgerSnapshot, Posting, Transaction, TransactionId,
};
use uuid::Uuid;

use crate::events::append_sealed_event;
use crate::Store;

impl Store {
    pub fn upsert_account(&mut self, account: Account) -> Result<Event> {
        let prev = self.last_event_hash()?;
        let event = Event::seal(
            EventKind::AccountUpserted,
            EventPayload::AccountUpserted {
                account: account.clone(),
            },
            prev,
        );

        let db_tx = self.conn.transaction()?;
        insert_account_rows(&db_tx, &account)?;
        let stored = append_sealed_event(&db_tx, event)?;
        db_tx.commit()?;
        Ok(stored)
    }

    pub fn get_account(&self, id: &AccountId) -> Result<Option<Account>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, account_type, commodity, name FROM accounts WHERE id = ?1")?;
        let mut rows = stmt.query([id.as_str()])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Account {
                id: AccountId::new(row.get::<_, String>(0)?).context("account id")?,
                account_type: parse_account_type(&row.get::<_, String>(1)?)?,
                commodity: Commodity::new(row.get::<_, String>(2)?).context("commodity")?,
                name: row.get(3)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Insert a balanced transaction + postings and append a `Posted` event.
    pub fn post_transaction(&mut self, transaction: Transaction) -> Result<Event> {
        verify_transaction(&transaction)?;

        let prev = self.last_event_hash()?;
        let event = Event::seal(
            EventKind::Posted,
            EventPayload::Posted {
                transaction: transaction.clone(),
            },
            prev,
        );

        let db_tx = self.conn.transaction()?;
        insert_transaction_rows(&db_tx, &transaction)?;
        let stored = append_sealed_event(&db_tx, event)?;
        db_tx.commit()?;
        Ok(stored)
    }

    pub fn load_snapshot(&self) -> Result<LedgerSnapshot> {
        let mut stmt = self.conn.prepare(
            "SELECT id, date, payee, merchant_id, narration, import_batch_id, duplicate_of, tags_json
             FROM transactions
             ORDER BY date ASC, id ASC",
        )?;
        let tx_rows = stmt.query_map([], |row| {
            Ok(TxRow {
                id: row.get(0)?,
                date: row.get(1)?,
                payee: row.get(2)?,
                merchant_id: row.get(3)?,
                narration: row.get(4)?,
                import_batch_id: row.get(5)?,
                duplicate_of: row.get(6)?,
                tags_json: row.get(7)?,
            })
        })?;

        let mut transactions = Vec::new();
        for row in tx_rows {
            let row = row?;
            let postings = self.load_postings(&row.id)?;
            transactions.push(decode_transaction(row, postings)?);
        }
        Ok(LedgerSnapshot { transactions })
    }

    fn load_postings(&self, tx_id: &str) -> Result<Vec<Posting>> {
        let mut stmt = self.conn.prepare(
            "SELECT account_id, amount, commodity, memo
             FROM postings
             WHERE transaction_id = ?1
             ORDER BY ordinal ASC",
        )?;
        let rows = stmt.query_map([tx_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        let mut postings = Vec::new();
        for row in rows {
            let (account, amount, commodity, memo) = row?;
            let mut posting = Posting::new(
                AccountId::new(account).context("posting account")?,
                Amount::parse(&amount).context("posting amount")?,
                Commodity::new(commodity).context("posting commodity")?,
            );
            if let Some(m) = memo {
                posting = posting.with_memo(m);
            }
            postings.push(posting);
        }
        Ok(postings)
    }
}

pub(crate) fn insert_account_rows(conn: &rusqlite::Connection, account: &Account) -> Result<()> {
    conn.execute(
        "INSERT INTO accounts (id, account_type, commodity, name)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET
           account_type=excluded.account_type,
           commodity=excluded.commodity,
           name=excluded.name",
        (
            account.id.as_str(),
            account_type_str(account.account_type),
            account.commodity.as_str(),
            account.name.as_str(),
        ),
    )?;
    conn.execute(
        "INSERT INTO commodities (code, decimals) VALUES (?1, 2)
         ON CONFLICT(code) DO NOTHING",
        [account.commodity.as_str()],
    )?;
    Ok(())
}

pub(crate) fn insert_transaction_rows(
    conn: &rusqlite::Connection,
    transaction: &Transaction,
) -> Result<()> {
    let tags_json = serde_json::to_string(&transaction.tags)?;
    conn.execute(
        "INSERT INTO transactions
           (id, date, payee, merchant_id, narration, import_batch_id, duplicate_of, tags_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            transaction.id.to_string(),
            transaction.date.format("%Y-%m-%d").to_string(),
            transaction.payee.as_str(),
            transaction.merchant_id.map(|m| m.to_string()),
            transaction.narration.as_deref(),
            transaction.import_batch_id.map(|b| b.to_string()),
            transaction.duplicate_of.map(|d| d.to_string()),
            tags_json,
        ),
    )?;

    for (ordinal, posting) in transaction.postings.iter().enumerate() {
        conn.execute(
            "INSERT INTO postings
               (transaction_id, account_id, amount, commodity, memo, ordinal)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                transaction.id.to_string(),
                posting.account.as_str(),
                posting.amount.to_string(),
                posting.commodity.as_str(),
                posting.memo.as_deref(),
                ordinal as i64,
            ),
        )?;
    }
    Ok(())
}

struct TxRow {
    id: String,
    date: String,
    payee: String,
    merchant_id: Option<String>,
    narration: Option<String>,
    import_batch_id: Option<String>,
    duplicate_of: Option<String>,
    tags_json: String,
}

fn decode_transaction(row: TxRow, postings: Vec<Posting>) -> Result<Transaction> {
    let id = TransactionId::from_uuid(Uuid::parse_str(&row.id).context("tx id")?);
    let date = chrono::NaiveDate::parse_from_str(&row.date, "%Y-%m-%d").context("tx date")?;
    let tags: Vec<String> = serde_json::from_str(&row.tags_json).unwrap_or_default();
    Ok(Transaction {
        id,
        date,
        payee: row.payee,
        merchant_id: row
            .merchant_id
            .map(|s| {
                Ok::<_, anyhow::Error>(ledgerkit_core::MerchantId::from_uuid(Uuid::parse_str(&s)?))
            })
            .transpose()?,
        narration: row.narration,
        postings,
        import_batch_id: row
            .import_batch_id
            .map(|s| {
                Ok::<_, anyhow::Error>(ledgerkit_core::ImportBatchId::from_uuid(Uuid::parse_str(
                    &s,
                )?))
            })
            .transpose()?,
        duplicate_of: row
            .duplicate_of
            .map(|s| Ok::<_, anyhow::Error>(TransactionId::from_uuid(Uuid::parse_str(&s)?)))
            .transpose()?,
        tags,
    })
}

fn account_type_str(t: AccountType) -> &'static str {
    match t {
        AccountType::Asset => "asset",
        AccountType::Liability => "liability",
        AccountType::Equity => "equity",
        AccountType::Income => "income",
        AccountType::Expense => "expense",
    }
}

fn parse_account_type(s: &str) -> Result<AccountType> {
    Ok(match s {
        "asset" => AccountType::Asset,
        "liability" => AccountType::Liability,
        "equity" => AccountType::Equity,
        "income" => AccountType::Income,
        "expense" => AccountType::Expense,
        other => bail!("unknown account type: {other}"),
    })
}
