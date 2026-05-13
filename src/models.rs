use crate::schema::{
    account_blocks, accounts, balances, movements, transactions, transfer_internal,
};
use diesel::prelude::*;
use std::{str::FromStr, time::SystemTime};

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
    pub id: i32,
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
#[diesel(table_name = crate::schema::transactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Transaction {
    pub id: i32,
    pub commited: bool,
    pub created_at: SystemTime,
    pub updated_at: Option<SystemTime>,
}

#[derive(Insertable)]
#[diesel(table_name = transactions)]
pub struct NewTransaction {
    pub commited: bool,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::schema::movements)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Movement {
    pub id: i32,
    pub tx: i32,
    pub account: i32,
    pub debit: i32,
    pub credit: i32,
    pub created_at: SystemTime,
}

#[derive(Insertable)]
#[diesel(table_name = movements)]
pub struct NewMovement {
    pub tx: i32,
    pub account: i32,
    pub debit: i32,
    pub credit: i32,
}

/// One leg of a journal entry; not a DB struct, used as input to post_journal_entry.
/// Represent a debit leg with credit = 0 and a credit leg with debit = 0.
pub struct NewMovementInput {
    pub account_id: i32,
    pub debit: i32,
    pub credit: i32,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::schema::balances)]
#[diesel(primary_key(account_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Balance {
    pub account_id: i32,
    pub commited_balance: i32,
    pub blocked: i32,
    pub updated_at: SystemTime,
}

#[derive(Insertable)]
#[diesel(table_name = balances)]
pub struct NewBalance {
    pub account_id: i32,
    pub commited_balance: i32,
    pub blocked: i32,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::schema::account_blocks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AccountBlock {
    pub id: i32,
    pub account_id: i32,
    pub amount: i32,
    pub created_at: SystemTime,
    pub transaction_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = account_blocks)]
pub struct NewAccountBlock {
    pub account_id: i32,
    pub amount: i32,
    pub transaction_id: i32,
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::schema::transfer_internal)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TransferInternal {
    pub id: i32,
    pub transaction_id: i32,
    pub from_account_id: i32,
    pub to_account_id: i32,
    pub amount: i32,
    pub initiated_at: SystemTime,
    pub completed_at: Option<SystemTime>,
    pub status: TransferStatus,
}

#[derive(Insertable)]
#[diesel(table_name = transfer_internal)]
pub struct NewTransferInternal {
    pub transaction_id: i32,
    pub from_account_id: i32,
    pub to_account_id: i32,
    pub amount: i32,
    pub status: TransferStatus,
}
