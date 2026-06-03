use crate::domain::AccountId;

/// Errors returned by [`LedgerService`] operations.
///
/// [`Storage`](LedgerError::Storage) is transient and safe to retry.
/// All other variants indicate a caller error and will not resolve
/// on retry without a change to the request.
///
/// [`LedgerService`]: crate::service::LedgerService
#[derive(Debug)]
pub enum LedgerError {
    /// A storage or database error — transient, safe to retry.
    Storage(String),
    /// The journal entry was rejected because total debits ≠ total credits.
    ImbalancedEntry {
        total_debits: i64,
        total_credits: i64,
    },
    /// The account exists but has not been activated and cannot receive postings.
    AccountNotActive(AccountId),
    /// No account exists with the given [`AccountId`].
    AccountNotFound(AccountId),
    /// The journal entry failed a structural rule (e.g. fewer than two legs).
    InvalidJournalEntry(String),
    /// An individual ledger line failed validation (e.g. zero amount).
    InvalidLedgerLine(String),
    /// General input validation failure not covered by a more specific variant.
    InvalidInput(String),
    /// The account's available balance (posted minus blocked) is too low.
    InsufficientFunds { available: i64, requested: i64 },
    /// The two accounts have incompatible accounting natures (one debit-normal,
    /// one credit-normal) and cannot participate in a direct transfer.
    AccountsIncompatible,
}
