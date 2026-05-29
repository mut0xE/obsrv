use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::errors::ApiError;

pub mod queries;
pub async fn connect(database_url: &str) -> Result<PgPool, ApiError> {
    PgPoolOptions::new()
        .max_connections(20)
        .connect(database_url)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB connect failed: {}", e)))
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), ApiError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("migration failed: {}", e)))
}
