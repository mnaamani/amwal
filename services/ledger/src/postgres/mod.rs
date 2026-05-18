use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;

mod accounts;
mod journal_entries;
mod models;
mod schema;

use crate::domain::{
    Account, AccountId, AccountType, Balance, JournalEntry, LedgerLine, NewLedgerLineInput,
    TrialBalanceReport,
};
use crate::errors::LedgerError;
use crate::store::LedgerStore;

// Required by diesel's `connection.transaction()`, which bounds E: From<diesel::result::Error>.
// Placed here to keep the diesel dependency out of the public errors module.
impl From<diesel::result::Error> for LedgerError {
    fn from(e: diesel::result::Error) -> Self {
        LedgerError::Storage(e.to_string())
    }
}

pub struct PostgresStore {
    conn: PgConnection,
}

impl PostgresStore {
    pub fn new(conn: PgConnection) -> Self {
        Self { conn }
    }

    pub fn from_env() -> Self {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let conn = PgConnection::establish(&database_url)
            .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));
        Self { conn }
    }
}

impl LedgerStore for PostgresStore {
    fn create_account(
        &mut self,
        name: &str,
        account_type: AccountType,
    ) -> Result<Account, LedgerError> {
        accounts::create_account(&mut self.conn, name, account_type)
    }

    fn activate_account(&mut self, id: AccountId) -> Result<Account, LedgerError> {
        accounts::activate_account(&mut self.conn, id)
    }

    fn delete_account(&mut self, pattern: &str) -> Result<usize, LedgerError> {
        accounts::delete_account(&mut self.conn, pattern)
    }

    fn get_account(&mut self, id: AccountId) -> Result<Option<Account>, LedgerError> {
        accounts::get_account(&mut self.conn, id)
    }

    fn get_active_accounts(&mut self) -> Result<Vec<Account>, LedgerError> {
        accounts::get_active_accounts(&mut self.conn)
    }

    fn post_journal_entry(
        &mut self,
        legs: Vec<NewLedgerLineInput>,
    ) -> Result<JournalEntry, LedgerError> {
        journal_entries::post_journal_entry(&mut self.conn, legs)
    }

    fn get_account_balance(&mut self, account_id: AccountId) -> Result<Balance, LedgerError> {
        journal_entries::get_account_balance(&mut self.conn, account_id)
    }

    fn get_account_lines(&mut self, account_id: AccountId) -> Result<Vec<LedgerLine>, LedgerError> {
        journal_entries::get_account_lines(&mut self.conn, account_id)
    }

    fn trial_balance(&mut self) -> Result<TrialBalanceReport, LedgerError> {
        journal_entries::trial_balance(&mut self.conn)
    }
}
