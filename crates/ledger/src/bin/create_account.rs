use ledger::LedgerService;
use ledger_api::{AccountType, LedgerClient};
use std::io::stdin;

fn main() {
    let client_id = std::env::args()
        .nth(1)
        .expect("Usage: create_account <client_id>");

    let svc = LedgerService::from_env();
    run(&svc, &client_id);
}

fn run(ledger: &dyn LedgerClient, client_id: &str) {
    let mut account_name = String::new();
    let mut account_type_input = String::new();

    println!("Enter account Name:");
    stdin().read_line(&mut account_name).unwrap();
    let account_name = account_name.trim();

    println!("Enter account type, [asset, liability, equity, expense, revenue]:");
    stdin().read_line(&mut account_type_input).unwrap();
    let account_type: AccountType = account_type_input.trim().parse().unwrap();

    let account = ledger.create_account(client_id, account_name, account_type).unwrap();
    println!("Created Account id = {}", account.id);
}
