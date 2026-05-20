use diesel::prelude::*;
use std::time::SystemTime;

use super::models;
use super::schema::{account_blocks, accounts::dsl as accts, balances};
use crate::domain::{Account, AccountBlock, AccountId, AccountType};
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

pub(super) fn sum_unreleased_blocks(
    conn: &mut PgConnection,
    account_id: AccountId,
) -> Result<i64, LedgerError> {
    use diesel::dsl::sql;
    use diesel::sql_types::BigInt;
    account_blocks::table
        .filter(account_blocks::account_id.eq(account_id))
        .filter(account_blocks::released.eq(false))
        .select(sql::<BigInt>("COALESCE(SUM(amount), 0)"))
        .first::<i64>(conn)
        .map_err(storage_err)
}

pub(super) fn block_funds(
    conn: &mut PgConnection,
    client_id: &str,
    account_id: AccountId,
    amount: i64,
) -> Result<AccountBlock, LedgerError> {
    conn.transaction(|conn| {
        let balance: i64 = balances::table
            .find(account_id)
            .select(balances::balance)
            .first(conn)
            .map_err(storage_err)?;
        let blocked = sum_unreleased_blocks(conn, account_id)?;
        let available = balance - blocked;
        if available < amount {
            return Err(LedgerError::InsufficientFunds {
                available,
                requested: amount,
            });
        }
        diesel::insert_into(account_blocks::table)
            .values(models::NewAccountBlock {
                client_id,
                account_id,
                amount,
            })
            .returning(models::AccountBlock::as_returning())
            .get_result(conn)
            .map(Into::into)
            .map_err(storage_err)
    })
}

/// If the block is already released the existing row is returned
/// unchanged. This makes `cancel_transfer` and `complete_transfer` safe to
/// retry (idempotent)
pub(super) fn release_account_block(
    conn: &mut PgConnection,
    client_id: &str,
) -> Result<AccountBlock, LedgerError> {
    let now = SystemTime::now();
    let result = diesel::update(
        account_blocks::table
            .filter(account_blocks::client_id.eq(client_id))
            .filter(account_blocks::released.eq(false)),
    )
    .set((
        account_blocks::released.eq(true),
        account_blocks::updated_at.eq(now),
    ))
    .returning(models::AccountBlock::as_returning())
    .get_result(conn);

    match result {
        Ok(block) => Ok(block.into()),
        // NotFound means either already released or the client_id doesn't exist.
        // Fetch the row to distinguish: if it exists (released=true) we return
        // it as a no-op success; if it's genuinely missing we propagate the error.
        //
        // Alternative: a single `ON CONFLICT … DO NOTHING` or a conditional
        // UPDATE with RETURNING would collapse this into one round trip.
        Err(diesel::result::Error::NotFound) => account_blocks::table
            .filter(account_blocks::client_id.eq(client_id))
            .select(models::AccountBlock::as_select())
            .first(conn)
            .map(Into::into)
            .map_err(storage_err),
        Err(e) => Err(storage_err(e)),
    }
}

pub(super) fn storage_err(e: diesel::result::Error) -> LedgerError {
    LedgerError::Storage(e.to_string())
}
