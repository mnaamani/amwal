// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "transfer_status"))]
    pub struct TransferStatus;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::TransferStatus;

    transfer_internal (id) {
        id -> Int4,
        #[max_length = 32]
        client_id -> Varchar,
        from_account_id -> Int4,
        to_account_id -> Int4,
        amount -> Int8,
        status -> TransferStatus,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}
