use diesel::prelude::*;
use std::time::SystemTime;

use super::models;
use super::schema::{accounts::dsl as accts, balances};
use crate::domain::{Account, AccountId, AccountType};
use crate::errors::LedgerError;

pub(super) fn insert_account(
    conn: &mut PgConnection,
    client_id: &str,
    name: &str,
    account_type: AccountType,
) -> Result<Account, LedgerError> {
    let new_account = models::NewAccount {
        client_id,
        name,
        account_type: account_type.into(),
    };
    diesel::insert_into(accts::accounts)
        .values(&new_account)
        .returning(models::Account::as_returning())
        .get_result(conn)
        .map(Into::into)
        .map_err(storage_err)
}

pub(super) fn set_account_active(
    conn: &mut PgConnection,
    id: AccountId,
) -> Result<Account, LedgerError> {
    conn.transaction::<Account, LedgerError, _>(|conn| {
        let account: models::Account = diesel::update(accts::accounts.find(id))
            .set((
                accts::active.eq(true),
                accts::updated_at.eq(SystemTime::now()),
            ))
            .returning(models::Account::as_returning())
            .get_result(conn)
            .map_err(storage_err)?;

        diesel::insert_into(balances::table)
            .values(models::NewBalance {
                account_id: id,
                balance: 0,
            })
            .on_conflict_do_nothing()
            .execute(conn)
            .map_err(storage_err)?;

        Ok(account.into())
    })
}

pub(super) fn find_account(
    conn: &mut PgConnection,
    id: AccountId,
) -> Result<Option<Account>, LedgerError> {
    accts::accounts
        .find(id)
        .select(models::Account::as_select())
        .first(conn)
        .optional()
        .map(|opt| opt.map(Into::into))
        .map_err(storage_err)
}

pub(super) fn find_accounts_by_ids(
    conn: &mut PgConnection,
    ids: &[AccountId],
) -> Result<Vec<Account>, LedgerError> {
    accts::accounts
        .filter(accts::id.eq_any(ids))
        .select(models::Account::as_select())
        .load(conn)
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(storage_err)
}

pub(super) fn list_active_accounts(conn: &mut PgConnection) -> Result<Vec<AccountId>, LedgerError> {
    accts::accounts
        .filter(accts::active.eq(true))
        .limit(5)
        .select(models::Account::as_select())
        .load(conn)
        .map(|v| v.into_iter().map(|v| v.id).collect())
        .map_err(storage_err)
}

pub(super) fn storage_err(e: diesel::result::Error) -> LedgerError {
    LedgerError::Storage(e.to_string())
}
