pub mod api_doc;
pub mod error;
pub mod handlers;

use actix_web::web;

/// Wire all business routes onto `cfg`. Used by both `main.rs` and integration
/// tests so the routing is defined exactly once.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
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
    );
}
