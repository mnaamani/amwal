use ledger::LedgerService;
use ledger_api::LedgerClient;
use std::env::args;

fn main() {
    let account_id = args()
        .nth(1)
        .expect("get_account_details requires an account id")
        .parse::<i32>()
        .expect("Invalid ID");

    let svc = LedgerService::from_env();
    run(&svc, account_id);
}

fn run(ledger: &dyn LedgerClient, account_id: i32) {
    match ledger.get_account(account_id) {
        Ok(Some(account)) => println!("{:?}", account),
        Ok(None) => println!("Unable to find account {}", account_id),
        Err(_) => println!("An error occurred while fetching account {}", account_id),
    }
}
