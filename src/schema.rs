// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "account_type"))]
    pub struct AccountType;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::AccountType;

    ledger_accounts (id) {
        id -> Int4,
        account_type -> AccountType,
        name -> Varchar,
        active -> Bool,
        created_at -> Timestamp,
    }
}
