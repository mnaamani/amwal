use crate::schema::accounts;
use diesel::prelude::*;
use std::time::SystemTime;

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
