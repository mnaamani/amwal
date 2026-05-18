use ledger::*;
use std::env::args;

fn main() {
    let target = args().nth(1).expect("Expected a target to match against");
    let pattern = format!("%{}%", target);

    let conn = &mut db_connect();
    let num_deleted = delete_account(conn, &pattern).expect("Error deleting account(s)");

    println!("Deleted {} accounts", num_deleted);
}
