use std::{num::NonZeroU64, time::SystemTime};

pub type AccountId = i32;
pub type JournalEntryId = i32;
pub type LedgerLineId = i32;
pub type AccountBlockId = i32;

pub use ledger_api::AccountType;

#[derive(Debug)]
pub struct Account {
    pub id: AccountId,
    pub client_id: String,
    pub account_type: AccountType,
    pub active: bool,
    pub name: String,
    pub created_at: SystemTime,
}

#[derive(Debug)]
pub struct JournalEntry {
    pub id: JournalEntryId,
    pub client_id: String,
    pub created_at: SystemTime,
    pub updated_at: Option<SystemTime>,
}

/// A single debit or credit posting to an account within a journal entry.
pub enum Posting {
    Debit(NonZeroU64),
    Credit(NonZeroU64),
}

impl Posting {
    pub fn debit(&self) -> i64 {
        match self {
            Self::Debit(v) => v.get() as i64,
            Self::Credit(_) => 0,
        }
    }

    pub fn credit(&self) -> i64 {
        match self {
            Self::Credit(v) => v.get() as i64,
            Self::Debit(_) => 0,
        }
    }

    pub fn amount(&self) -> NonZeroU64 {
        match self {
            Self::Debit(v) | Self::Credit(v) => *v,
        }
    }
}

#[derive(Debug)]
pub struct LedgerLine {
    pub id: LedgerLineId,
    pub journal_entry_id: JournalEntryId,
    pub account: AccountId,
    pub debit: i64,
    pub credit: i64,
    pub created_at: SystemTime,
}

impl LedgerLine {
    pub fn posting(&self) -> Posting {
        if self.debit > 0 {
            Posting::Debit(NonZeroU64::new(self.debit as u64).expect("debit checked > 0"))
        } else {
            Posting::Credit(NonZeroU64::new(self.credit as u64).expect("credit checked > 0"))
        }
    }
}

#[derive(Debug)]
pub struct Balance {
    pub account_id: AccountId,
    pub balance: i64,
    pub updated_at: SystemTime,
}

/// Input for one line of a journal entry.
pub struct NewLedgerLineInput {
    pub account_id: AccountId,
    pub posting: Posting,
}

#[derive(Debug)]
pub struct AccountBlock {
    pub id: AccountBlockId,
    pub client_id: String,
    pub account_id: AccountId,
    pub amount: i64,
    pub released: bool,
    pub created_at: SystemTime,
}

pub struct TrialBalanceReport {
    pub asset: i64,
    pub expense: i64,
    pub liability: i64,
    pub equity: i64,
    pub revenue: i64,
    pub is_balanced: bool,
}
