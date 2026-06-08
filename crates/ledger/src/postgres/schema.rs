// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "account_type"))]
    pub struct AccountType;

    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "posting_direction"))]
    pub struct PostingDirection;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::AccountType;

    accounts (id) {
        id -> Int8,
        #[max_length = 64]
        client_id -> Varchar,
        account_type -> AccountType,
        name -> Varchar,
        active -> Bool,
        debits_posted -> Int8,
        credits_posted -> Int8,
        amount_pending -> Int8,
        created_at -> Timestamp,
    }
}

diesel::table! {
    journal_entries (id) {
        id -> Int8,
        #[max_length = 64]
        client_id -> Varchar,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::PostingDirection;

    ledger_lines (id) {
        id -> Int8,
        journal_entry_id -> Int8,
        account -> Int8,
        amount -> Int8,
        direction -> PostingDirection,
        created_at -> Timestamp,
    }
}

diesel::table! {
    account_blocks (id) {
        id -> Int8,
        #[max_length = 64]
        client_id -> Varchar,
        account_id -> Int8,
        amount -> Int8,
        released -> Bool,
        created_at -> Timestamp,
    }
}

diesel::table! {
    outbox (id) {
        id -> Int8,
        event_type -> Varchar,
        payload -> Jsonb,
        created_at -> Timestamp,
        delivered_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(account_blocks -> accounts (account_id));
diesel::joinable!(ledger_lines -> accounts (account));
diesel::joinable!(ledger_lines -> journal_entries (journal_entry_id));

diesel::allow_tables_to_appear_in_same_query!(
    account_blocks,
    accounts,
    journal_entries,
    ledger_lines,
    outbox,
);
