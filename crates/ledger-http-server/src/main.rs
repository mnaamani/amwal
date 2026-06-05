use std::sync::Arc;
use std::time::Duration;

use actix_web::{web, App, HttpServer};
use domain_events::EventBus;
use ledger::{LedgerService, OutboxRelay, PostgresLedgerStore};
use ledger_api::LedgerClient;
use ledger_http_server::{api_doc::ApiDoc, configure};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

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
    println!("Starting ledger-http-server on {bind_addr}");

    HttpServer::new(move || {
        let openapi = ApiDoc::openapi();
        App::new()
            .app_data(data.clone())
            .configure(configure)
            .service(SwaggerUi::new("/api-docs/{_:.*}").url("/api-docs/openapi.json", openapi))
    })
    .bind(&bind_addr)?
    .run()
    .await
}
