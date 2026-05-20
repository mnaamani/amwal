use ledger::LedgerService;
use ledger_api::LedgerClient;

fn main() {
    let svc = LedgerService::from_env();
    run(&svc);
}

fn run(ledger: &dyn LedgerClient) {
    let accounts = ledger.list_active_accounts().unwrap();
    println!("Displaying {} accounts", accounts.len());
    for account in accounts {
        println!("account id: {} name: {}", account.id, account.name);
    }
}
