pub mod embedded;

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
    info!("Memeriksa dan menjalankan migrasi database terintegrasi...");

    // 1. Buat tabel `_migrations` jika belum ada (gaya Laravel)
    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id SERIAL PRIMARY KEY,
            migration VARCHAR(255) NOT NULL UNIQUE,
            batch INTEGER NOT NULL,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
        );"
    )
    .execute(pool)
    .await?;

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
        ("011_outlet_customer_group_prices", include_str!("../../migrations/011_outlet_customer_group_prices.sql")),
        ("012_license_system", include_str!("../../migrations/012_license_system.sql")),
        ("013_product_images_and_advanced_import", include_str!("../../migrations/013_product_images_and_advanced_import.sql")),
        ("014_fix_product_image_storage", include_str!("../../migrations/014_fix_product_image_storage.sql")),
        ("015_flash_sale_system", include_str!("../../migrations/015_flash_sale_system.sql")),
        ("016_fix_flash_sale_discount_types", include_str!("../../migrations/016_fix_flash_sale_discount_types.sql")),
        ("017_receipt_discount_tax_settings", include_str!("../../migrations/017_receipt_discount_tax_settings.sql")),
        ("018_sales_discount_accounts", include_str!("../../migrations/018_sales_discount_accounts.sql")),
        ("019_purchase_order_enhancements", include_str!("../../migrations/019_purchase_order_enhancements.sql")),
        ("020_outlet_coa_and_stock_transfer", include_str!("../../migrations/020_outlet_coa_and_stock_transfer.sql")),
        ("021_accounting_period_closing_and_balance_sheet", include_str!("../../migrations/021_accounting_period_closing_and_balance_sheet.sql")),
        ("022_goods_receipts_and_procurement_workflow", include_str!("../../migrations/022_goods_receipts_and_procurement_workflow.sql")),
        ("023_purchase_discount_and_tax_accounts", include_str!("../../migrations/023_purchase_discount_and_tax_accounts.sql")),
        ("024_purchase_returns_and_cancellations", include_str!("../../migrations/024_purchase_returns_and_cancellations.sql")),
        ("025_stock_transfer_workflow_and_in_transit", include_str!("../../migrations/025_stock_transfer_workflow_and_in_transit.sql")),
        ("026_remove_default_warehouse_from_outlets", include_str!("../../migrations/026_remove_default_warehouse_from_outlets.sql")),
        ("027_loyalty_point_settings_and_rules", include_str!("../../migrations/027_loyalty_point_settings_and_rules.sql")),
        ("028_loyalty_permissions", include_str!("../../migrations/028_loyalty_permissions.sql")),
        ("029_action_feature_permissions", include_str!("../../migrations/029_action_feature_permissions.sql")),
        ("030_customer_point_ledgers_indices", include_str!("../../migrations/030_customer_point_ledgers_indices.sql")),
        ("031_product_costing_methods_and_batches", include_str!("../../migrations/031_product_costing_methods_and_batches.sql")),
        ("032_inventory_costing_settings_and_global_fallback", include_str!("../../migrations/032_inventory_costing_settings_and_global_fallback.sql")),
        ("033_expense_transactions", include_str!("../../migrations/033_expense_transactions.sql")),
        ("034_capital_deposits", include_str!("../../migrations/034_capital_deposits.sql")),
        ("035_coa_system_lock", include_str!("../../migrations/035_coa_system_lock.sql")),
        ("036_lock_all_system_coa_and_bank_accounts", include_str!("../../migrations/036_lock_all_system_coa_and_bank_accounts.sql")),
        ("037_retail_features_p1_p2", include_str!("../../migrations/037_retail_features_p1_p2.sql")),
        ("038_store_branding_logo", include_str!("../../migrations/038_store_branding_logo.sql")),
        ("039_pairing_code_persistence", include_str!("../../migrations/039_pairing_code_persistence.sql")),
        ("040_provider_server_config", include_str!("../../migrations/040_provider_server_config.sql")),
        ("041_supervisor_biometric_settings", include_str!("../../migrations/041_supervisor_biometric_settings.sql")),
        ("042_all_ownership_permissions", include_str!("../../migrations/042_all_ownership_permissions.sql")),
        ("043_complete_granular_action_permissions", include_str!("../../migrations/043_complete_granular_action_permissions.sql")),
        ("044_user_current_outlet_personalization", include_str!("../../migrations/044_user_current_outlet_personalization.sql")),
        ("045_voucher_quota_and_claims", include_str!("../../migrations/045_voucher_quota_and_claims.sql")),
        ("046_audit_log_system_and_retention", include_str!("../../migrations/046_audit_log_system_and_retention.sql")),
        ("047_add_primary_supplier_to_products", include_str!("../../migrations/047_add_primary_supplier_to_products.sql")),
        ("048_sale_item_stock_batches_and_fefo_tracking", include_str!("../../migrations/048_sale_item_stock_batches_and_fefo_tracking.sql")),
        ("049_backfill_initial_stock_batches", include_str!("../../migrations/049_backfill_initial_stock_batches.sql")),
        ("050_grant_batch_tracking_roles", include_str!("../../migrations/050_grant_batch_tracking_roles.sql")),
        ("051_backfill_grn_batches_expiry_and_product_costing", include_str!("../../migrations/051_backfill_grn_batches_expiry_and_product_costing.sql")),
        ("052_rekening_antar_kantor_dan_jurnal_cabang", include_str!("../../migrations/052_rekening_antar_kantor_dan_jurnal_cabang.sql")),
        ("053_sync_missing_outlet_rak_accounts", include_str!("../../migrations/053_sync_missing_outlet_rak_accounts.sql")),
        ("054_dimensional_accounting_and_soft_hard_close", include_str!("../../migrations/054_dimensional_accounting_and_soft_hard_close.sql")),
        ("055_journal_entry_for_initial_stocks", include_str!("../../migrations/055_journal_entry_for_initial_stocks.sql")),
        ("056_initial_setup_wizard_settings", include_str!("../../migrations/056_initial_setup_wizard_settings.sql")),
        ("057_dividend_distributions", include_str!("../../migrations/057_dividend_distributions.sql")),
        ("058_dividend_linked_to_accounting_closings", include_str!("../../migrations/058_dividend_linked_to_accounting_closings.sql")),
        ("059_additional_expense_categories_and_equity_support", include_str!("../../migrations/059_additional_expense_categories_and_equity_support.sql")),
    ];

    // 2. Ambil daftar migrasi yang sudah pernah dieksekusi
    let executed_migrations: Vec<String> = sqlx::query_scalar::<_, String>(
        "SELECT migration FROM _migrations ORDER BY id ASC"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Jika tabel _migrations masih kosong, periksa apakah database sudah memiliki tabel skema awal (misal: tabel `products`)
    // Ini menangani database yang sudah berisi data agar skema dan seed data tidak dieksekusi ulang / ter-reset.
    if executed_migrations.is_empty() {
        let has_products_table: bool = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                SELECT FROM information_schema.tables 
                WHERE table_schema = 'public' AND table_name = 'products'
            )"
        )
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        if has_products_table {
            info!("Mendeteksi database yang sudah berisi skema. Menandai migrasi 001 s/d 011 sebagai terpasang (Batch 1)...");
            for (name, _) in migrations {
                let _ = sqlx::query("INSERT INTO _migrations (migration, batch) VALUES ($1, 1) ON CONFLICT DO NOTHING")
                    .bind(name)
                    .execute(pool)
                    .await;
            }
            info!("Tabel _migrations berhasil diinisialisasi untuk database yang sudah ada.");
            return Ok(());
        }
    }

    // 3. Hitung batch berikutnya
    let current_max_batch: Option<i32> = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(batch) FROM _migrations"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(None);

    let next_batch = current_max_batch.unwrap_or(0) + 1;
    let mut applied_count = 0;

    for (name, sql) in migrations {
        if executed_migrations.contains(&name.to_string()) {
            continue;
        }

        info!("Menjalankan migrasi baru: {} (Batch {})...", name, next_batch);
        match sqlx::raw_sql(*sql).execute(pool).await {
            Ok(_) => {
                if let Err(e) = sqlx::query(
                    "INSERT INTO _migrations (migration, batch) VALUES ($1, $2)"
                )
                .bind(name)
                .bind(next_batch)
                .execute(pool)
                .await {
                    tracing::error!("Gagal mencatat migrasi {} ke tabel _migrations: {}", name, e);
                } else {
                    info!("Migrasi {} berhasil dieksekusi dan dicatat (Batch {}).", name, next_batch);
                    applied_count += 1;
                }
            }
            Err(e) => {
                tracing::error!("Gagal mengeksekusi migrasi {}: {}", name, e);
                return Err(Box::new(e));
            }
        }
    }

    if applied_count == 0 {
        info!("Database skema sudah mutakhir (tidak ada migrasi baru yang perlu dijalankan).");
    } else {
        info!("Selesai mengeksekusi {} migrasi baru.", applied_count);
    }

    Ok(())
}
