use ledger::{db_connect, get_account};
use std::env::args;

fn main() {
    let account_id = args()
        .nth(1)
        .expect("get_accounts requires an account id")
        .parse::<i32>()
        .expect("Invalid ID");

    let connection = &mut db_connect();

    let account = get_account(connection, account_id);

    match account {
        Ok(Some(account)) => println!("Account id: {} name: {}", account.id, account.name),
        Ok(None) => println!("Unable to find account {}", account_id),
        Err(_) => println!("An error occured while fetching account {}", account_id),
    }
}
