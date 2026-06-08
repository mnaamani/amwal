use crate::domain::{
    Account, AccountBlock, AccountId, AccountType, JournalEntry, LedgerLine, NewLedgerLineInput,
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
    /// Activate the account. Balance columns default to zero and live on the
    /// account row — no separate seed step needed.
    fn set_account_active(&self, id: AccountId) -> Result<Account, LedgerError>;
    fn find_account(&self, id: AccountId) -> Result<Option<Account>, LedgerError>;
    fn find_accounts_by_ids(&self, ids: &[AccountId]) -> Result<Vec<Account>, LedgerError>;
    fn list_active_accounts(&self) -> Result<Vec<Account>, LedgerError>;

    // -- Journal --
    /// Atomically insert the journal entry, its lines, and update the
    /// `debits_posted`/`credits_posted` counters on the affected account rows.
    fn persist_journal_entry(
        &self,
        client_id: &str,
        legs: &[NewLedgerLineInput],
    ) -> Result<JournalEntry, LedgerError>;
    fn find_ledger_lines(&self, account_id: AccountId) -> Result<Vec<LedgerLine>, LedgerError>;
    /// Returns one `(account_type, balance)` row per active account — used by
    /// the service layer to compute the trial balance.
    fn aggregate_balances_by_type(&self) -> Result<Vec<(AccountType, i64)>, LedgerError>;

    // -- Account blocks --
    /// Atomically check available balance (`posted_balance - amount_pending`)
    /// and insert a block, incrementing `amount_pending` on the account.
    /// Returns `InsufficientFunds` if the check fails.
    fn apply_account_block(
        &self,
        client_id: &str,
        account_id: AccountId,
        amount: i64,
    ) -> Result<AccountBlock, LedgerError>;
    /// Release the block and decrement `amount_pending` on the account.
    fn release_account_block(&self, client_id: &str) -> Result<AccountBlock, LedgerError>;
}
