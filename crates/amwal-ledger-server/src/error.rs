use actix_web::HttpResponse;
use actix_web::ResponseError;
use ledger_api::LedgerClientError;

/// Newtype wrapper so we can impl `ResponseError` (orphan rule).
#[derive(Debug)]
pub struct LedgerApiError(pub LedgerClientError);

impl std::fmt::Display for LedgerApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl ResponseError for LedgerApiError {
    fn error_response(&self) -> HttpResponse {
        use actix_web::http::StatusCode;
        use LedgerClientError::*;

        let status = match &self.0 {
            AccountNotFound(_) => StatusCode::NOT_FOUND,
            AccountNotActive(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ImbalancedEntry { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            InsufficientFunds { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            InvalidRequest(_) => StatusCode::BAD_REQUEST,
            AccountsIncompatible => StatusCode::UNPROCESSABLE_ENTITY,
            Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        };
        HttpResponse::build(status).json(&self.0)
    }
}

impl From<LedgerClientError> for LedgerApiError {
    fn from(e: LedgerClientError) -> Self {
        LedgerApiError(e)
    }
}
