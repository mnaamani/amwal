use actix_web::{web, HttpResponse};
use ledger_api::{LedgerClient, LedgerClientError};

use crate::error::LedgerApiError;

fn internal_err() -> LedgerApiError {
    LedgerApiError(LedgerClientError::Unavailable("internal error".into()))
}

/// Get the posted balance for an account.
#[utoipa::path(
    get,
    path = "/accounts/{id}/balance",
    params(("id" = i32, Path, description = "Account ID")),
    responses(
        (status = 200, description = "Posted balance in fils", body = i64),
        (status = 404, description = "Account not found", body = ledger_api::LedgerClientError),
        (status = 503, description = "Storage unavailable", body = ledger_api::LedgerClientError),
    ),
    tag = "balances"
)]
pub async fn get_account_balance(
    data: web::Data<dyn LedgerClient>,
    path: web::Path<i32>,
) -> Result<HttpResponse, LedgerApiError> {
    let id = path.into_inner();
    let result = web::block(move || data.get_account_balance(id))
        .await
        .map_err(|_| internal_err())??;
    Ok(HttpResponse::Ok().json(result))
}

/// Get the available balance (posted minus blocked) for an account.
#[utoipa::path(
    get,
    path = "/accounts/{id}/available-balance",
    params(("id" = i32, Path, description = "Account ID")),
    responses(
        (status = 200, description = "Available balance in fils", body = i64),
        (status = 404, description = "Account not found", body = ledger_api::LedgerClientError),
        (status = 503, description = "Storage unavailable", body = ledger_api::LedgerClientError),
    ),
    tag = "balances"
)]
pub async fn get_available_balance(
    data: web::Data<dyn LedgerClient>,
    path: web::Path<i32>,
) -> Result<HttpResponse, LedgerApiError> {
    let id = path.into_inner();
    let result = web::block(move || data.get_available_balance(id))
        .await
        .map_err(|_| internal_err())??;
    Ok(HttpResponse::Ok().json(result))
}

/// Get the full balance history for an account (one entry per journal posting).
#[utoipa::path(
    get,
    path = "/accounts/{id}/balance-history",
    params(("id" = i32, Path, description = "Account ID")),
    responses(
        (status = 200, description = "Balance history", body = Vec<ledger_api::BalanceSnapshot>),
        (status = 503, description = "Storage unavailable", body = ledger_api::LedgerClientError),
    ),
    tag = "balances"
)]
pub async fn get_balance_history(
    data: web::Data<dyn LedgerClient>,
    path: web::Path<i32>,
) -> Result<HttpResponse, LedgerApiError> {
    let id = path.into_inner();
    let result = web::block(move || data.get_balance_history(id))
        .await
        .map_err(|_| internal_err())??;
    Ok(HttpResponse::Ok().json(result))
}
