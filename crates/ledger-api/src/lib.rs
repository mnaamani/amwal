use std::num::NonZeroU64;
use std::str::FromStr;
use std::time::SystemTime;

/// Opaque account identifier.
pub type AccountId = i64;
/// Opaque journal entry identifier returned after a successful posting.
pub type JournalEntryId = i64;
/// Monetary amount expressed in the currency's minor unit (cents, pence, halala, …).
pub type Amount = i64;

/// The accounting classification of an account, which determines how debits
/// and credits affect its balance.
///
/// Accounts fall into two groups based on their *normal balance* — the side
/// (debit or credit) that increases the account:
///
/// - **Debit-normal** (Asset, Expense): a Debit increases the balance;
///   a Credit decreases it.
/// - **Credit-normal** (Liability, Equity, Revenue): a Credit increases the
///   balance; a Debit decreases it.
///
/// This distinction drives balance-delta computation in the service layer and
/// determines which posting directions are valid for a direct transfer between
/// two accounts (see [`accounts_compatible`]).
#[derive(Debug, PartialEq, Eq, Copy, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum AccountType {
    /// Debit-normal. Represents something the institution owns or is owed.
    Asset,
    /// Credit-normal. Represents amounts owed to depositors or creditors.
    Liability,
    /// Credit-normal. Represents ownership interest or retained earnings.
    Equity,
    /// Credit-normal. Represents income earned.
    Revenue,
    /// Debit-normal. Represents costs incurred.
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

/// A read-only summary of an account returned by query operations.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AccountSummary {
    pub id: AccountId,
    /// Whether the account has been activated and can receive journal postings.
    pub active: bool,
    pub name: String,
    pub account_type: AccountType,
}

/// One side of a double-entry journal posting submitted via [`LedgerClient`].
///
/// This is the API-layer equivalent of the domain's `Posting` type.
/// Amounts are in the currency's minor unit and must be non-zero — use
/// [`NonZeroU64`] to enforce this at construction time.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "direction", content = "amount")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum JournalPosting {
    Debit(#[cfg_attr(feature = "openapi", schema(value_type = u64))] NonZeroU64),
    Credit(#[cfg_attr(feature = "openapi", schema(value_type = u64))] NonZeroU64),
}

/// The account balance after a specific journal entry was posted.
/// Used to reconstruct balance history for zakat or audit purposes.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct BalanceSnapshot {
    #[serde(with = "system_time_serde")]
    #[cfg_attr(feature = "openapi", schema(value_type = u64, example = 1700000000000u64))]
    pub timestamp: SystemTime,
    /// Running balance in the currency's minor unit.
    pub balance: Amount,
}

/// A single leg of a journal entry submitted to the ledger.
///
/// A well-formed entry requires at least two legs whose debits and credits
/// balance. The `account_id` must refer to an active account.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct JournalLeg {
    pub account_id: AccountId,
    pub posting: JournalPosting,
}

/// Errors returned by [`LedgerClient`] operations.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "error", content = "detail")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum LedgerClientError {
    /// No account exists with the given [`AccountId`].
    AccountNotFound(AccountId),
    /// The account exists but has not been activated yet.
    AccountNotActive(AccountId),
    /// The journal entry was rejected because total debits ≠ total credits.
    ImbalancedEntry {
        total_debits: Amount,
        total_credits: Amount,
    },
    /// A structural or business-rule validation failure (malformed input).
    InvalidRequest(String),
    /// A storage or infrastructure error — safe to retry.
    Unavailable(String),
    /// The account's available balance (posted minus blocked) is too low.
    InsufficientFunds {
        available: Amount,
        requested: Amount,
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
///
/// ## Idempotency
///
/// Every write operation accepts a `client_id: &str` that acts as an
/// idempotency key. Submitting the same `client_id` a second time returns
/// the result of the first successful call rather than performing a duplicate
/// write. Callers should generate a stable, unique key per logical operation
/// (e.g. a UUID) and retry freely on `Unavailable` errors.
pub trait LedgerClient: Send + Sync {
    // -- Account management --

    /// Create a new inactive account. The account cannot receive journal
    /// postings until [`LedgerClient::activate_account`] is called.
    fn create_account(
        &self,
        client_id: &str,
        name: &str,
        account_type: AccountType,
    ) -> Result<AccountSummary, LedgerClientError>;

    /// Activate an account so it can receive journal postings.
    fn activate_account(&self, id: AccountId) -> Result<AccountSummary, LedgerClientError>;

    /// Fetch a single account by ID. Returns `None` if no such account exists.
    fn get_account(&self, id: AccountId) -> Result<Option<AccountSummary>, LedgerClientError>;

    /// List all accounts that have been activated.
    fn list_active_accounts(&self) -> Result<Vec<AccountSummary>, LedgerClientError>;

    // -- Journal --

    /// The posted balance: the sum of all committed journal entries.
    /// Does **not** subtract unreleased fund blocks — use
    /// [`LedgerClient::get_available_balance`] for that.
    fn get_account_balance(&self, id: AccountId) -> Result<Amount, LedgerClientError>;

    /// Posted balance minus the sum of all unreleased fund blocks on the account.
    /// This is the amount the account holder can actually spend right now.
    fn get_available_balance(&self, id: AccountId) -> Result<Amount, LedgerClientError>;

    /// Post a balanced double-entry journal entry. The sum of all Debit legs
    /// must equal the sum of all Credit legs, and every account must be active.
    ///
    /// Idempotent on `client_id`: a duplicate submission returns the original
    /// entry rather than posting again.
    fn post_journal_entry(
        &self,
        client_id: &str,
        legs: Vec<JournalLeg>,
    ) -> Result<JournalEntryId, LedgerClientError>;

    // -- Funds blocking --

    /// Reserve `amount` minor units on `account_id` so they cannot be spent
    /// while a transfer is in flight. The block is identified by `client_id`
    /// and must be released (via [`LedgerClient::release_funds`]) or the
    /// transfer completed before the funds are freed.
    ///
    /// Returns `InsufficientFunds` if `available_balance < amount`.
    /// Idempotent: a duplicate `client_id` returns the existing block.
    fn block_funds(
        &self,
        client_id: &str,
        account_id: AccountId,
        amount: Amount,
    ) -> Result<(), LedgerClientError>;

    /// Release the fund block identified by `block_client_id`, making those
    /// minor units available again. No-op if the block was already released.
    fn release_funds(&self, block_client_id: &str) -> Result<(), LedgerClientError>;

    // -- History --

    /// Returns the running balance after each journal posting, in chronological
    /// order. Used by consumers that need to reason about balance over time
    /// without coupling to ledger internals.
    fn get_balance_history(
        &self,
        account_id: AccountId,
    ) -> Result<Vec<BalanceSnapshot>, LedgerClientError>;

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
        amount: Amount,
    ) -> Result<(), LedgerClientError>;
}

// ── SystemTime serde helper ───────────────────────────────────────────────────

/// Serialises `SystemTime` as milliseconds since the Unix epoch (u64).
/// Avoids any external dependency while producing a compact, JSON-safe value.
mod system_time_serde {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(t: &SystemTime, s: S) -> Result<S::Ok, S::Error> {
        let ms = t
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis() as u64;
        s.serialize_u64(ms)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<SystemTime, D::Error> {
        let ms = <u64 as serde::Deserialize>::deserialize(d)?;
        Ok(UNIX_EPOCH + Duration::from_millis(ms))
    }
}
