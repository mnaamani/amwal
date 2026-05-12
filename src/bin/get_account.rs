use self::models::Account;
use diesel::prelude::*;
use ledger::*;
use std::env::args;

fn main() {
    use self::schema::accounts::dsl::accounts;

    let account_id = args()
        .nth(1)
        .expect("get_accounts requires an account id")
        .parse::<i32>()
        .expect("Invalid ID");

    let connection = &mut db_connect();

    let account = accounts
        .find(account_id)
        .select(Account::as_select())
        .first(connection)
        .optional(); // This allows for returning an Option<Post>, otherwise it will throw an error

    match account {
        Ok(Some(account)) => println!("Account with id: {} has name: {}", account.id, account.name),
        Ok(None) => println!("Unable to find account {}", account_id),
        Err(_) => println!("An error occured while fetching account {}", account_id),
    }
}
