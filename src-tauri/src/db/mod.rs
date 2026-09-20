use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use tracing::info;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    info!("Menghubungkan ke PostgreSQL: {}", database_url);
    PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(10))
        .connect(database_url)
        .await
}

pub async fn run_embedded_migrations(pool: &PgPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Menjalankan migrasi database terintegrasi...");

    let schema_sql = include_str!("../../migrations/001_initial_schema.sql");
    sqlx::raw_sql(schema_sql).execute(pool).await?;
    info!("Migrasi skema 001_initial_schema berhasil dieksekusi.");

    let seed_sql = include_str!("../../migrations/002_seed_data.sql");
    sqlx::raw_sql(seed_sql).execute(pool).await?;
    info!("Migrasi seed data 002_seed_data berhasil dieksekusi.");

    let receipt_sql = include_str!("../../migrations/003_receipt_settings.sql");
    sqlx::raw_sql(receipt_sql).execute(pool).await?;
    info!("Migrasi 003_receipt_settings berhasil dieksekusi.");

    Ok(())
}
