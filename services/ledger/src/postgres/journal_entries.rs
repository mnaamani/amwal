use std::collections::HashMap;

use diesel::prelude::*;

use super::models;
use super::schema::{accounts, balances, journal_entries, ledger_lines};
use crate::domain::{
    AccountId, AccountType, Balance, JournalEntry, LedgerLine, NewLedgerLineInput, Posting,
    TrialBalanceReport,
};
use crate::errors::LedgerError;

pub(super) fn post_journal_entry(
    conn: &mut PgConnection,
    legs: Vec<NewLedgerLineInput>,
) -> Result<JournalEntry, LedgerError> {
    if legs.len() < 2 {
        return Err(LedgerError::InvalidJournalEntry(
            "a journal entry requires at least two legs".into(),
        ));
    }

    let total_debits: i64 = legs.iter().map(|l| l.posting.debit()).sum();
    let total_credits: i64 = legs.iter().map(|l| l.posting.credit()).sum();
    if total_debits != total_credits {
        return Err(LedgerError::ImbalancedEntry {
            total_debits,
            total_credits,
        });
    }

    conn.transaction::<JournalEntry, LedgerError, _>(|conn| {
        let distinct_ids: Vec<AccountId> = {
            let mut ids: Vec<AccountId> = legs.iter().map(|l| l.account_id).collect();
            ids.sort_unstable();
            ids.dedup();
            ids
        };

        let found_accounts: Vec<models::Account> = accounts::table
            .filter(accounts::id.eq_any(&distinct_ids))
            .select(models::Account::as_select())
            .load(conn)
            .map_err(storage_err)?;

        if found_accounts.len() != distinct_ids.len() {
            return Err(LedgerError::Storage(
                "one or more accounts not found".into(),
            ));
        }

        for account in &found_accounts {
            if !account.active {
                return Err(LedgerError::AccountNotActive(account.id));
            }
        }

        let entry: models::JournalEntry = diesel::insert_into(journal_entries::table)
            .default_values()
            .returning(models::JournalEntry::as_returning())
            .get_result(conn)
            .map_err(storage_err)?;

        let new_lines: Vec<models::NewLedgerLine> = legs
            .iter()
            .map(|leg| models::NewLedgerLine {
                journal_entry_id: entry.id,
                account: leg.account_id,
                debit: leg.posting.debit(),
                credit: leg.posting.credit(),
            })
            .collect();

        diesel::insert_into(ledger_lines::table)
            .values(&new_lines)
            .execute(conn)
            .map_err(storage_err)?;

        // Compute per-account balance deltas.
        // Debit-nature (Asset, Expense): delta = debit - credit
        // Credit-nature (Liability, Equity, Revenue): delta = credit - debit
        let account_type_map: HashMap<AccountId, AccountType> = found_accounts
            .iter()
            .map(|a| (a.id, a.account_type.into()))
            .collect();

        let mut deltas: HashMap<AccountId, i64> = HashMap::new();
        for leg in &legs {
            let account_type = account_type_map[&leg.account_id];
            let delta = match (&leg.posting, account_type) {
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
                .execute(conn)
                .map_err(storage_err)?;
        }

        Ok(entry.into())
    })
}

pub(super) fn get_account_balance(
    conn: &mut PgConnection,
    account_id: AccountId,
) -> Result<Balance, LedgerError> {
    balances::table
        .find(account_id)
        .select(models::Balance::as_select())
        .first(conn)
        .map(Into::into)
        .map_err(storage_err)
}

pub(super) fn get_account_lines(
    conn: &mut PgConnection,
    account_id: AccountId,
) -> Result<Vec<LedgerLine>, LedgerError> {
    ledger_lines::table
        .filter(ledger_lines::account.eq(account_id))
        .select(models::LedgerLine::as_select())
        .load(conn)
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(storage_err)
}

pub(super) fn trial_balance(conn: &mut PgConnection) -> Result<TrialBalanceReport, LedgerError> {
    let rows: Vec<(models::AccountType, i64)> = balances::table
        .inner_join(accounts::table)
        .select((accounts::account_type, balances::balance))
        .load(conn)
        .map_err(storage_err)?;

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
            models::AccountType::Asset => report.asset += balance,
            models::AccountType::Expense => report.expense += balance,
            models::AccountType::Liability => report.liability += balance,
            models::AccountType::Equity => report.equity += balance,
            models::AccountType::Revenue => report.revenue += balance,
        }
    }

    report.is_balanced =
        (report.asset + report.expense) == (report.liability + report.equity + report.revenue);

    Ok(report)
}

fn storage_err(e: diesel::result::Error) -> LedgerError {
    LedgerError::Storage(e.to_string())
}
