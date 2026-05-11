use crate::schema::ledger_accounts;
use diesel::prelude::*;
use std::time::SystemTime;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[derive(diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::schema::sql_types::AccountType"]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::schema::ledger_accounts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Account {
    pub id: i32,
    pub account_type: AccountType,
    pub active: bool,
    pub name: String,
    pub created_at: SystemTime,
}

#[derive(Insertable)]
#[diesel(table_name = ledger_accounts)]
pub struct NewAccount<'a> {
    pub name: &'a str,
    pub account_type: AccountType,
}