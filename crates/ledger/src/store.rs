use std::collections::HashMap;

use crate::domain::{
    Account, AccountBlock, AccountId, AccountType, Balance, JournalEntry, LedgerLine,
    NewLedgerLineInput,
};
use crate::errors::LedgerError;

/// Raw storage interface for the ledger — one method per atomic DB operation,
/// no business rules. Implementations are free to use any backend (Postgres,
/// in-memory, etc.).  `Send + Sync` so implementors can be shared via `Arc`.
pub trait LedgerStore: Send + Sync {
    // -- Accounts --
    fn insert_account(
        &self,
        client_id: &str,
        name: &str,
        account_type: AccountType,
    ) -> Result<Account, LedgerError>;
    /// Activate the account and seed a zero-balance row (atomic).
    fn set_account_active(&self, id: AccountId) -> Result<Account, LedgerError>;
    fn find_account(&self, id: AccountId) -> Result<Option<Account>, LedgerError>;
    fn find_accounts_by_ids(&self, ids: &[AccountId]) -> Result<Vec<Account>, LedgerError>;
    fn list_active_accounts(&self) -> Result<Vec<AccountId>, LedgerError>;

    // -- Journal --
    /// Atomically insert the journal entry, its lines, and apply pre-computed
    /// balance deltas. The caller is responsible for computing valid deltas.
    fn persist_journal_entry(
        &self,
        client_id: &str,
        legs: &[NewLedgerLineInput],
        balance_deltas: HashMap<AccountId, i64>,
    ) -> Result<JournalEntry, LedgerError>;
    fn find_balance(&self, account_id: AccountId) -> Result<Balance, LedgerError>;
    fn find_ledger_lines(&self, account_id: AccountId) -> Result<Vec<LedgerLine>, LedgerError>;
    /// Returns one `(account_type, balance)` row per account — used by the
    /// service layer to compute the trial balance.
    fn aggregate_balances_by_type(&self) -> Result<Vec<(AccountType, i64)>, LedgerError>;

    // -- Account blocks --
    /// Sum of all unreleased block amounts for the account.
    fn sum_unreleased_blocks(&self, account_id: AccountId) -> Result<i64, LedgerError>;
    /// Atomically check available balance and insert a block. Returns
    /// `InsufficientFunds` if `posted_balance - unreleased_blocks < amount`.
    fn apply_account_block(
        &self,
        client_id: &str,
        account_id: AccountId,
        amount: i64,
    ) -> Result<AccountBlock, LedgerError>;
    fn release_account_block(&self, client_id: &str) -> Result<AccountBlock, LedgerError>;
}
