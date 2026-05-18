use super::schema::{
    account_blocks, accounts, balances, journal_entries, ledger_lines, transfer_internal,
};
use diesel::prelude::*;
use std::time::SystemTime;

use crate::domain;

// When deriving DbEnum, there will be a duplicate Clone trait derived on the sql_types struct.
// Manually remove Clone from the sql_types derive in schema.rs to fix the compiler error.

#[derive(Debug, PartialEq, Eq, Copy, Clone, diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::postgres::schema::sql_types::AccountType"]
pub(super) enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

impl From<AccountType> for domain::AccountType {
    fn from(t: AccountType) -> Self {
        match t {
            AccountType::Asset => domain::AccountType::Asset,
            AccountType::Liability => domain::AccountType::Liability,
            AccountType::Equity => domain::AccountType::Equity,
            AccountType::Revenue => domain::AccountType::Revenue,
            AccountType::Expense => domain::AccountType::Expense,
        }
    }
}

impl From<domain::AccountType> for AccountType {
    fn from(t: domain::AccountType) -> Self {
        match t {
            domain::AccountType::Asset => AccountType::Asset,
            domain::AccountType::Liability => AccountType::Liability,
            domain::AccountType::Equity => AccountType::Equity,
            domain::AccountType::Revenue => AccountType::Revenue,
            domain::AccountType::Expense => AccountType::Expense,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::postgres::schema::sql_types::TransferStatus"]
pub(super) enum TransferStatus {
    Pending,
    Cancelled,
    Completed,
}

impl From<TransferStatus> for domain::TransferStatus {
    fn from(s: TransferStatus) -> Self {
        match s {
            TransferStatus::Pending => domain::TransferStatus::Pending,
            TransferStatus::Cancelled => domain::TransferStatus::Cancelled,
            TransferStatus::Completed => domain::TransferStatus::Completed,
        }
    }
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = accounts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(super) struct Account {
    pub id: i32,
    pub account_type: AccountType,
    pub active: bool,
    pub name: String,
    pub created_at: SystemTime,
}

impl From<Account> for domain::Account {
    fn from(a: Account) -> Self {
        domain::Account {
            id: a.id,
            account_type: a.account_type.into(),
            active: a.active,
            name: a.name,
            created_at: a.created_at,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = accounts)]
pub(super) struct NewAccount<'a> {
    pub name: &'a str,
    pub account_type: AccountType,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = journal_entries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(super) struct JournalEntry {
    pub id: i32,
    pub created_at: SystemTime,
    pub updated_at: Option<SystemTime>,
}

impl From<JournalEntry> for domain::JournalEntry {
    fn from(e: JournalEntry) -> Self {
        domain::JournalEntry {
            id: e.id,
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = ledger_lines)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(super) struct LedgerLine {
    pub id: i32,
    pub journal_entry_id: i32,
    pub account: i32,
    pub debit: i64,
    pub credit: i64,
    pub created_at: SystemTime,
}

impl From<LedgerLine> for domain::LedgerLine {
    fn from(l: LedgerLine) -> Self {
        domain::LedgerLine {
            id: l.id,
            journal_entry_id: l.journal_entry_id,
            account: l.account,
            debit: l.debit,
            credit: l.credit,
            created_at: l.created_at,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = ledger_lines)]
pub(super) struct NewLedgerLine {
    pub journal_entry_id: i32,
    pub account: i32,
    pub debit: i64,
    pub credit: i64,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = balances)]
#[diesel(primary_key(account_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(super) struct Balance {
    pub account_id: i32,
    pub balance: i64,
    pub updated_at: SystemTime,
}

impl From<Balance> for domain::Balance {
    fn from(b: Balance) -> Self {
        domain::Balance {
            account_id: b.account_id,
            balance: b.balance,
            updated_at: b.updated_at,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = balances)]
pub(super) struct NewBalance {
    pub account_id: i32,
    pub balance: i64,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = account_blocks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(super) struct AccountBlock {
    pub id: i32,
    pub account_id: i32,
    pub amount: i64,
    pub created_at: SystemTime,
    pub transfer_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = account_blocks)]
pub(super) struct NewAccountBlock {
    pub account_id: i32,
    pub amount: i64,
    pub transfer_id: i32,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = transfer_internal)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(super) struct TransferInternal {
    pub id: i32,
    pub journal_entry_id: Option<i32>,
    pub from_account_id: i32,
    pub to_account_id: i32,
    pub amount: i64,
    pub created_at: SystemTime,
    pub completed_at: Option<SystemTime>,
    pub status: TransferStatus,
}

#[derive(Insertable)]
#[diesel(table_name = transfer_internal)]
pub(super) struct NewTransferInternal {
    pub from_account_id: i32,
    pub to_account_id: i32,
    pub amount: i64,
}
