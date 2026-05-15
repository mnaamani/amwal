// Journal entry posting, balance queries, and trial balance

use std::collections::HashMap;

use diesel::prelude::*;

use crate::errors::{DbErrorKind, LedgerError};
use crate::models::{
    Account, AccountId, AccountType, Balance, JournalEntry, LedgerLine, NewLedgerLine,
    NewLedgerLineInput, Posting,
};

pub struct TrialBalanceReport {
    pub asset: i64,
    pub expense: i64,
    pub liability: i64,
    pub equity: i64,
    pub revenue: i64,
    pub is_balanced: bool,
}

pub fn post_journal_entry(
    conn: &mut PgConnection,
    legs: Vec<NewLedgerLineInput>,
) -> Result<JournalEntry, LedgerError> {
    if legs.len() < 2 {
        return Err(LedgerError::InvalidJournalEntry(
            "a journal entry requires at least two legs".into(),
        ));
    }

    // Double-entry invariant: total debits must equal total credits
    let total_debits: i64 = legs.iter().map(|l| l.posting.debit() as i64).sum();
    let total_credits: i64 = legs.iter().map(|l| l.posting.credit() as i64).sum();
    if total_debits != total_credits {
        return Err(LedgerError::ImbalancedEntry {
            total_debits,
            total_credits,
        });
    }

    conn.transaction::<JournalEntry, LedgerError, _>(|conn| {
        use crate::schema::{accounts, balances, journal_entries, ledger_lines};

        // Deduplicate account IDs for the bulk lookup
        let distinct_ids: Vec<AccountId> = {
            let mut ids: Vec<AccountId> = legs.iter().map(|l| l.account_id).collect();
            ids.sort_unstable();
            ids.dedup();
            ids
        };

        let found_accounts: Vec<Account> = accounts::table
            .filter(accounts::id.eq_any(&distinct_ids))
            .select(Account::as_select())
            .load(conn)?;

        if found_accounts.len() != distinct_ids.len() {
            return Err(LedgerError::Db(DbErrorKind::NotFound));
        }

        for account in &found_accounts {
            if !account.active {
                return Err(LedgerError::AccountNotActive(account.id));
            }
        }

        // Insert the journal entry record (all columns have DB defaults)
        let entry: JournalEntry = diesel::insert_into(journal_entries::table)
            .default_values()
            .returning(JournalEntry::as_returning())
            .get_result(conn)?;

        // Insert all journal lines
        let new_lines: Vec<NewLedgerLine> = legs
            .iter()
            .map(|leg| NewLedgerLine {
                journal_entry_id: entry.id,
                account: leg.account_id,
                debit: leg.posting.debit(),
                credit: leg.posting.credit(),
            })
            .collect();

        diesel::insert_into(ledger_lines::table)
            .values(&new_lines)
            .execute(conn)?;

        // Compute per-account balance deltas.
        // Debit-nature accounts (Asset, Expense): delta = debit - credit
        // Credit-nature accounts (Liability, Equity, Revenue): delta = credit - debit
        let account_map: HashMap<AccountId, &Account> =
            found_accounts.iter().map(|a| (a.id, a)).collect();

        let mut deltas: HashMap<AccountId, i64> = HashMap::new();
        for leg in &legs {
            let account = account_map[&leg.account_id];
            let delta = match (&leg.posting, account.account_type) {
                (Posting::Debit(v), AccountType::Asset | AccountType::Expense) => v.get() as i64,
                (Posting::Credit(v), AccountType::Asset | AccountType::Expense) => {
                    -(v.get() as i64)
                }
                (Posting::Credit(v), _) => v.get() as i64,
                (Posting::Debit(v), _) => -(v.get() as i64),
            };
            *deltas.entry(leg.account_id).or_insert(0) += delta;
        }

        let now = std::time::SystemTime::now();
        for (account_id, delta) in deltas {
            diesel::update(balances::table.find(account_id))
                .set((
                    balances::balance.eq(balances::balance + delta),
                    balances::updated_at.eq(now),
                ))
                .execute(conn)?;
        }

        Ok(entry)
    })
}

pub fn get_account_balance(
    conn: &mut PgConnection,
    account_id: AccountId,
) -> Result<Balance, LedgerError> {
    use crate::schema::balances;

    balances::table
        .find(account_id)
        .select(Balance::as_select())
        .first(conn)
        .map_err(LedgerError::from)
}

pub fn get_account_lines(
    conn: &mut PgConnection,
    account_id: AccountId,
) -> Result<Vec<LedgerLine>, LedgerError> {
    use crate::schema::ledger_lines;

    ledger_lines::table
        .filter(ledger_lines::account.eq(account_id))
        .select(LedgerLine::as_select())
        .load(conn)
        .map_err(LedgerError::from)
}

pub fn trial_balance(conn: &mut PgConnection) -> Result<TrialBalanceReport, LedgerError> {
    use crate::schema::{accounts, balances};

    let rows: Vec<(AccountType, i64)> = balances::table
        .inner_join(accounts::table)
        .select((accounts::account_type, balances::balance))
        .load(conn)
        .map_err(LedgerError::from)?;

    let mut report = TrialBalanceReport {
        asset: 0,
        expense: 0,
        liability: 0,
        equity: 0,
        revenue: 0,
        is_balanced: false,
    };

    for (account_type, balance) in rows {
        match account_type {
            AccountType::Asset => report.asset += balance,
            AccountType::Expense => report.expense += balance,
            AccountType::Liability => report.liability += balance,
            AccountType::Equity => report.equity += balance,
            AccountType::Revenue => report.revenue += balance,
        }
    }

    // Expanded accounting equation: A + E = L + Eq + R
    report.is_balanced =
        (report.asset + report.expense) == (report.liability + report.equity + report.revenue);

    Ok(report)
}
