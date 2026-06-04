use std::sync::Arc;
use std::time::Duration;

use actix_web::{web, App, HttpServer};
use domain_events::EventBus;
use ledger::{LedgerService, OutboxRelay, PostgresLedgerStore};
use ledger_api::LedgerClient;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod api_doc;
mod error;
mod handlers;

use api_doc::ApiDoc;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    let store = PostgresLedgerStore::from_env();
    let service = LedgerService::new(store.clone());

    let bus = Arc::new(EventBus::new());
    // The JoinHandle is intentionally dropped — the relay runs for the process lifetime.
    let _ = OutboxRelay::new(store, bus, Duration::from_secs(2)).spawn();

    let ledger: Arc<dyn LedgerClient> = Arc::new(service);
    let data = web::Data::from(ledger);

    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    println!("Starting amwal-ledger-server on {bind_addr}");

    HttpServer::new(move || {
        let openapi = ApiDoc::openapi();
        App::new()
            .app_data(data.clone())
            .service(
                web::scope("/accounts")
                    .route("", web::post().to(handlers::accounts::create_account))
                    .route("", web::get().to(handlers::accounts::list_active_accounts))
                    .route("/{id}", web::get().to(handlers::accounts::get_account))
                    .route(
                        "/{id}/activate",
                        web::post().to(handlers::accounts::activate_account),
                    )
                    .route(
                        "/{id}/balance",
                        web::get().to(handlers::balances::get_account_balance),
                    )
                    .route(
                        "/{id}/available-balance",
                        web::get().to(handlers::balances::get_available_balance),
                    )
                    .route(
                        "/{id}/balance-history",
                        web::get().to(handlers::balances::get_balance_history),
                    ),
            )
            .service(
                web::scope("/journal")
                    .route(
                        "/entries",
                        web::post().to(handlers::journal::post_journal_entry),
                    )
                    .route(
                        "/transfers",
                        web::post().to(handlers::transfers::post_transfer),
                    ),
            )
            .service(
                web::scope("/funds")
                    .route("/blocks", web::post().to(handlers::funds::block_funds))
                    .route(
                        "/blocks/{client_id}",
                        web::delete().to(handlers::funds::release_funds),
                    ),
            )
            .service(SwaggerUi::new("/api-docs/{_:.*}").url("/api-docs/openapi.json", openapi))
    })
    .bind(&bind_addr)?
    .run()
    .await
}
