use std::num::NonZeroU64;
use std::str::FromStr;

/// Opaque account identifier — mirrors ledger's internal AccountId.
pub type AccountId = i32;
/// Opaque journal entry identifier returned after a successful posting.
pub type JournalEntryId = i32;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

#[derive(Debug)]
pub struct ParseAccountTypeError;

impl FromStr for AccountType {
    type Err = ParseAccountTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Asset" | "asset" => Ok(AccountType::Asset),
            "Liability" | "liability" => Ok(AccountType::Liability),
            "Equity" | "equity" => Ok(AccountType::Equity),
            "Revenue" | "revenue" => Ok(AccountType::Revenue),
            "Expense" | "expense" => Ok(AccountType::Expense),
            _ => Err(ParseAccountTypeError),
        }
    }
}

#[derive(Debug)]
pub struct AccountSummary {
    pub id: AccountId,
    pub active: bool,
    pub name: String,
    pub account_type: AccountType,
}

/// One side of a double-entry journal posting.
pub enum JournalPosting {
    Debit(NonZeroU64),
    Credit(NonZeroU64),
}

/// A single leg of a journal entry submitted to the ledger.
pub struct JournalLeg {
    pub account_id: AccountId,
    pub posting: JournalPosting,
}

#[derive(Debug)]
pub enum LedgerClientError {
    AccountNotFound(AccountId),
    AccountNotActive(AccountId),
    ImbalancedEntry {
        total_debits: i64,
        total_credits: i64,
    },
    InvalidRequest(String),
    Unavailable(String),
    InsufficientFunds {
        available: i64,
        requested: i64,
    },
    /// The two accounts have different accounting natures (one debit-normal,
    /// one credit-normal) and cannot participate in a direct transfer.
    AccountsIncompatible,
}

/// Returns true if two account types can participate in a direct transfer.
///
/// Transfers are only defined between accounts of the same accounting nature.
/// Moving money between a debit-normal account (Asset, Expense) and a
/// credit-normal account (Liability, Equity, Revenue) would require a
/// contra/intermediate account and is out of scope here.
pub fn accounts_compatible(a: AccountType, b: AccountType) -> bool {
    fn is_debit_normal(t: AccountType) -> bool {
        matches!(t, AccountType::Asset | AccountType::Expense)
    }
    is_debit_normal(a) == is_debit_normal(b)
}

/// The full interface for interacting with the ledger — account management,
/// journal posting, and funds blocking. `&self` throughout so impls can be
/// shared via `Arc`.
pub trait LedgerClient: Send + Sync {
    // -- Account management --
    fn create_account(
        &self,
        client_id: &str,
        name: &str,
        account_type: AccountType,
    ) -> Result<AccountSummary, LedgerClientError>;
    fn activate_account(&self, id: AccountId) -> Result<AccountSummary, LedgerClientError>;
    fn get_account(&self, id: AccountId) -> Result<Option<AccountSummary>, LedgerClientError>;
    fn list_active_accounts(&self) -> Result<Vec<AccountSummary>, LedgerClientError>;

    // -- Journal --
    fn get_account_balance(&self, id: AccountId) -> Result<i64, LedgerClientError>;
    /// Posted balance minus the sum of all unreleased fund blocks on the account.
    fn get_available_balance(&self, id: AccountId) -> Result<i64, LedgerClientError>;
    fn post_journal_entry(
        &self,
        client_id: &str,
        legs: Vec<JournalLeg>,
    ) -> Result<JournalEntryId, LedgerClientError>;

    // -- Funds blocking --
    fn block_funds(
        &self,
        client_id: &str,
        account_id: AccountId,
        amount: i64,
    ) -> Result<(), LedgerClientError>;
    fn release_funds(&self, block_client_id: &str) -> Result<(), LedgerClientError>;

    // -- Transfers --
    /// Post a transfer between two accounts: decrease the sender's balance,
    /// increase the receiver's balance. Posting direction (Debit/Credit) is
    /// determined automatically from account type. Both accounts must be
    /// active and of the same accounting nature.
    fn post_transfer(
        &self,
        client_id: &str,
        from_account_id: AccountId,
        to_account_id: AccountId,
        amount: i64,
    ) -> Result<(), LedgerClientError>;
}
