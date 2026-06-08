use diesel::prelude::*;

use super::models;
use super::schema::{account_blocks, accounts};
use crate::domain::{Account, AccountBlock, AccountId, AccountType};
use crate::errors::LedgerError;

pub(super) fn insert_account(
    conn: &mut PgConnection,
    client_id: &str,
    name: &str,
    account_type: AccountType,
) -> Result<Account, LedgerError> {
    diesel::insert_into(accounts::table)
        .values(&models::NewAccount {
            client_id,
            name,
            account_type: account_type.into(),
        })
        .returning(models::Account::as_returning())
        .get_result(conn)
        .map(Into::into)
        .map_err(LedgerError::from)
}

pub(super) fn set_account_active(
    conn: &mut PgConnection,
    id: AccountId,
) -> Result<Account, LedgerError> {
    diesel::update(accounts::table.find(id))
        .set(accounts::active.eq(true))
        .returning(models::Account::as_returning())
        .get_result(conn)
        .map(Into::into)
        .map_err(LedgerError::from)
}

pub(super) fn find_account(
    conn: &mut PgConnection,
    id: AccountId,
) -> Result<Option<Account>, LedgerError> {
    accounts::table
        .find(id)
        .select(models::Account::as_select())
        .first(conn)
        .optional()
        .map(|opt| opt.map(Into::into))
        .map_err(LedgerError::from)
}

pub(super) fn find_accounts_by_ids(
    conn: &mut PgConnection,
    ids: &[AccountId],
) -> Result<Vec<Account>, LedgerError> {
    accounts::table
        .filter(accounts::id.eq_any(ids))
        .select(models::Account::as_select())
        .load(conn)
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(LedgerError::from)
}

pub(super) fn list_active_accounts(conn: &mut PgConnection) -> Result<Vec<Account>, LedgerError> {
    accounts::table
        .filter(accounts::active.eq(true))
        .select(models::Account::as_select())
        .load(conn)
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(LedgerError::from)
}

/// Atomically check available balance and insert a block.
///
/// Idempotent on `client_id`: if a block with the same `client_id` already
/// exists the existing row is returned without re-checking the balance.
///
/// Uses `SELECT ... FOR UPDATE` on the account row to serialize concurrent
/// block placements so the available-balance check is race-free.
pub(super) fn apply_account_block(
    conn: &mut PgConnection,
    client_id: &str,
    account_id: AccountId,
    amount: i64,
) -> Result<AccountBlock, LedgerError> {
    conn.transaction::<AccountBlock, LedgerError, _>(|conn| {
        let acct: models::Account = accounts::table
            .find(account_id)
            .select(models::Account::as_select())
            .for_update()
            .first(conn)?;

        let available = acct.available_balance();
        if available < amount {
            return Err(LedgerError::InsufficientFunds {
                available,
                requested: amount,
            });
        }

        let insert_result = diesel::insert_into(account_blocks::table)
            .values(models::NewAccountBlock {
                client_id,
                account_id,
                amount,
            })
            .returning(models::AccountBlock::as_returning())
            .get_result(conn);

        match insert_result {
            Ok(block) => {
                diesel::update(accounts::table.find(account_id))
                    .set(accounts::amount_pending.eq(accounts::amount_pending + amount))
                    .execute(conn)?;
                Ok(block.into())
            }
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            )) => account_blocks::table
                .filter(account_blocks::client_id.eq(client_id))
                .select(models::AccountBlock::as_select())
                .first(conn)
                .map(Into::into)
                .map_err(LedgerError::from),
            Err(e) => Err(e.into()),
        }
    })
}

/// If the block is already released the existing row is returned unchanged.
/// Decrements `amount_pending` on the account atomically with the release.
pub(super) fn release_account_block(
    conn: &mut PgConnection,
    client_id: &str,
) -> Result<AccountBlock, LedgerError> {
    conn.transaction::<AccountBlock, LedgerError, _>(|conn| {
        let result = diesel::update(
            account_blocks::table
                .filter(account_blocks::client_id.eq(client_id))
                .filter(account_blocks::released.eq(false)),
        )
        .set(account_blocks::released.eq(true))
        .returning(models::AccountBlock::as_returning())
        .get_result(conn);

        match result {
            Ok(block) => {
                diesel::update(accounts::table.find(block.account_id))
                    .set(accounts::amount_pending.eq(accounts::amount_pending - block.amount))
                    .execute(conn)?;
                Ok(block.into())
            }
            Err(diesel::result::Error::NotFound) => account_blocks::table
                .filter(account_blocks::client_id.eq(client_id))
                .select(models::AccountBlock::as_select())
                .first(conn)
                .map(Into::into)
                .map_err(LedgerError::from),
            Err(e) => Err(e.into()),
        }
    })
}
