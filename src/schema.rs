// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "account_type"))]
    pub struct AccountType;

    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "transfer_status"))]
    pub struct TransferStatus;
}

diesel::table! {
    account_blocks (id) {
        id -> Int4,
        account_id -> Int4,
        amount -> Int4,
        created_at -> Timestamp,
        transfer_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::AccountType;

    accounts (id) {
        id -> Int4,
        account_type -> AccountType,
        name -> Varchar,
        active -> Bool,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    balances (account_id) {
        account_id -> Int4,
        commited_balance -> Int4,
        blocked -> Int4,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    movements (id) {
        id -> Int4,
        tx -> Int4,
        account -> Int4,
        debit -> Int4,
        credit -> Int4,
        created_at -> Timestamp,
    }
}

diesel::table! {
    transactions (id) {
        id -> Int4,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::TransferStatus;

    transfer_internal (id) {
        id -> Int4,
        transaction_id -> Nullable<Int4>,
        from_account_id -> Int4,
        to_account_id -> Int4,
        amount -> Int4,
        initiated_at -> Timestamp,
        completed_at -> Nullable<Timestamp>,
        status -> TransferStatus,
    }
}

diesel::joinable!(account_blocks -> accounts (account_id));
diesel::joinable!(account_blocks -> transfer_internal (transfer_id));
diesel::joinable!(balances -> accounts (account_id));
diesel::joinable!(movements -> accounts (account));
diesel::joinable!(movements -> transactions (tx));
diesel::joinable!(transfer_internal -> transactions (transaction_id));

diesel::allow_tables_to_appear_in_same_query!(
    account_blocks,
    accounts,
    balances,
    movements,
    transactions,
    transfer_internal,
);
