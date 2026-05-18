use ledger::{LedgerStore, PostgresStore};
use std::env::args;

fn main() {
    let account_id = args()
        .nth(1)
        .expect("get_account_details requires an account id")
        .parse::<i32>()
        .expect("Invalid ID");

    let mut store = PostgresStore::from_env();

    match store.get_account(account_id) {
        Ok(Some(account)) => println!("Account id: {} name: {}", account.id, account.name),
        Ok(None) => println!("Unable to find account {}", account_id),
        Err(_) => println!("An error occurred while fetching account {}", account_id),
    }
}
