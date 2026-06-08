use std::{num::NonZeroU64, time::SystemTime};

pub use ledger_api::{AccountId, AccountType, Amount, JournalEntryId};

/// Internal ledger line identifier (one line per leg of a journal entry).
pub type LedgerLineId = i64;
/// Internal fund block identifier.
pub type AccountBlockId = i64;

/// A ledger account record as stored internally.
///
/// Newly created accounts are inactive (`active: false`) and cannot receive
/// journal postings until explicitly activated. Balance columns are maintained
/// atomically alongside every journal posting — no separate balances table.
#[derive(Debug)]
pub struct Account {
    pub id: AccountId,
    /// Caller-supplied idempotency key used when creating the account.
    pub client_id: String,
    pub account_type: AccountType,
    pub active: bool,
    pub name: String,
    /// Cumulative sum of all debit postings to this account.
    pub debits_posted: i64,
    /// Cumulative sum of all credit postings to this account.
    pub credits_posted: i64,
    /// Total amount currently held in unreleased fund blocks.
    pub amount_pending: i64,
    pub created_at: SystemTime,
}

impl Account {
    /// Signed balance in the account's normal direction.
    /// Positive means the account is in its expected state (e.g. an asset with
    /// value, a liability owed to depositors).
    pub fn posted_balance(&self) -> i64 {
        match self.account_type {
            AccountType::Asset | AccountType::Expense => self.debits_posted - self.credits_posted,
            _ => self.credits_posted - self.debits_posted,
        }
    }

    /// Funds available to spend right now: posted balance minus pending blocks.
    pub fn available_balance(&self) -> i64 {
        self.posted_balance() - self.amount_pending
    }
}

/// A committed journal entry header. The actual postings live in the associated
/// [`LedgerLine`] rows. Immutable once committed.
#[derive(Debug)]
pub struct JournalEntry {
    pub id: JournalEntryId,
    /// Caller-supplied idempotency key — duplicate submissions with the same
    /// `client_id` return this entry rather than posting again.
    pub client_id: String,
    pub created_at: SystemTime,
}

/// The direction of a single posting in a journal entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostingDirection {
    Debit,
    Credit,
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
    /// Returns the debit amount in minor units, or 0 if this is a Credit posting.
    pub fn debit(&self) -> i64 {
        match self {
            Self::Debit(v) => v.get() as i64,
            Self::Credit(_) => 0,
        }
    }

    /// Returns the credit amount in minor units, or 0 if this is a Debit posting.
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

    pub fn direction(&self) -> PostingDirection {
        match self {
            Self::Debit(_) => PostingDirection::Debit,
            Self::Credit(_) => PostingDirection::Credit,
        }
    }
}

/// One committed leg of a journal entry as stored in the ledger. Immutable.
#[derive(Debug)]
pub struct LedgerLine {
    pub id: LedgerLineId,
    pub journal_entry_id: JournalEntryId,
    pub account: AccountId,
    pub amount: i64,
    pub direction: PostingDirection,
    pub created_at: SystemTime,
}

impl LedgerLine {
    /// Reconstruct the typed [`Posting`] from the stored amount and direction.
    pub fn posting(&self) -> Posting {
        let amount = NonZeroU64::new(self.amount as u64).expect("ledger line amount > 0");
        match self.direction {
            PostingDirection::Debit => Posting::Debit(amount),
            PostingDirection::Credit => Posting::Credit(amount),
        }
    }
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
    /// Amount reserved, in minor units.
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
