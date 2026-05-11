use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;

use crate::models::NewAccount;

pub mod models;
pub mod schema;

pub fn db_connect() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn create_account(conn: &mut PgConnection, name: &str) -> models::Account {
    use crate::schema::ledger_accounts;
    let new_account = NewAccount { name };

    diesel::insert_into(ledger_accounts::table)
        .values(&new_account)
        .returning(models::Account::as_returning())
        .get_result(conn)
        .expect("Error Creating Account")
}
