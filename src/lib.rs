use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;

use crate::models::{Account, AccountType, NewAccount};

pub mod models;
// Keep schema private, hide the details from the application. They only need to know about the models
mod schema;

pub fn db_connect() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn create_account(
    conn: &mut PgConnection,
    name: &str,
    account_type: AccountType,
) -> Result<Account, diesel::result::Error> {
    use crate::schema::accounts;
    let new_account = NewAccount { name, account_type };

    diesel::insert_into(accounts::table)
        .values(&new_account)
        .returning(Account::as_returning())
        .get_result(conn)
}

pub fn activate_account(
    conn: &mut PgConnection,
    id: i32,
) -> Result<Account, diesel::result::Error> {
    use self::schema::accounts::dsl::{accounts, active};

    diesel::update(accounts.find(id))
        .set(active.eq(true))
        .returning(Account::as_returning())
        .get_result(conn)
}

pub fn delete_account(
    conn: &mut PgConnection,
    pattern: &str,
) -> Result<usize, diesel::result::Error> {
    use self::schema::accounts::dsl::{accounts, name};

    diesel::delete(accounts.filter(name.like(pattern))).execute(conn)
}

pub fn get_account(
    conn: &mut PgConnection,
    id: i32,
) -> Result<Option<Account>, diesel::result::Error> {
    use self::schema::accounts::dsl::accounts;
    accounts
        .find(id)
        .select(Account::as_select())
        .first(conn)
        .optional()
}

pub fn get_active_accounts(conn: &mut PgConnection) -> Result<Vec<Account>, diesel::result::Error> {
    use self::schema::accounts::dsl::{accounts, active};
    accounts
        .filter(active.eq(true))
        .limit(5)
        .select(Account::as_select())
        .load(conn)
}
