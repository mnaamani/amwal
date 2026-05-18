use ledger::{LedgerStore, PostgresStore};
use std::env::args;

fn main() {
    let target = args().nth(1).expect("Expected a target to match against");
    let pattern = format!("%{}%", target);

    let mut store = PostgresStore::from_env();
    let num_deleted = store
        .delete_account(&pattern)
        .expect("Error deleting account(s)");
    println!("Deleted {} accounts", num_deleted);
}
