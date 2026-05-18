use ledger::{LedgerStore, PostgresStore};

fn main() {
    let mut store = PostgresStore::from_env();

    let results = store.get_active_accounts().unwrap();

    println!("Displaying {} accounts", results.len());
    for account in results {
        println!("account id: {} name: {}", account.id, account.name);
    }
}
