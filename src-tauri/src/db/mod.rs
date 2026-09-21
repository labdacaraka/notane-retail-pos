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

    let migrations: &[(&str, &str)] = &[
        ("001_initial_schema", include_str!("../../migrations/001_initial_schema.sql")),
        ("002_seed_data", include_str!("../../migrations/002_seed_data.sql")),
        ("003_receipt_settings", include_str!("../../migrations/003_receipt_settings.sql")),
        ("004_pin_management_permission", include_str!("../../migrations/004_pin_management_permission.sql")),
        ("005_outlets_and_permissions", include_str!("../../migrations/005_outlets_and_permissions.sql")),
        ("006_decouple_warehouses_and_link_outlets", include_str!("../../migrations/006_decouple_warehouses_and_link_outlets.sql")),
        ("007_unify_warehouses_into_outlets", include_str!("../../migrations/007_unify_warehouses_into_outlets.sql")),
        ("008_user_outlets_access_control", include_str!("../../migrations/008_user_outlets_access_control.sql")),
        ("009_product_outlet_prices", include_str!("../../migrations/009_product_outlet_prices.sql")),
        ("010_impersonate_permission", include_str!("../../migrations/010_impersonate_permission.sql")),
    ];

    for (name, sql) in migrations {
        match sqlx::raw_sql(*sql).execute(pool).await {
            Ok(_) => info!("Migrasi {} berhasil dieksekusi.", name),
            Err(e) => tracing::warn!("Peringatan migrasi {}: {}", name, e),
        }
    }

    Ok(())
}
