use std::{num::NonZeroU64, str::FromStr, time::SystemTime};

pub type AccountId = i32;
pub type JournalEntryId = i32;
pub type LedgerLineId = i32;
pub type AccountBlockId = i32;
pub type TransferInternalId = i32;

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

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum TransferStatus {
    Pending,
    Cancelled,
    Completed,
}

#[derive(Debug)]
pub struct Account {
    pub id: AccountId,
    pub account_type: AccountType,
    pub active: bool,
    pub name: String,
    pub created_at: SystemTime,
}

#[derive(Debug)]
pub struct JournalEntry {
    pub id: JournalEntryId,
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

#[derive(Debug)]
pub struct AccountBlock {
    pub id: AccountBlockId,
    pub account_id: AccountId,
    pub amount: i64,
    pub created_at: SystemTime,
    pub transfer_id: TransferInternalId,
}

#[derive(Debug)]
pub struct TransferInternal {
    pub id: TransferInternalId,
    pub journal_entry_id: Option<JournalEntryId>,
    pub from_account_id: AccountId,
    pub to_account_id: AccountId,
    pub amount: i64,
    pub created_at: SystemTime,
    pub completed_at: Option<SystemTime>,
    pub status: TransferStatus,
}

/// Input for one line of a journal entry — the type system enforces exactly one posting per line.
pub struct NewLedgerLineInput {
    pub account_id: AccountId,
    pub posting: Posting,
}

pub struct NewTransferInternal {
    pub from_account_id: AccountId,
    pub to_account_id: AccountId,
    pub amount: i64,
}

pub struct TrialBalanceReport {
    pub asset: i64,
    pub expense: i64,
    pub liability: i64,
    pub equity: i64,
    pub revenue: i64,
    pub is_balanced: bool,
}
