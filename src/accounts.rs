use std::time::SystemTime;

use diesel::prelude::*;

use crate::errors::LedgerError;
use crate::models::{Account, AccountId, AccountType, NewAccount, NewBalance};

pub fn create_account(
    conn: &mut PgConnection,
    name: &str,
    account_type: AccountType,
) -> Result<Account, LedgerError> {
    use crate::schema::accounts;
    let new_account = NewAccount { name, account_type };

    diesel::insert_into(accounts::table)
        .values(&new_account)
        .returning(Account::as_returning())
        .get_result(conn)
        .map_err(LedgerError::from)
}

pub fn activate_account(conn: &mut PgConnection, id: AccountId) -> Result<Account, LedgerError> {
    use crate::schema::accounts::dsl::{accounts, active, updated_at};
    use crate::schema::balances;

    conn.transaction::<Account, LedgerError, _>(|conn| {
        let account = diesel::update(accounts.find(id))
            .set((active.eq(true), updated_at.eq(SystemTime::now())))
            .returning(Account::as_returning())
            .get_result(conn)
            .map_err(LedgerError::from)?;

        // Seed a zero balance row; on_conflict_do_nothing guards against double-activation
        diesel::insert_into(balances::table)
            .values(NewBalance {
                account_id: id,
                balance: 0,
            })
            .on_conflict_do_nothing()
            .execute(conn)
            .map_err(LedgerError::from)?;

        Ok(account)
    })
}

pub fn delete_account(conn: &mut PgConnection, pattern: &str) -> Result<usize, LedgerError> {
    use crate::schema::accounts::dsl::{accounts, name};

    diesel::delete(accounts.filter(name.like(pattern)))
        .execute(conn)
        .map_err(LedgerError::from)
}

pub fn get_account(conn: &mut PgConnection, id: AccountId) -> Result<Option<Account>, LedgerError> {
    use crate::schema::accounts::dsl::accounts;

    accounts
        .find(id)
        .select(Account::as_select())
        .first(conn)
        .optional()
        .map_err(LedgerError::from)
}

pub fn get_active_accounts(conn: &mut PgConnection) -> Result<Vec<Account>, LedgerError> {
    use crate::schema::accounts::dsl::{accounts, active};

    accounts
        .filter(active.eq(true))
        .limit(5)
        .select(Account::as_select())
        .load(conn)
        .map_err(LedgerError::from)
}
