use utoipa::OpenApi;

use crate::handlers::{
    accounts::CreateAccountRequest, funds::BlockFundsRequest, journal::PostJournalEntryRequest,
    transfers::PostTransferRequest,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::accounts::create_account,
        crate::handlers::accounts::activate_account,
        crate::handlers::accounts::get_account,
        crate::handlers::accounts::list_active_accounts,
        crate::handlers::balances::get_account_balance,
        crate::handlers::balances::get_available_balance,
        crate::handlers::balances::get_balance_history,
        crate::handlers::journal::post_journal_entry,
        crate::handlers::transfers::post_transfer,
        crate::handlers::funds::block_funds,
        crate::handlers::funds::release_funds,
    ),
    components(schemas(
        ledger_api::AccountSummary,
        ledger_api::AccountType,
        ledger_api::BalanceSnapshot,
        ledger_api::JournalLeg,
        ledger_api::JournalPosting,
        ledger_api::LedgerClientError,
        CreateAccountRequest,
        PostJournalEntryRequest,
        PostTransferRequest,
        BlockFundsRequest,
    )),
    tags(
        (name = "accounts", description = "Account management"),
        (name = "balances", description = "Balance queries"),
        (name = "journal", description = "Journal entries and transfers"),
        (name = "funds", description = "Fund blocking and release"),
    ),
    info(
        title = "Amwal Ledger API",
        version = "0.1.0",
        description = "Double-entry ledger REST API"
    )
)]
pub struct ApiDoc;
