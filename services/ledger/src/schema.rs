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
        amount -> Int8,
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
        balance -> Int8,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    journal_entries (id) {
        id -> Int4,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    ledger_lines (id) {
        id -> Int4,
        journal_entry_id -> Int4,
        account -> Int4,
        debit -> Int8,
        credit -> Int8,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::TransferStatus;

    transfer_internal (id) {
        id -> Int4,
        journal_entry_id -> Nullable<Int4>,
        from_account_id -> Int4,
        to_account_id -> Int4,
        amount -> Int8,
        created_at -> Timestamp,
        completed_at -> Nullable<Timestamp>,
        status -> TransferStatus,
    }
}

diesel::joinable!(account_blocks -> accounts (account_id));
diesel::joinable!(account_blocks -> transfer_internal (transfer_id));
diesel::joinable!(balances -> accounts (account_id));
diesel::joinable!(ledger_lines -> accounts (account));
diesel::joinable!(ledger_lines -> journal_entries (journal_entry_id));
diesel::joinable!(transfer_internal -> journal_entries (journal_entry_id));

diesel::allow_tables_to_appear_in_same_query!(
    account_blocks,
    accounts,
    balances,
    journal_entries,
    ledger_lines,
    transfer_internal,
);
