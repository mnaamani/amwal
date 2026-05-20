use ledger::LedgerService;
use ledger_api::LedgerClient;
use std::env::args;

fn main() {
    let id = args()
        .nth(1)
        .expect("activate_account requires an account id")
        .parse::<i32>()
        .expect("Invalid ID");

    let svc = LedgerService::from_env();
    run(&svc, id);
}

fn run(ledger: &dyn LedgerClient, id: i32) {
    ledger.activate_account(id).unwrap();
    println!("Account is now active");
}
