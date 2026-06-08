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
        from_account_id: i64,
        to_account_id: i64,
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
        let mut conn = self.conn()?;
        dsl::transfer_internal
            .filter(dsl::client_id.eq(client_id))
            .select(models::TransferInternal::as_select())
            .first(&mut *conn)
            .map(Into::into)
            .map_err(|e| match e {
                diesel::result::Error::NotFound => crate::TransferError::TransferNotFound,
                e => crate::TransferError::Storage(e.to_string()),
            })
    }

    pub(crate) fn find_transfers_by_status(
        &self,
        status: crate::TransferStatus,
    ) -> Result<Vec<crate::Transfer>, crate::TransferError> {
        let mut conn = self.conn()?;
        dsl::transfer_internal
            .filter(dsl::status.eq(models::TransferStatus::from(status)))
            .select(models::TransferInternal::as_select())
            .load(&mut *conn)
            .map(|v| v.into_iter().map(Into::into).collect())
            .map_err(|e| crate::TransferError::Storage(e.to_string()))
    }

    /// Atomically transition the transfer from `Pending` to `new_status`,
    /// returning the effective status after the attempt.
    pub(crate) fn claim_pending(
        &self,
        id: i64,
        new_status: crate::TransferStatus,
    ) -> Result<crate::TransferStatus, crate::TransferError> {
        let mut conn = self.conn()?;
        let rows = diesel::update(
            dsl::transfer_internal
                .find(id)
                .filter(dsl::status.eq(models::TransferStatus::Pending)),
        )
        .set(dsl::status.eq(models::TransferStatus::from(new_status)))
        .execute(&mut *conn)
        .map_err(|e| crate::TransferError::Storage(e.to_string()))?;

        if rows == 1 {
            return Ok(new_status);
        }

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
        id: i64,
        status: crate::TransferStatus,
    ) -> Result<(), crate::TransferError> {
        let mut conn = self.conn()?;
        diesel::update(dsl::transfer_internal.find(id))
            .set(dsl::status.eq(models::TransferStatus::from(status)))
            .execute(&mut *conn)
            .map(|_| ())
            .map_err(|e| crate::TransferError::Storage(e.to_string()))
    }
}
