use diesel::prelude::*;

use crate::errors::LedgerError;
use crate::models::{Account, AccountId, AccountType, NewAccount};

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
    use crate::schema::accounts::dsl::{accounts, active};

    diesel::update(accounts.find(id))
        .set(active.eq(true))
        .returning(Account::as_returning())
        .get_result(conn)
        .map_err(LedgerError::from)
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
