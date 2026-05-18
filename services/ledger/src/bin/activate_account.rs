use ledger::{LedgerStore, PostgresStore};
use std::env::args;

fn main() {
    let id = args()
        .nth(1)
        .expect("activate_account requires an account id")
        .parse::<i32>()
        .expect("Invalid ID");

    let mut store = PostgresStore::from_env();
    store.activate_account(id).unwrap();
    println!("Account is now active");
}
