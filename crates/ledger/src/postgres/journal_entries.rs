use std::collections::HashSet;

use diesel::prelude::*;

use super::models;
use super::schema::{accounts, journal_entries, ledger_lines, outbox};
use crate::domain::{
    AccountId, AccountType, JournalEntry, LedgerLine, NewLedgerLineInput, Posting,
};
use crate::errors::LedgerError;

/// Atomically insert a journal entry, its ledger lines, and update the
/// per-account debit/credit posted counters on the accounts table.
///
/// Idempotent on `client_id`: if a journal entry with the same `client_id`
/// already exists the existing row is returned and no further writes are made.
pub(super) fn persist_journal_entry(
    conn: &mut PgConnection,
    client_id: &str,
    legs: &[NewLedgerLineInput],
) -> Result<JournalEntry, LedgerError> {
    conn.transaction::<JournalEntry, LedgerError, _>(|conn| {
        let insert_result = diesel::insert_into(journal_entries::table)
            .values(models::NewJournalEntry { client_id })
            .returning(models::JournalEntry::as_returning())
            .get_result(conn);

        let entry: models::JournalEntry = match insert_result {
            Ok(e) => e,
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            )) => {
                return journal_entries::table
                    .filter(journal_entries::client_id.eq(client_id))
                    .select(models::JournalEntry::as_select())
                    .first(conn)
                    .map(Into::into)
                    .map_err(LedgerError::from);
            }
            Err(e) => return Err(e.into()),
        };

        let new_lines: Vec<models::NewLedgerLine> = legs
            .iter()
            .map(|leg| models::NewLedgerLine {
                journal_entry_id: entry.id,
                account: leg.account_id,
                amount: leg.posting.amount().get() as i64,
                direction: leg.posting.direction().into(),
            })
            .collect();

        diesel::insert_into(ledger_lines::table)
            .values(&new_lines)
            .execute(conn)?;

        for leg in legs {
            match &leg.posting {
                Posting::Debit(v) => {
                    diesel::update(accounts::table.find(leg.account_id))
                        .set(accounts::debits_posted.eq(accounts::debits_posted + v.get() as i64))
                        .execute(conn)?;
                }
                Posting::Credit(v) => {
                    diesel::update(accounts::table.find(leg.account_id))
                        .set(accounts::credits_posted.eq(accounts::credits_posted + v.get() as i64))
                        .execute(conn)?;
                }
            }
        }

        let distinct_ids: Vec<AccountId> = legs
            .iter()
            .map(|l| l.account_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let affected: Vec<models::Account> = accounts::table
            .filter(accounts::id.eq_any(&distinct_ids))
            .select(models::Account::as_select())
            .load(conn)?;

        for acct in &affected {
            let new_balance = acct.posted_balance();
            let event = domain_events::DomainEvent::BalanceChanged {
                account_id: acct.id,
                new_balance,
                journal_entry_id: entry.id,
            };
            let payload =
                serde_json::to_value(&event).map_err(|e| LedgerError::Storage(e.to_string()))?;

            diesel::insert_into(outbox::table)
                .values(models::NewOutboxEvent {
                    event_type: "BalanceChanged",
                    payload,
                })
                .execute(conn)?;
        }

        Ok(entry.into())
    })
}

pub(super) fn find_ledger_lines(
    conn: &mut PgConnection,
    account_id: AccountId,
) -> Result<Vec<LedgerLine>, LedgerError> {
    ledger_lines::table
        .filter(ledger_lines::account.eq(account_id))
        .order(ledger_lines::id.asc())
        .select(models::LedgerLine::as_select())
        .load(conn)
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(LedgerError::from)
}

pub(super) fn aggregate_balances_by_type(
    conn: &mut PgConnection,
) -> Result<Vec<(AccountType, i64)>, LedgerError> {
    let rows: Vec<(models::AccountType, i64, i64)> = accounts::table
        .filter(accounts::active.eq(true))
        .select((
            accounts::account_type,
            accounts::debits_posted,
            accounts::credits_posted,
        ))
        .load(conn)
        .map_err(LedgerError::from)?;

    Ok(rows
        .into_iter()
        .map(|(at, debits, credits)| {
            let domain_at: AccountType = at.into();
            let balance = match domain_at {
                AccountType::Asset | AccountType::Expense => debits - credits,
                _ => credits - debits,
            };
            (domain_at, balance)
        })
        .collect())
}
