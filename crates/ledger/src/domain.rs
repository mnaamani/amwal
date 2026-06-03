use std::{num::NonZeroU64, time::SystemTime};

/// Internal account identifier. Opaque to callers — do not interpret the value.
pub type AccountId = i32;
/// Internal journal entry identifier returned after a successful posting.
pub type JournalEntryId = i32;
/// Internal ledger line identifier (one line per leg of a journal entry).
pub type LedgerLineId = i32;
/// Internal fund block identifier.
pub type AccountBlockId = i32;

pub use ledger_api::AccountType;

/// A ledger account record as stored internally.
///
/// Newly created accounts are inactive (`active: false`) and cannot receive
/// journal postings until explicitly activated. Activation also seeds the
/// balance row.
#[derive(Debug)]
pub struct Account {
    pub id: AccountId,
    /// Caller-supplied idempotency key used when creating the account.
    pub client_id: String,
    pub account_type: AccountType,
    pub active: bool,
    pub name: String,
    pub created_at: SystemTime,
}

/// A committed journal entry header. The actual debits and credits live in
/// the associated [`LedgerLine`] rows.
#[derive(Debug)]
pub struct JournalEntry {
    pub id: JournalEntryId,
    /// Caller-supplied idempotency key — duplicate submissions with the same
    /// `client_id` return this entry rather than posting again.
    pub client_id: String,
    pub created_at: SystemTime,
    pub updated_at: Option<SystemTime>,
}

/// A single debit or credit posting to an account within a journal entry.
///
/// Amounts are non-zero by construction — use [`NonZeroU64`] at the call site.
/// A well-formed entry always has at least one `Debit` and one `Credit` leg,
/// and their totals must be equal (double-entry invariant).
pub enum Posting {
    Debit(NonZeroU64),
    Credit(NonZeroU64),
}

impl Posting {
    /// Returns the debit amount in fils, or 0 if this is a Credit posting.
    pub fn debit(&self) -> i64 {
        match self {
            Self::Debit(v) => v.get() as i64,
            Self::Credit(_) => 0,
        }
    }

    /// Returns the credit amount in fils, or 0 if this is a Debit posting.
    pub fn credit(&self) -> i64 {
        match self {
            Self::Credit(v) => v.get() as i64,
            Self::Debit(_) => 0,
        }
    }

    /// The absolute amount regardless of direction.
    pub fn amount(&self) -> NonZeroU64 {
        match self {
            Self::Debit(v) | Self::Credit(v) => *v,
        }
    }
}

/// One committed leg of a journal entry as stored in the ledger.
///
/// Exactly one of `debit` or `credit` is non-zero for any given row
/// (split columns rather than a signed amount, for clarity in SQL).
#[derive(Debug)]
pub struct LedgerLine {
    pub id: LedgerLineId,
    pub journal_entry_id: JournalEntryId,
    pub account: AccountId,
    /// Non-zero when this leg is a Debit; zero otherwise.
    pub debit: i64,
    /// Non-zero when this leg is a Credit; zero otherwise.
    pub credit: i64,
    pub created_at: SystemTime,
}

impl LedgerLine {
    /// Reconstruct the typed [`Posting`] from the split debit/credit columns.
    pub fn posting(&self) -> Posting {
        if self.debit > 0 {
            Posting::Debit(NonZeroU64::new(self.debit as u64).expect("debit checked > 0"))
        } else {
            Posting::Credit(NonZeroU64::new(self.credit as u64).expect("credit checked > 0"))
        }
    }
}

/// The current posted balance for an account, maintained as a running total
/// and updated atomically with each journal entry.
#[derive(Debug)]
pub struct Balance {
    pub account_id: AccountId,
    /// Posted balance in fils. Does not subtract unreleased fund blocks.
    pub balance: i64,
    pub updated_at: SystemTime,
}

/// Input for one line of a journal entry.
pub struct NewLedgerLineInput {
    pub account_id: AccountId,
    pub posting: Posting,
}

/// A temporary reservation of funds that prevents them from being spent while
/// a transfer is in flight. Identified by `client_id` for idempotent placement
/// and release. Released automatically when the transfer completes or cancels.
#[derive(Debug)]
pub struct AccountBlock {
    pub id: AccountBlockId,
    pub client_id: String,
    pub account_id: AccountId,
    /// Amount reserved, in fils.
    pub amount: i64,
    /// True once the block has been released (either by completion or cancellation).
    pub released: bool,
    pub created_at: SystemTime,
}

/// Aggregated balances by account type — used to verify the fundamental
/// accounting equation: Assets + Expenses = Liabilities + Equity + Revenue.
pub struct TrialBalanceReport {
    pub asset: i64,
    pub expense: i64,
    pub liability: i64,
    pub equity: i64,
    pub revenue: i64,
    /// True when `asset + expense == liability + equity + revenue`.
    pub is_balanced: bool,
}
