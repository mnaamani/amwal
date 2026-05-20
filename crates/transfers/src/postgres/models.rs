use super::schema::transfer_internal;
use diesel::prelude::*;
use std::time::SystemTime;

#[derive(Debug, PartialEq, Eq, Copy, Clone, diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::postgres::schema::sql_types::TransferStatus"]
pub(super) enum TransferStatus {
    Pending,
    Cancelled,
    Completed,
    Failed,
}

impl From<TransferStatus> for crate::TransferStatus {
    fn from(s: TransferStatus) -> Self {
        match s {
            TransferStatus::Pending => crate::TransferStatus::Pending,
            TransferStatus::Cancelled => crate::TransferStatus::Cancelled,
            TransferStatus::Completed => crate::TransferStatus::Completed,
            TransferStatus::Failed => crate::TransferStatus::Failed,
        }
    }
}

impl From<crate::TransferStatus> for TransferStatus {
    fn from(s: crate::TransferStatus) -> Self {
        match s {
            crate::TransferStatus::Pending => TransferStatus::Pending,
            crate::TransferStatus::Cancelled => TransferStatus::Cancelled,
            crate::TransferStatus::Completed => TransferStatus::Completed,
            crate::TransferStatus::Failed => TransferStatus::Failed,
        }
    }
}

#[derive(Queryable, Selectable, Identifiable)]
#[diesel(table_name = transfer_internal)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(super) struct TransferInternal {
    pub id: i32,
    pub client_id: String,
    pub from_account_id: i32,
    pub to_account_id: i32,
    pub amount: i64,
    pub status: TransferStatus,
    pub created_at: SystemTime,
    pub updated_at: Option<SystemTime>,
}

impl From<TransferInternal> for crate::Transfer {
    fn from(t: TransferInternal) -> Self {
        crate::Transfer {
            id: t.id,
            client_id: t.client_id,
            from_account_id: t.from_account_id,
            to_account_id: t.to_account_id,
            amount: t.amount,
            status: t.status.into(),
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = transfer_internal)]
pub(super) struct NewTransferInternal<'a> {
    pub client_id: &'a str,
    pub from_account_id: i32,
    pub to_account_id: i32,
    pub amount: i64,
}
