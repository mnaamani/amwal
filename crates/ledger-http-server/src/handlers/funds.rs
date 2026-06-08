use actix_web::{web, HttpResponse};
use ledger_api::{LedgerClient, LedgerClientError};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::error::LedgerApiError;

#[derive(Debug, Deserialize, ToSchema)]
pub struct BlockFundsRequest {
    pub client_id: String,
    pub account_id: i64,
    pub amount: i64,
}

fn internal_err() -> LedgerApiError {
    LedgerApiError(LedgerClientError::Unavailable("internal error".into()))
}

/// Reserve funds on an account for the duration of an in-flight transfer.
#[utoipa::path(
    post,
    path = "/funds/blocks",
    request_body = BlockFundsRequest,
    responses(
        (status = 204, description = "Funds blocked"),
        (status = 422, description = "Insufficient funds", body = ledger_api::LedgerClientError),
        (status = 503, description = "Storage unavailable", body = ledger_api::LedgerClientError),
    ),
    tag = "funds"
)]
pub async fn block_funds(
    data: web::Data<dyn LedgerClient>,
    body: web::Json<BlockFundsRequest>,
) -> Result<HttpResponse, LedgerApiError> {
    let body = body.into_inner();
    web::block(move || data.block_funds(&body.client_id, body.account_id, body.amount))
        .await
        .map_err(|_| internal_err())??;
    Ok(HttpResponse::NoContent().finish())
}

/// Release a previously placed fund block.
#[utoipa::path(
    delete,
    path = "/funds/blocks/{client_id}",
    params(("client_id" = String, Path, description = "The client_id used when blocking funds")),
    responses(
        (status = 204, description = "Funds released"),
        (status = 503, description = "Storage unavailable", body = ledger_api::LedgerClientError),
    ),
    tag = "funds"
)]
pub async fn release_funds(
    data: web::Data<dyn LedgerClient>,
    path: web::Path<String>,
) -> Result<HttpResponse, LedgerApiError> {
    let client_id = path.into_inner();
    web::block(move || data.release_funds(&client_id))
        .await
        .map_err(|_| internal_err())??;
    Ok(HttpResponse::NoContent().finish())
}
