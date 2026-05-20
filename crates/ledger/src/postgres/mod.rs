use std::collections::HashMap;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use dotenvy::dotenv;
use std::env;

mod accounts;
mod journal_entries;
mod models;
mod schema;

use crate::domain::{
    Account, AccountBlock, AccountId, AccountType, Balance, JournalEntry, LedgerLine,
    NewLedgerLineInput,
};
use crate::errors::LedgerError;
use crate::store::LedgerStore;

type PgPool = Pool<ConnectionManager<PgConnection>>;
type PgConn = PooledConnection<ConnectionManager<PgConnection>>;

// Required by diesel's `connection.transaction()`, which bounds E: From<diesel::result::Error>.
// Placed here to keep the diesel dependency out of the public errors module.
impl From<diesel::result::Error> for LedgerError {
    fn from(e: diesel::result::Error) -> Self {
        LedgerError::Storage(e.to_string())
    }
}

/// Postgres-backed implementation of [`LedgerStore`].
/// Pure storage — no business rules, no validation.
pub struct PostgresLedgerStore {
    pool: PgPool,
}

impl PostgresLedgerStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn from_env() -> Self {
        dotenv().ok();
        let database_url =
            env::var("LEDGER_DATABASE_URL").expect("LEDGER_DATABASE_URL must be set");
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create ledger connection pool");
        Self { pool }
    }

    fn conn(&self) -> Result<PgConn, LedgerError> {
        self.pool
            .get()
            .map_err(|e| LedgerError::Storage(e.to_string()))
    }
}

impl LedgerStore for PostgresLedgerStore {
    fn insert_account(
        &self,
        client_id: &str,
        name: &str,
        account_type: AccountType,
    ) -> Result<Account, LedgerError> {
        let mut conn = self.conn()?;
        accounts::insert_account(&mut *conn, client_id, name, account_type)
    }

    fn set_account_active(&self, id: AccountId) -> Result<Account, LedgerError> {
        let mut conn = self.conn()?;
        accounts::set_account_active(&mut *conn, id)
    }

    fn find_account(&self, id: AccountId) -> Result<Option<Account>, LedgerError> {
        let mut conn = self.conn()?;
        accounts::find_account(&mut *conn, id)
    }

    fn find_accounts_by_ids(&self, ids: &[AccountId]) -> Result<Vec<Account>, LedgerError> {
        let mut conn = self.conn()?;
        accounts::find_accounts_by_ids(&mut *conn, ids)
    }

    fn list_active_accounts(&self) -> Result<Vec<AccountId>, LedgerError> {
        let mut conn = self.conn()?;
        accounts::list_active_accounts(&mut *conn)
    }

    fn persist_journal_entry(
        &self,
        client_id: &str,
        legs: &[NewLedgerLineInput],
        balance_deltas: HashMap<AccountId, i64>,
    ) -> Result<JournalEntry, LedgerError> {
        let mut conn = self.conn()?;
        journal_entries::persist_journal_entry(&mut *conn, client_id, legs, balance_deltas)
    }

    fn find_balance(&self, account_id: AccountId) -> Result<Balance, LedgerError> {
        let mut conn = self.conn()?;
        journal_entries::find_balance(&mut *conn, account_id)
    }

    fn find_ledger_lines(&self, account_id: AccountId) -> Result<Vec<LedgerLine>, LedgerError> {
        let mut conn = self.conn()?;
        journal_entries::find_ledger_lines(&mut *conn, account_id)
    }

    fn aggregate_balances_by_type(&self) -> Result<Vec<(AccountType, i64)>, LedgerError> {
        let mut conn = self.conn()?;
        journal_entries::aggregate_balances_by_type(&mut *conn)
    }

    fn create_account_block(
        &self,
        client_id: &str,
        account_id: AccountId,
        amount: i64,
    ) -> Result<AccountBlock, LedgerError> {
        let mut conn = self.conn()?;
        journal_entries::create_account_block(&mut *conn, client_id, account_id, amount)
    }

    fn release_account_block(&self, client_id: &str) -> Result<AccountBlock, LedgerError> {
        let mut conn = self.conn()?;
        journal_entries::release_account_block(&mut *conn, client_id)
    }
}
