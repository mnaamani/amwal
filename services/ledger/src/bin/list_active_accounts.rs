use ledger::{db_connect, get_active_accounts};

fn main() {
    let connection = &mut db_connect();

    let results = get_active_accounts(connection).unwrap();

    println!("Displaying {} accounts", results.len());
    for account in results {
        println!("account id: {} name: {}", account.id, account.name);
    }
}
