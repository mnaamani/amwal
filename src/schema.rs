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
        account_id -> Nullable<Int4>,
        amount -> Nullable<Int4>,
        created_at -> Timestamp,
        transaction_id -> Nullable<Int4>,
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
        commited_balance -> Nullable<Int4>,
        blocked -> Nullable<Int4>,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    movements (id) {
        id -> Int4,
        tx -> Nullable<Int4>,
        account -> Nullable<Int4>,
        debit -> Nullable<Int4>,
        credit -> Nullable<Int4>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    transactions (id) {
        id -> Int4,
        commited -> Nullable<Bool>,
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
        from_account_id -> Nullable<Int4>,
        to_account_id -> Nullable<Int4>,
        amount -> Nullable<Int4>,
        initiated_at -> Nullable<Timestamp>,
        completed_at -> Nullable<Timestamp>,
        status -> Nullable<TransferStatus>,
    }
}

diesel::joinable!(account_blocks -> accounts (account_id));
diesel::joinable!(account_blocks -> transactions (transaction_id));
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
