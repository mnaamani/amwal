use std::collections::HashMap;

use diesel::prelude::*;

use super::accounts::storage_err;
use super::models;
use super::schema::{accounts, balances, journal_entries, ledger_lines};
use crate::domain::{
    AccountId, AccountType, Balance, JournalEntry, LedgerLine, NewLedgerLineInput,
};
use crate::errors::LedgerError;

/// Atomically insert a journal entry, its ledger lines, and apply the
/// pre-computed balance deltas. All validation and delta computation happens
/// in the service layer before this is called.
pub(super) fn persist_journal_entry(
    conn: &mut PgConnection,
    client_id: &str,
    legs: &[NewLedgerLineInput],
    balance_deltas: HashMap<AccountId, i64>,
) -> Result<JournalEntry, LedgerError> {
    conn.transaction::<JournalEntry, LedgerError, _>(|conn| {
        let entry: models::JournalEntry = diesel::insert_into(journal_entries::table)
            .values(models::NewJournalEntry { client_id })
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

        let now = std::time::SystemTime::now();
        for (account_id, delta) in &balance_deltas {
            diesel::update(balances::table.find(*account_id))
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

pub(super) fn find_balance(
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

pub(super) fn find_ledger_lines(
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

pub(super) fn aggregate_balances_by_type(
    conn: &mut PgConnection,
) -> Result<Vec<(AccountType, i64)>, LedgerError> {
    let rows: Vec<(models::AccountType, i64)> = balances::table
        .inner_join(accounts::table)
        .select((accounts::account_type, balances::balance))
        .load(conn)
        .map_err(storage_err)?;

    Ok(rows.into_iter().map(|(t, b)| (t.into(), b)).collect())
}
