use super::schema::{account_blocks, accounts, journal_entries, ledger_lines, outbox};
use diesel::prelude::*;
use std::time::SystemTime;

use crate::domain;

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
#[ExistingTypePath = "crate::postgres::schema::sql_types::PostingDirection"]
pub(super) enum PostingDirection {
    Debit,
    Credit,
}

impl From<PostingDirection> for domain::PostingDirection {
    fn from(d: PostingDirection) -> Self {
        match d {
            PostingDirection::Debit => domain::PostingDirection::Debit,
            PostingDirection::Credit => domain::PostingDirection::Credit,
        }
    }
}

impl From<domain::PostingDirection> for PostingDirection {
    fn from(d: domain::PostingDirection) -> Self {
        match d {
            domain::PostingDirection::Debit => PostingDirection::Debit,
            domain::PostingDirection::Credit => PostingDirection::Credit,
        }
    }
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = accounts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(super) struct Account {
    pub id: i64,
    pub client_id: String,
    pub account_type: AccountType,
    pub name: String,
    pub active: bool,
    pub debits_posted: i64,
    pub credits_posted: i64,
    pub amount_pending: i64,
    pub created_at: SystemTime,
}

impl Account {
    pub fn posted_balance(&self) -> i64 {
        match self.account_type {
            AccountType::Asset | AccountType::Expense => self.debits_posted - self.credits_posted,
            _ => self.credits_posted - self.debits_posted,
        }
    }

    pub fn available_balance(&self) -> i64 {
        self.posted_balance() - self.amount_pending
    }
}

impl From<Account> for domain::Account {
    fn from(a: Account) -> Self {
        domain::Account {
            id: a.id,
            client_id: a.client_id,
            account_type: a.account_type.into(),
            active: a.active,
            name: a.name,
            debits_posted: a.debits_posted,
            credits_posted: a.credits_posted,
            amount_pending: a.amount_pending,
            created_at: a.created_at,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = accounts)]
pub(super) struct NewAccount<'a> {
    pub client_id: &'a str,
    pub name: &'a str,
    pub account_type: AccountType,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = journal_entries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(super) struct JournalEntry {
    pub id: i64,
    pub client_id: String,
    pub created_at: SystemTime,
}

impl From<JournalEntry> for domain::JournalEntry {
    fn from(e: JournalEntry) -> Self {
        domain::JournalEntry {
            id: e.id,
            client_id: e.client_id,
            created_at: e.created_at,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = journal_entries)]
pub(super) struct NewJournalEntry<'a> {
    pub client_id: &'a str,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = ledger_lines)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(super) struct LedgerLine {
    pub id: i64,
    pub journal_entry_id: i64,
    pub account: i64,
    pub amount: i64,
    pub direction: PostingDirection,
    pub created_at: SystemTime,
}

impl From<LedgerLine> for domain::LedgerLine {
    fn from(l: LedgerLine) -> Self {
        domain::LedgerLine {
            id: l.id,
            journal_entry_id: l.journal_entry_id,
            account: l.account,
            amount: l.amount,
            direction: l.direction.into(),
            created_at: l.created_at,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = ledger_lines)]
pub(super) struct NewLedgerLine {
    pub journal_entry_id: i64,
    pub account: i64,
    pub amount: i64,
    pub direction: PostingDirection,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = account_blocks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(super) struct AccountBlock {
    pub id: i64,
    pub client_id: String,
    pub account_id: i64,
    pub amount: i64,
    pub released: bool,
    pub created_at: SystemTime,
}

impl From<AccountBlock> for domain::AccountBlock {
    fn from(b: AccountBlock) -> Self {
        domain::AccountBlock {
            id: b.id,
            client_id: b.client_id,
            account_id: b.account_id,
            amount: b.amount,
            released: b.released,
            created_at: b.created_at,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = account_blocks)]
pub(super) struct NewAccountBlock<'a> {
    pub client_id: &'a str,
    pub account_id: i64,
    pub amount: i64,
}

// ── Outbox ────────────────────────────────────────────────────────────────────

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = outbox)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(super) struct OutboxRow {
    pub id: i64,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: SystemTime,
    pub delivered_at: Option<SystemTime>,
}

#[derive(Insertable)]
#[diesel(table_name = outbox)]
pub(super) struct NewOutboxEvent<'a> {
    pub event_type: &'a str,
    pub payload: serde_json::Value,
}
