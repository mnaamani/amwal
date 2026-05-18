use ledger::{AccountType, LedgerStore, PostgresStore};
use std::io::stdin;

fn main() {
    let mut store = PostgresStore::from_env();
    let mut account_name = String::new();
    let mut account_type_input = String::new();

    println!("Enter account Name:");
    stdin().read_line(&mut account_name).unwrap();
    let account_name = account_name.trim();

    println!("Enter account type, [asset, liability, equity, expense, revenue]:");
    stdin().read_line(&mut account_type_input).unwrap();
    let account_type: AccountType = account_type_input.trim().parse().unwrap();

    let new_account = store.create_account(account_name, account_type).unwrap();
    println!("Created Account id = {}", new_account.id);
}
