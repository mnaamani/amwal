use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use actix_web::{test, web, App};
use ledger_api::{
    AccountId, AccountSummary, AccountType, BalanceSnapshot, JournalEntryId, JournalLeg,
    LedgerClient, LedgerClientError,
};
use ledger_http_server::configure;
use serde_json::{json, Value};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn stub_account(id: i32, active: bool) -> AccountSummary {
    AccountSummary {
        id,
        active,
        name: format!("Account {id}"),
        account_type: AccountType::Asset,
    }
}

macro_rules! app {
    ($client:expr) => {{
        let data: web::Data<dyn LedgerClient> = web::Data::from($client as Arc<dyn LedgerClient>);
        test::init_service(App::new().app_data(data).configure(configure)).await
    }};
}

// Consumes `resp`, returning (status_code, json_body).
async fn unpack(resp: actix_web::dev::ServiceResponse) -> (u16, Value) {
    let status = resp.status().as_u16();
    let body: Value = test::read_body_json(resp).await;
    (status, body)
}

// ── OkClient — all methods succeed with fixed sensible data ───────────────────

struct OkClient;

impl LedgerClient for OkClient {
    fn create_account(
        &self,
        _client_id: &str,
        name: &str,
        account_type: AccountType,
    ) -> Result<AccountSummary, LedgerClientError> {
        Ok(AccountSummary {
            id: 1,
            active: false,
            name: name.into(),
            account_type,
        })
    }
    fn activate_account(&self, id: AccountId) -> Result<AccountSummary, LedgerClientError> {
        Ok(stub_account(id, true))
    }
    fn get_account(&self, id: AccountId) -> Result<Option<AccountSummary>, LedgerClientError> {
        Ok(Some(stub_account(id, true)))
    }
    fn list_active_accounts(&self) -> Result<Vec<AccountSummary>, LedgerClientError> {
        Ok(vec![stub_account(1, true), stub_account(2, true)])
    }
    fn get_account_balance(&self, _id: AccountId) -> Result<i64, LedgerClientError> {
        Ok(10_000)
    }
    fn get_available_balance(&self, _id: AccountId) -> Result<i64, LedgerClientError> {
        Ok(8_000)
    }
    fn get_balance_history(
        &self,
        _account_id: AccountId,
    ) -> Result<Vec<BalanceSnapshot>, LedgerClientError> {
        Ok(vec![BalanceSnapshot {
            timestamp: UNIX_EPOCH + Duration::from_millis(1_700_000_000_000),
            balance: 10_000,
        }])
    }
    fn post_journal_entry(
        &self,
        _client_id: &str,
        _legs: Vec<JournalLeg>,
    ) -> Result<JournalEntryId, LedgerClientError> {
        Ok(42)
    }
    fn block_funds(
        &self,
        _client_id: &str,
        _account_id: AccountId,
        _amount: i64,
    ) -> Result<(), LedgerClientError> {
        Ok(())
    }
    fn release_funds(&self, _block_client_id: &str) -> Result<(), LedgerClientError> {
        Ok(())
    }
    fn post_transfer(
        &self,
        _client_id: &str,
        _from_account_id: AccountId,
        _to_account_id: AccountId,
        _amount: i64,
    ) -> Result<(), LedgerClientError> {
        Ok(())
    }
}

// ── ErrClient — all methods return the same configurable error ────────────────

struct ErrClient(fn() -> LedgerClientError);

impl LedgerClient for ErrClient {
    fn create_account(
        &self,
        _client_id: &str,
        _name: &str,
        _account_type: AccountType,
    ) -> Result<AccountSummary, LedgerClientError> {
        Err((self.0)())
    }
    fn activate_account(&self, _id: AccountId) -> Result<AccountSummary, LedgerClientError> {
        Err((self.0)())
    }
    fn get_account(&self, _id: AccountId) -> Result<Option<AccountSummary>, LedgerClientError> {
        Err((self.0)())
    }
    fn list_active_accounts(&self) -> Result<Vec<AccountSummary>, LedgerClientError> {
        Err((self.0)())
    }
    fn get_account_balance(&self, _id: AccountId) -> Result<i64, LedgerClientError> {
        Err((self.0)())
    }
    fn get_available_balance(&self, _id: AccountId) -> Result<i64, LedgerClientError> {
        Err((self.0)())
    }
    fn get_balance_history(
        &self,
        _account_id: AccountId,
    ) -> Result<Vec<BalanceSnapshot>, LedgerClientError> {
        Err((self.0)())
    }
    fn post_journal_entry(
        &self,
        _client_id: &str,
        _legs: Vec<JournalLeg>,
    ) -> Result<JournalEntryId, LedgerClientError> {
        Err((self.0)())
    }
    fn block_funds(
        &self,
        _client_id: &str,
        _account_id: AccountId,
        _amount: i64,
    ) -> Result<(), LedgerClientError> {
        Err((self.0)())
    }
    fn release_funds(&self, _block_client_id: &str) -> Result<(), LedgerClientError> {
        Err((self.0)())
    }
    fn post_transfer(
        &self,
        _client_id: &str,
        _from_account_id: AccountId,
        _to_account_id: AccountId,
        _amount: i64,
    ) -> Result<(), LedgerClientError> {
        Err((self.0)())
    }
}

// ── NoneClient — get_account returns Ok(None) ─────────────────────────────────

struct NoneClient;

impl LedgerClient for NoneClient {
    fn get_account(&self, _id: AccountId) -> Result<Option<AccountSummary>, LedgerClientError> {
        Ok(None)
    }
    fn create_account(
        &self,
        _client_id: &str,
        _name: &str,
        _account_type: AccountType,
    ) -> Result<AccountSummary, LedgerClientError> {
        unimplemented!()
    }
    fn activate_account(&self, _id: AccountId) -> Result<AccountSummary, LedgerClientError> {
        unimplemented!()
    }
    fn list_active_accounts(&self) -> Result<Vec<AccountSummary>, LedgerClientError> {
        unimplemented!()
    }
    fn get_account_balance(&self, _id: AccountId) -> Result<i64, LedgerClientError> {
        unimplemented!()
    }
    fn get_available_balance(&self, _id: AccountId) -> Result<i64, LedgerClientError> {
        unimplemented!()
    }
    fn get_balance_history(
        &self,
        _account_id: AccountId,
    ) -> Result<Vec<BalanceSnapshot>, LedgerClientError> {
        unimplemented!()
    }
    fn post_journal_entry(
        &self,
        _client_id: &str,
        _legs: Vec<JournalLeg>,
    ) -> Result<JournalEntryId, LedgerClientError> {
        unimplemented!()
    }
    fn block_funds(
        &self,
        _client_id: &str,
        _account_id: AccountId,
        _amount: i64,
    ) -> Result<(), LedgerClientError> {
        unimplemented!()
    }
    fn release_funds(&self, _block_client_id: &str) -> Result<(), LedgerClientError> {
        unimplemented!()
    }
    fn post_transfer(
        &self,
        _client_id: &str,
        _from_account_id: AccountId,
        _to_account_id: AccountId,
        _amount: i64,
    ) -> Result<(), LedgerClientError> {
        unimplemented!()
    }
}

// ── Accounts ──────────────────────────────────────────────────────────────────

#[actix_web::test]
async fn create_account_returns_200_with_summary() {
    let app = app!(Arc::new(OkClient));
    let req = test::TestRequest::post()
        .uri("/accounts")
        .set_json(json!({"client_id": "c-1", "name": "Checking", "account_type": "Asset"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (status, body) = unpack(resp).await;
    assert_eq!(status, 200);
    assert_eq!(body["name"], "Checking");
    assert_eq!(body["active"], false);
    assert_eq!(body["account_type"], "Asset");
}

#[actix_web::test]
async fn create_account_bad_json_returns_400() {
    let app = app!(Arc::new(OkClient));
    // `name` and `account_type` are missing
    let req = test::TestRequest::post()
        .uri("/accounts")
        .set_payload(r#"{"client_id": "c-1"}"#)
        .insert_header(("content-type", "application/json"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 400);
}

#[actix_web::test]
async fn list_accounts_returns_200_with_array() {
    let app = app!(Arc::new(OkClient));
    let req = test::TestRequest::get().uri("/accounts").to_request();
    let resp = test::call_service(&app, req).await;
    let (status, body) = unpack(resp).await;
    assert_eq!(status, 200);
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[actix_web::test]
async fn get_account_returns_200_with_correct_id() {
    let app = app!(Arc::new(OkClient));
    let req = test::TestRequest::get().uri("/accounts/7").to_request();
    let resp = test::call_service(&app, req).await;
    let (status, body) = unpack(resp).await;
    assert_eq!(status, 200);
    assert_eq!(body["id"], 7);
    assert_eq!(body["active"], true);
}

#[actix_web::test]
async fn get_account_not_found_returns_404() {
    let app = app!(Arc::new(NoneClient));
    let req = test::TestRequest::get().uri("/accounts/999").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
}

#[actix_web::test]
async fn activate_account_returns_200_with_active_flag() {
    let app = app!(Arc::new(OkClient));
    let req = test::TestRequest::post()
        .uri("/accounts/3/activate")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (status, body) = unpack(resp).await;
    assert_eq!(status, 200);
    assert_eq!(body["id"], 3);
    assert_eq!(body["active"], true);
}

// ── Balances ──────────────────────────────────────────────────────────────────

#[actix_web::test]
async fn get_balance_returns_200_with_amount() {
    let app = app!(Arc::new(OkClient));
    let req = test::TestRequest::get()
        .uri("/accounts/1/balance")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (status, body) = unpack(resp).await;
    assert_eq!(status, 200);
    assert_eq!(body, json!(10_000));
}

#[actix_web::test]
async fn get_available_balance_returns_200_with_amount() {
    let app = app!(Arc::new(OkClient));
    let req = test::TestRequest::get()
        .uri("/accounts/1/available-balance")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (status, body) = unpack(resp).await;
    assert_eq!(status, 200);
    assert_eq!(body, json!(8_000));
}

#[actix_web::test]
async fn get_balance_history_returns_200_with_snapshots() {
    let app = app!(Arc::new(OkClient));
    let req = test::TestRequest::get()
        .uri("/accounts/1/balance-history")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (status, body) = unpack(resp).await;
    assert_eq!(status, 200);
    assert!(body.is_array());
    assert_eq!(body[0]["balance"], 10_000);
}

// ── Journal entries ───────────────────────────────────────────────────────────

#[actix_web::test]
async fn post_journal_entry_returns_200_with_id() {
    let app = app!(Arc::new(OkClient));
    let req = test::TestRequest::post()
        .uri("/journal/entries")
        .set_json(json!({
            "client_id": "je-1",
            "legs": [
                {"account_id": 1, "posting": {"direction": "Debit",  "amount": 5000}},
                {"account_id": 2, "posting": {"direction": "Credit", "amount": 5000}}
            ]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (status, body) = unpack(resp).await;
    assert_eq!(status, 200);
    assert_eq!(body, json!(42));
}

#[actix_web::test]
async fn post_journal_entry_missing_legs_returns_400() {
    let app = app!(Arc::new(OkClient));
    let req = test::TestRequest::post()
        .uri("/journal/entries")
        .set_payload(r#"{"client_id": "je-1"}"#)
        .insert_header(("content-type", "application/json"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 400);
}

#[actix_web::test]
async fn post_journal_entry_imbalanced_returns_422() {
    let app = app!(Arc::new(ErrClient(|| LedgerClientError::ImbalancedEntry {
        total_debits: 5000,
        total_credits: 3000,
    })));
    let req = test::TestRequest::post()
        .uri("/journal/entries")
        .set_json(json!({
            "client_id": "je-2",
            "legs": [
                {"account_id": 1, "posting": {"direction": "Debit",  "amount": 5000}},
                {"account_id": 2, "posting": {"direction": "Credit", "amount": 3000}}
            ]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 422);
}

#[actix_web::test]
async fn post_journal_entry_inactive_account_returns_422() {
    let app = app!(Arc::new(ErrClient(|| LedgerClientError::AccountNotActive(
        1
    ))));
    let req = test::TestRequest::post()
        .uri("/journal/entries")
        .set_json(json!({
            "client_id": "je-3",
            "legs": [
                {"account_id": 1, "posting": {"direction": "Debit",  "amount": 100}},
                {"account_id": 2, "posting": {"direction": "Credit", "amount": 100}}
            ]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 422);
}

// ── Transfers ─────────────────────────────────────────────────────────────────

#[actix_web::test]
async fn post_transfer_returns_204() {
    let app = app!(Arc::new(OkClient));
    let req = test::TestRequest::post()
        .uri("/journal/transfers")
        .set_json(json!({
            "client_id": "tx-1",
            "from_account_id": 1,
            "to_account_id": 2,
            "amount": 1000
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 204);
}

#[actix_web::test]
async fn post_transfer_incompatible_accounts_returns_422() {
    let app = app!(Arc::new(ErrClient(|| {
        LedgerClientError::AccountsIncompatible
    })));
    let req = test::TestRequest::post()
        .uri("/journal/transfers")
        .set_json(json!({
            "client_id": "tx-2",
            "from_account_id": 1,
            "to_account_id": 3,
            "amount": 1000
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 422);
}

#[actix_web::test]
async fn post_transfer_account_not_found_returns_404() {
    let app = app!(Arc::new(ErrClient(|| LedgerClientError::AccountNotFound(
        99
    ))));
    let req = test::TestRequest::post()
        .uri("/journal/transfers")
        .set_json(json!({
            "client_id": "tx-3",
            "from_account_id": 99,
            "to_account_id": 2,
            "amount": 500
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
}

// ── Fund blocks ───────────────────────────────────────────────────────────────

#[actix_web::test]
async fn block_funds_returns_204() {
    let app = app!(Arc::new(OkClient));
    let req = test::TestRequest::post()
        .uri("/funds/blocks")
        .set_json(json!({"client_id": "blk-1", "account_id": 1, "amount": 500}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 204);
}

#[actix_web::test]
async fn block_funds_insufficient_returns_422() {
    let app = app!(Arc::new(ErrClient(|| {
        LedgerClientError::InsufficientFunds {
            available: 200,
            requested: 500,
        }
    })));
    let req = test::TestRequest::post()
        .uri("/funds/blocks")
        .set_json(json!({"client_id": "blk-2", "account_id": 1, "amount": 500}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 422);
}

#[actix_web::test]
async fn release_funds_returns_204() {
    let app = app!(Arc::new(OkClient));
    let req = test::TestRequest::delete()
        .uri("/funds/blocks/blk-1")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 204);
}

// ── Error → HTTP status mapping (exhaustive) ──────────────────────────────────

#[actix_web::test]
async fn error_account_not_found_maps_to_404() {
    let app = app!(Arc::new(ErrClient(|| LedgerClientError::AccountNotFound(
        1
    ))));
    let req = test::TestRequest::get().uri("/accounts/1").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
}

#[actix_web::test]
async fn error_account_not_active_maps_to_422() {
    let app = app!(Arc::new(ErrClient(|| LedgerClientError::AccountNotActive(
        1
    ))));
    let req = test::TestRequest::post()
        .uri("/accounts/1/activate")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 422);
}

#[actix_web::test]
async fn error_invalid_request_maps_to_400() {
    let app = app!(Arc::new(ErrClient(|| {
        LedgerClientError::InvalidRequest("name cannot be blank".into())
    })));
    let req = test::TestRequest::post()
        .uri("/accounts")
        .set_json(json!({"client_id": "c-1", "name": "", "account_type": "Asset"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 400);
}

#[actix_web::test]
async fn error_unavailable_maps_to_503() {
    let app = app!(Arc::new(ErrClient(|| {
        LedgerClientError::Unavailable("db timeout".into())
    })));
    let req = test::TestRequest::get().uri("/accounts").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 503);
}

#[actix_web::test]
async fn error_body_contains_tagged_error_field() {
    let app = app!(Arc::new(ErrClient(|| LedgerClientError::AccountNotFound(
        5
    ))));
    let req = test::TestRequest::get().uri("/accounts/5").to_request();
    let resp = test::call_service(&app, req).await;
    let (status, body) = unpack(resp).await;
    assert_eq!(status, 404);
    assert_eq!(body["error"], "AccountNotFound");
    assert_eq!(body["detail"], 5);
}
