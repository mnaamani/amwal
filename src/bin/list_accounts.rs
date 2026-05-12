use self::models::*;
use diesel::prelude::*;
use ledger::*;

fn main() {
    use self::schema::accounts::dsl::*;

    let connection = &mut db_connect();
    let results = accounts
        .filter(active.eq(true))
        .limit(5)
        .select(Account::as_select())
        .load(connection)
        .expect("Error loading accounts");

    println!("Displaying {} accounts", results.len());
    for account in results {
        println!("id={} name={}", account.id, account.name);
    }
}
