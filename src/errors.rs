use diesel::result::{DatabaseErrorKind, Error as DieselError};

use crate::models::AccountId;

#[derive(Debug)]
pub enum DbErrorKind {
    NotFound,
    CheckViolation,
    UniqueViolation,
    Connection,
    Other(String),
}

impl From<DieselError> for DbErrorKind {
    fn from(e: DieselError) -> Self {
        match e {
            DieselError::NotFound => DbErrorKind::NotFound,
            DieselError::DatabaseError(kind, info) => match kind {
                DatabaseErrorKind::CheckViolation => DbErrorKind::CheckViolation,
                DatabaseErrorKind::UniqueViolation => DbErrorKind::UniqueViolation,
                _ => DbErrorKind::Other(info.message().to_string()),
            },
            _ => DbErrorKind::Other(e.to_string()),
        }
    }
}

#[derive(Debug)]
pub enum LedgerError {
    Db(DbErrorKind),
    ImbalancedEntry {
        total_debits: i64,
        total_credits: i64,
    },
    InsufficientFunds {
        available: i32,
        requested: i32,
    },
    AccountNotActive(AccountId),
    InvalidMovement(String),
    InvalidTransferState(String),
}

impl From<DieselError> for LedgerError {
    fn from(e: DieselError) -> Self {
        LedgerError::Db(DbErrorKind::from(e))
    }
}
