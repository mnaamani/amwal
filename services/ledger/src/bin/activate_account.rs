use ledger::{activate_account, db_connect};
use std::env::args;

fn main() {
    let id = args()
        .nth(1)
        .expect("set_account_active requires an account id")
        .parse::<i32>()
        .expect("Invalid ID");

    let conn = &mut db_connect();

    activate_account(conn, id).unwrap();

    println!("Account is now active");
}
