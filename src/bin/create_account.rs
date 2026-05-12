use ledger::models::AccountType;
use ledger::{create_account, db_connect};
use std::io::stdin;

fn main() {
    let conn = &mut db_connect();
    let mut account_name = String::new();
    let mut account_type = String::new();

    println!("Enter account Name:");
    stdin().read_line(&mut account_name).unwrap();
    let account_name = account_name.trim();

    println!("Enter account type, [asset, liability, equity, expense, revenue]:");
    stdin().read_line(&mut account_type).unwrap();
    let account_type: AccountType = account_type.trim().parse().unwrap();

    let new_account = create_account(conn, &account_name, account_type).unwrap();
    println!("Created Account id = {}", new_account.id);
}
