use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;

pub mod accounts;
pub mod errors;
pub mod journal_entries;
pub mod models;
pub mod transfers;
// Keep schema private, hide the details from the application. They only need to know about the models
mod schema;

pub use accounts::{
    activate_account, create_account, delete_account, get_account, get_active_accounts,
};
pub use journal_entries::{
    TrialBalanceReport, get_account_balance, get_account_lines, post_journal_entry, trial_balance,
};

pub fn db_connect() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
