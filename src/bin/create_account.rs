use ledger::models::AccountType;
use ledger::{create_account, db_connect};
use std::io::stdin;

fn main() {
    let conn = &mut db_connect();
    let mut account_name = String::new();

    println!("Enter account Name:");
    stdin().read_line(&mut account_name).unwrap();
    let account_name = account_name.trim();

    let new_account = create_account(conn, &account_name, AccountType::Asset);
    println!("Created Account id = {}", new_account.id);
}
