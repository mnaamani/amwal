use crate::schema::{account_blocks, accounts, balances, ledger_lines, transfer_internal};
use diesel::prelude::*;
use std::{num::NonZeroU64, str::FromStr, time::SystemTime};

pub type AccountId = i32;
pub type JournalEntryId = i32;
pub type LedgerLineId = i32;
pub type AccountBlockId = i32;
pub type TransferInternalId = i32;

// When deriving the Enum here, there will be a duplicate Clone trait
// derived on the existing type, manually remove it there to fix compiler error
#[derive(Debug, PartialEq, Eq, Copy, Clone, diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::schema::sql_types::AccountType"]
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

// When deriving the Enum here, there will be a duplicate Clone trait
// derived on the existing type, manually remove it there to fix compiler error
#[derive(Debug, PartialEq, Eq, Copy, Clone, diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::schema::sql_types::TransferStatus"]
pub enum TransferStatus {
    Pending,
    Cancelled,
    Completed,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::schema::accounts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Account {
    pub id: AccountId,
    pub account_type: AccountType,
    pub active: bool,
    pub name: String,
    pub created_at: SystemTime,
}

#[derive(Insertable)]
#[diesel(table_name = accounts)]
pub struct NewAccount<'a> {
    pub name: &'a str,
    pub account_type: AccountType,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::schema::journal_entries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
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

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::schema::ledger_lines)]
#[diesel(check_for_backend(diesel::pg::Pg))]
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

#[derive(Insertable)]
#[diesel(table_name = ledger_lines)]
pub struct NewLedgerLine {
    pub journal_entry_id: JournalEntryId,
    pub account: AccountId,
    pub debit: i64,
    pub credit: i64,
}

/// Input for one line of a journal entry — the type system enforces exactly one posting per line.
pub struct NewLedgerLineInput {
    pub account_id: AccountId,
    pub posting: Posting,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::schema::balances)]
#[diesel(primary_key(account_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Balance {
    pub account_id: AccountId,
    pub balance: i64,
    pub updated_at: SystemTime,
}

#[derive(Insertable)]
#[diesel(table_name = balances)]
pub struct NewBalance {
    pub account_id: AccountId,
    pub balance: i64,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::schema::account_blocks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AccountBlock {
    pub id: AccountBlockId,
    pub account_id: AccountId,
    pub amount: i64,
    pub created_at: SystemTime,
    pub transfer_id: TransferInternalId,
}

#[derive(Insertable)]
#[diesel(table_name = account_blocks)]
pub struct NewAccountBlock {
    pub account_id: AccountId,
    pub amount: i64,
    pub transfer_id: TransferInternalId,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::schema::transfer_internal)]
#[diesel(check_for_backend(diesel::pg::Pg))]
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

#[derive(Insertable)]
#[diesel(table_name = transfer_internal)]
pub struct NewTransferInternal {
    pub from_account_id: AccountId,
    pub to_account_id: AccountId,
    pub amount: i64,
    pub status: TransferStatus,
}
