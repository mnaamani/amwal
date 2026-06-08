use actix_web::{web, HttpResponse};
use ledger_api::{JournalLeg, LedgerClient, LedgerClientError};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::error::LedgerApiError;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PostJournalEntryRequest {
    pub client_id: String,
    pub legs: Vec<JournalLeg>,
}

fn internal_err() -> LedgerApiError {
    LedgerApiError(LedgerClientError::Unavailable("internal error".into()))
}

/// Post a balanced double-entry journal entry.
#[utoipa::path(
    post,
    path = "/journal/entries",
    request_body = PostJournalEntryRequest,
    responses(
        (status = 200, description = "Journal entry ID", body = i64),
        (status = 400, description = "Invalid request", body = ledger_api::LedgerClientError),
        (status = 422, description = "Validation error", body = ledger_api::LedgerClientError),
        (status = 503, description = "Storage unavailable", body = ledger_api::LedgerClientError),
    ),
    tag = "journal"
)]
pub async fn post_journal_entry(
    data: web::Data<dyn LedgerClient>,
    body: web::Json<PostJournalEntryRequest>,
) -> Result<HttpResponse, LedgerApiError> {
    let body = body.into_inner();
    let result = web::block(move || data.post_journal_entry(&body.client_id, body.legs))
        .await
        .map_err(|_| internal_err())??;
    Ok(HttpResponse::Ok().json(result))
}
