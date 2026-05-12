use diesel::prelude::*;
use ledger::db_connect;
use ledger::models::Account;
use std::env::args;

fn main() {
    use ledger::schema::accounts::dsl::{accounts, active};

    let id = args()
        .nth(1)
        .expect("set_account_active requires an account id")
        .parse::<i32>()
        .expect("Invalid ID");

    let conn = &mut db_connect();

    let account = diesel::update(accounts.find(id))
        .set(active.eq(true))
        .returning(Account::as_returning())
        .get_result(conn)
        .unwrap();

    println!(
        "Account id: {} name: {}, is now active.",
        account.id, account.name
    );
}
