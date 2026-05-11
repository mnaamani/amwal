use diesel::prelude::*;
use ledger::*;
use std::env::args;

fn main() {
    use self::schema::ledger_accounts::dsl::*;

    let target = args().nth(1).expect("Expected a target to match against");
    let pattern = format!("%{}%", target);

    let connection = &mut db_connect();
    let num_deleted = diesel::delete(ledger_accounts.filter(name.like(pattern)))
        .execute(connection)
        .expect("Error deleting accounts");

    println!("Deleted {} accounts", num_deleted);
}
