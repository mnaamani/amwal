// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "account_type"))]
    pub struct AccountType;
}

diesel::table! {
    account_blocks (id) {
        id -> Int4,
        #[max_length = 32]
        client_id -> Varchar,
        account_id -> Int4,
        amount -> Int8,
        released -> Bool,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::AccountType;

    accounts (id) {
        id -> Int4,
        #[max_length = 32]
        client_id -> Varchar,
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
        #[max_length = 32]
        client_id -> Varchar,
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

diesel::joinable!(account_blocks -> accounts (account_id));
diesel::joinable!(balances -> accounts (account_id));
diesel::joinable!(ledger_lines -> accounts (account));
diesel::joinable!(ledger_lines -> journal_entries (journal_entry_id));

diesel::allow_tables_to_appear_in_same_query!(
    account_blocks,
    accounts,
    balances,
    journal_entries,
    ledger_lines,
);
