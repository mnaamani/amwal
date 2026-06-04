use actix_web::{web, HttpResponse};
use ledger_api::{AccountType, LedgerClient, LedgerClientError};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::error::LedgerApiError;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAccountRequest {
    pub client_id: String,
    pub name: String,
    pub account_type: AccountType,
}

fn internal_err() -> LedgerApiError {
    LedgerApiError(LedgerClientError::Unavailable("internal error".into()))
}

/// Create a new inactive account.
#[utoipa::path(
    post,
    path = "/accounts",
    request_body = CreateAccountRequest,
    responses(
        (status = 200, description = "Account created", body = ledger_api::AccountSummary),
        (status = 400, description = "Invalid request", body = ledger_api::LedgerClientError),
        (status = 503, description = "Storage unavailable", body = ledger_api::LedgerClientError),
    ),
    tag = "accounts"
)]
pub async fn create_account(
    data: web::Data<dyn LedgerClient>,
    body: web::Json<CreateAccountRequest>,
) -> Result<HttpResponse, LedgerApiError> {
    let body = body.into_inner();
    let result =
        web::block(move || data.create_account(&body.client_id, &body.name, body.account_type))
            .await
            .map_err(|_| internal_err())??;
    Ok(HttpResponse::Ok().json(result))
}

/// Activate an account so it can receive journal postings.
#[utoipa::path(
    post,
    path = "/accounts/{id}/activate",
    params(("id" = i32, Path, description = "Account ID")),
    responses(
        (status = 200, description = "Account activated", body = ledger_api::AccountSummary),
        (status = 404, description = "Account not found", body = ledger_api::LedgerClientError),
        (status = 422, description = "Validation error", body = ledger_api::LedgerClientError),
    ),
    tag = "accounts"
)]
pub async fn activate_account(
    data: web::Data<dyn LedgerClient>,
    path: web::Path<i32>,
) -> Result<HttpResponse, LedgerApiError> {
    let id = path.into_inner();
    let result = web::block(move || data.activate_account(id))
        .await
        .map_err(|_| internal_err())??;
    Ok(HttpResponse::Ok().json(result))
}

/// Fetch a single account by ID.
#[utoipa::path(
    get,
    path = "/accounts/{id}",
    params(("id" = i32, Path, description = "Account ID")),
    responses(
        (status = 200, description = "Account found", body = ledger_api::AccountSummary),
        (status = 404, description = "Account not found"),
        (status = 503, description = "Storage unavailable", body = ledger_api::LedgerClientError),
    ),
    tag = "accounts"
)]
pub async fn get_account(
    data: web::Data<dyn LedgerClient>,
    path: web::Path<i32>,
) -> Result<HttpResponse, LedgerApiError> {
    let id = path.into_inner();
    let result = web::block(move || data.get_account(id))
        .await
        .map_err(|_| internal_err())?
        .map_err(LedgerApiError::from)?;
    match result {
        Some(acc) => Ok(HttpResponse::Ok().json(acc)),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

/// List all active accounts.
#[utoipa::path(
    get,
    path = "/accounts",
    responses(
        (status = 200, description = "List of active accounts", body = Vec<ledger_api::AccountSummary>),
        (status = 503, description = "Storage unavailable", body = ledger_api::LedgerClientError),
    ),
    tag = "accounts"
)]
pub async fn list_active_accounts(
    data: web::Data<dyn LedgerClient>,
) -> Result<HttpResponse, LedgerApiError> {
    let result = web::block(move || data.list_active_accounts())
        .await
        .map_err(|_| internal_err())??;
    Ok(HttpResponse::Ok().json(result))
}
