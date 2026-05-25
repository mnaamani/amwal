use crate::domain::AccountId;

#[derive(Debug)]
pub enum LedgerError {
    Storage(String),
    ImbalancedEntry {
        total_debits: i64,
        total_credits: i64,
    },
    AccountNotActive(AccountId),
    AccountNotFound(AccountId),
    InvalidJournalEntry(String),
    InvalidLedgerLine(String),
    InvalidInput(String),
    InsufficientFunds {
        available: i64,
        requested: i64,
    },
    AccountsIncompatible,
}
