use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use dotenvy::dotenv;
use std::env;

mod models;
mod schema;

use schema::transfer_internal::dsl;

type PgPool = Pool<ConnectionManager<PgConnection>>;
type PgConn = PooledConnection<ConnectionManager<PgConnection>>;

pub struct TransferStore {
    pool: PgPool,
}

impl TransferStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn from_env() -> Self {
        dotenv().ok();
        let database_url =
            env::var("TRANSFERS_DATABASE_URL").expect("TRANSFERS_DATABASE_URL must be set");
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create transfers connection pool");
        Self { pool }
    }

    fn conn(&self) -> Result<PgConn, crate::TransferError> {
        self.pool
            .get()
            .map_err(|e| crate::TransferError::Storage(e.to_string()))
    }

    pub(crate) fn insert_transfer(
        &self,
        client_id: &str,
        from_account_id: i32,
        to_account_id: i32,
        amount: i64,
    ) -> Result<crate::Transfer, crate::TransferError> {
        let mut conn = self.conn()?;
        diesel::insert_into(dsl::transfer_internal)
            .values(models::NewTransferInternal {
                client_id,
                from_account_id,
                to_account_id,
                amount,
            })
            .returning(models::TransferInternal::as_returning())
            .get_result(&mut *conn)
            .map(Into::into)
            .map_err(|e| crate::TransferError::Storage(e.to_string()))
    }

    pub(crate) fn find_transfer_by_client_id(
        &self,
        client_id: &str,
    ) -> Result<crate::Transfer, crate::TransferError> {
        use schema::transfer_internal::client_id as sql_client_id;
        let mut conn = self.conn()?;
        dsl::transfer_internal
            .filter(sql_client_id.eq(client_id))
            .select(models::TransferInternal::as_select())
            .first(&mut *conn)
            .map(Into::into)
            .map_err(|e| match e {
                diesel::result::Error::NotFound => crate::TransferError::TransferNotFound,
                e => crate::TransferError::Storage(e.to_string()),
            })
    }

    /// Atomically transition the transfer from `Pending` to `new_status`,
    /// returning the effective status after the attempt.
    ///
    /// - If the transfer was `Pending`, it is now `new_status` → returns `new_status`.
    /// - If it was already `new_status` (retry), returns `new_status` unchanged.
    /// - Any other current status is returned as-is; the caller decides the error.
    pub(crate) fn claim_pending(
        &self,
        id: i32,
        new_status: crate::TransferStatus,
    ) -> Result<crate::TransferStatus, crate::TransferError> {
        let mut conn = self.conn()?;
        let rows = diesel::update(
            dsl::transfer_internal
                .find(id)
                .filter(dsl::status.eq(models::TransferStatus::Pending)),
        )
        .set((
            dsl::status.eq(models::TransferStatus::from(new_status)),
            dsl::updated_at.eq(std::time::SystemTime::now()),
        ))
        .execute(&mut *conn)
        .map_err(|e| crate::TransferError::Storage(e.to_string()))?;

        if rows == 1 {
            return Ok(new_status);
        }

        // Row was not Pending — fetch the current status so the caller can
        // distinguish "already in the target state (retry)" from "conflict".
        dsl::transfer_internal
            .find(id)
            .select(dsl::status)
            .first::<models::TransferStatus>(&mut *conn)
            .map(Into::into)
            .map_err(|e| match e {
                diesel::result::Error::NotFound => crate::TransferError::TransferNotFound,
                e => crate::TransferError::Storage(e.to_string()),
            })
    }

    pub(crate) fn set_transfer_status(
        &self,
        id: i32,
        status: crate::TransferStatus,
    ) -> Result<(), crate::TransferError> {
        let mut conn = self.conn()?;
        diesel::update(dsl::transfer_internal.find(id))
            .set((
                dsl::status.eq(models::TransferStatus::from(status)),
                dsl::updated_at.eq(std::time::SystemTime::now()),
            ))
            .execute(&mut *conn)
            .map(|_| ())
            .map_err(|e| crate::TransferError::Storage(e.to_string()))
    }
}
