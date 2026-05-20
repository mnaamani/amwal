use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use dotenvy::dotenv;
use std::env;

mod models;
mod schema;

type PgPool = Pool<ConnectionManager<PgConnection>>;
type PgConn = PooledConnection<ConnectionManager<PgConnection>>;

/// Owns the transfers service's DB connection pool.
/// Reads TRANSFERS_DATABASE_URL
pub struct TransferStore {
    pool: PgPool,
}

impl TransferStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn from_env() -> Self {
        dotenv().ok();
        let database_url = env::var("TRANSFERS_DATABASE_URL")
            .expect("TRANSFERS_DATABASE_URL must be set");
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create transfers connection pool");
        Self { pool }
    }

    pub(crate) fn conn(&self) -> Result<PgConn, String> {
        self.pool.get().map_err(|e| e.to_string())
    }
}
