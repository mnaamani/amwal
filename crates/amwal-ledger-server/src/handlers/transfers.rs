use actix_web::{web, HttpResponse};
use ledger_api::{LedgerClient, LedgerClientError};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::error::LedgerApiError;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PostTransferRequest {
    pub client_id: String,
    pub from_account_id: i32,
    pub to_account_id: i32,
    pub amount: i64,
}

fn internal_err() -> LedgerApiError {
    LedgerApiError(LedgerClientError::Unavailable("internal error".into()))
}

/// Post a direct transfer between two accounts of the same accounting nature.
#[utoipa::path(
    post,
    path = "/journal/transfers",
    request_body = PostTransferRequest,
    responses(
        (status = 204, description = "Transfer posted"),
        (status = 404, description = "Account not found", body = ledger_api::LedgerClientError),
        (status = 422, description = "Validation error", body = ledger_api::LedgerClientError),
        (status = 503, description = "Storage unavailable", body = ledger_api::LedgerClientError),
    ),
    tag = "journal"
)]
pub async fn post_transfer(
    data: web::Data<dyn LedgerClient>,
    body: web::Json<PostTransferRequest>,
) -> Result<HttpResponse, LedgerApiError> {
    let body = body.into_inner();
    web::block(move || {
        data.post_transfer(
            &body.client_id,
            body.from_account_id,
            body.to_account_id,
            body.amount,
        )
    })
    .await
    .map_err(|_| internal_err())??;
    Ok(HttpResponse::NoContent().finish())
}
