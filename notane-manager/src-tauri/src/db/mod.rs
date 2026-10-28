use crate::crypto::ed25519::{DEFAULT_MASTER_PRIVATE_KEY_HEX, DEFAULT_MASTER_PUBLIC_KEY_HEX};
use sqlx::any::AnyPoolOptions;
use std::fs;
use std::path::PathBuf;
use tracing::info;

pub type DbPool = sqlx::AnyPool;

pub async fn init_db(db_path: Option<PathBuf>) -> Result<DbPool, sqlx::Error> {
    sqlx::any::install_default_drivers();

    let resolve_sqlite = |custom_path: Option<PathBuf>| -> String {
        if let Ok(env_db) = std::env::var("DATABASE_PATH") {
            let trimmed = env_db.trim();
            if !trimmed.is_empty() {
                let p = PathBuf::from(trimmed);
                if let Some(parent) = p.parent() {
                    fs::create_dir_all(parent).ok();
                }
                return format!("sqlite://{}?mode=rwc", p.to_str().unwrap_or(trimmed));
            }
        }
        match custom_path {
            Some(p) => {
                if let Some(parent) = p.parent() {
                    fs::create_dir_all(parent).ok();
                }
                format!("sqlite://{}?mode=rwc", p.to_str().unwrap_or("notane_provider.db"))
            }
            None => {
                let mut dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
                dir.push("notane-license-manager");
                fs::create_dir_all(&dir).ok();
                dir.push("notane_provider.db");
                format!("sqlite://{}?mode=rwc", dir.to_str().unwrap_or("notane_provider.db"))
            }
        }
    };

    let database_url = if let Ok(env_mgr_url) = std::env::var("MANAGER_DATABASE_URL") {
        let trimmed = env_mgr_url.trim();
        if !trimmed.is_empty() {
            trimmed.to_string()
        } else {
            resolve_sqlite(db_path)
        }
    } else if let Ok(env_url) = std::env::var("DATABASE_URL") {
        let trimmed = env_url.trim();
        // Cegah tabrakan fatal dengan database POS notane-server (notane_pos / port 15432)
        if !trimmed.is_empty()
            && !trimmed.ends_with("/notane_pos")
            && !trimmed.contains("15432")
            && (trimmed.contains("manager") || trimmed.contains("provider"))
        {
            trimmed.to_string()
        } else {
            resolve_sqlite(db_path)
        }
    } else {
        resolve_sqlite(db_path)
    };

    let is_pg = database_url.starts_with("postgres://") || database_url.starts_with("postgresql://");
    if is_pg {
        info!("Inisialisasi Database PostgreSQL Provider...");
    } else {
        info!("Inisialisasi Database SQLite Provider di: {}", database_url);
    }

    let pool = AnyPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    run_migrations(&pool).await?;
    seed_rbac_and_defaults(&pool).await?;

    Ok(pool)
}

async fn run_migrations(pool: &DbPool) -> Result<(), sqlx::Error> {
    // 1. RBAC Tables
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS roles (
            id TEXT PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL
        );"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS permissions (
            id TEXT PRIMARY KEY,
            slug TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            module TEXT NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL
        );"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS role_permissions (
            role_id TEXT NOT NULL,
            permission_id TEXT NOT NULL,
            PRIMARY KEY (role_id, permission_id),
            FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
            FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE
        );"
    ).execute(pool).await?;

    // 2. Provider Tables
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS provider_clients (
            id TEXT PRIMARY KEY,
            client_name TEXT NOT NULL,
            company_name TEXT NOT NULL,
            email TEXT UNIQUE NOT NULL,
            phone TEXT,
            address TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS provider_licenses (
            id TEXT PRIMARY KEY,
            license_key TEXT UNIQUE NOT NULL,
            client_id TEXT NOT NULL,
            tier TEXT NOT NULL DEFAULT 'ENTERPRISE',
            max_terminals INTEGER NOT NULL DEFAULT 5,
            status TEXT NOT NULL DEFAULT 'ACTIVE',
            issued_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            grace_period_days INTEGER NOT NULL DEFAULT 7,
            features TEXT NOT NULL,
            notes TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (client_id) REFERENCES provider_clients(id) ON DELETE CASCADE
        );"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS provider_device_activations (
            id TEXT PRIMARY KEY,
            license_id TEXT NOT NULL,
            hardware_id TEXT NOT NULL,
            device_name TEXT,
            ip_address TEXT,
            os_info TEXT,
            app_version TEXT,
            activated_at TEXT NOT NULL,
            last_ping_at TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            UNIQUE(license_id, hardware_id),
            FOREIGN KEY (license_id) REFERENCES provider_licenses(id) ON DELETE CASCADE
        );"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS provider_app_releases (
            id TEXT PRIMARY KEY,
            version TEXT UNIQUE NOT NULL,
            release_name TEXT NOT NULL,
            changelog TEXT NOT NULL,
            release_date TEXT NOT NULL,
            download_url TEXT NOT NULL,
            checksum_sha256 TEXT NOT NULL,
            signature_ed25519 TEXT NOT NULL,
            is_mandatory INTEGER NOT NULL DEFAULT 0,
            min_license_tier TEXT NOT NULL DEFAULT 'BASIC',
            has_db_migration INTEGER NOT NULL DEFAULT 1,
            is_published INTEGER NOT NULL DEFAULT 1,
            component TEXT NOT NULL DEFAULT 'server',
            min_server_version TEXT,
            created_at TEXT NOT NULL
        );"
    ).execute(pool).await?;

    // Migration kolom jika tabel sudah ada sebelumnya
    let _ = sqlx::query("ALTER TABLE provider_app_releases ADD COLUMN component TEXT NOT NULL DEFAULT 'server';")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE provider_app_releases ADD COLUMN min_server_version TEXT;")
        .execute(pool)
        .await;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS provider_license_audit_logs (
            id TEXT PRIMARY KEY,
            license_key TEXT,
            hardware_id TEXT,
            action TEXT NOT NULL,
            ip_address TEXT,
            status TEXT NOT NULL,
            details TEXT,
            created_at TEXT NOT NULL
        );"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS provider_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS provider_users (
            id TEXT PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            full_name TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            quick_pin TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'super_admin',
            last_login_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );"
    ).execute(pool).await?;

    Ok(())
}

async fn seed_rbac_and_defaults(pool: &DbPool) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();

    // Default Administrator Seed
    let count_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM provider_users")
        .fetch_one(pool)
        .await
        .unwrap_or((0,));

    if count_users.0 == 0 {
        let pass_hash = crate::crypto::hash::hash_secret("admin");
        let pin_hash = crate::crypto::hash::hash_secret("123456");
        let _ = sqlx::query(
            "INSERT INTO provider_users (
                id, username, full_name, password_hash, quick_pin, role, created_at, updated_at
            ) VALUES ('usr_master_admin', 'admin', 'Master Administrator', $1, $2, 'super_admin', $3, $4)
            ON CONFLICT DO NOTHING"
        )
        .bind(pass_hash)
        .bind(pin_hash)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await;
    }

    // Roles
    sqlx::query(
        "INSERT INTO roles (id, name, description, created_at)
         VALUES ('role_super_admin', 'super_admin', 'Pemilik Utama / Provider License Administrator', $1)
         ON CONFLICT DO NOTHING;"
    )
    .bind(&now)
    .execute(pool)
    .await?;

    // Permissions (Uniform naming convention: <module>:<action>)
    let permissions = [
        ("client:read", "Lihat Data Klien", "client", "Melihat daftar dan detail data klien tenant"),
        ("client:create", "Tambah Klien", "client", "Menambahkan klien baru"),
        ("client:update", "Edit Klien", "client", "Memperbarui informasi klien"),
        ("client:delete", "Hapus Klien", "client", "Menghapus akun klien"),
        ("license:read", "Lihat Lisensi", "license", "Melihat daftar lisensi terbit"),
        ("license:create", "Terbitkan Lisensi", "license", "Membuat dan menandatangani lisensi baru"),
        ("license:update", "Ubah Status Lisensi", "license", "Memperpanjang atau mengubah status lisensi"),
        ("license:revoke", "Cabut Lisensi", "license", "Mencabut/revoke lisensi"),
        ("device:read", "Lihat Perangkat", "device", "Melihat terminal HWID aktif"),
        ("device:unbind", "Reset Perangkat HWID", "device", "Melepas tautan hardware ID mesin klien"),
        ("release:read", "Lihat Rilis OTA", "release", "Melihat riwayat pembaruan aplikasi"),
        ("release:create", "Terbitkan Rilis Baru", "release", "Membuat dan menandatangani rilis update baru"),
        ("release:delete", "Hapus Rilis", "release", "Menghapus versi rilis"),
        ("server:control", "Kontrol Server API", "server", "Menyalakan atau menghentikan server API provider"),
        ("audit:read", "Lihat Audit Log", "audit", "Melihat log aktivitas lisensi dan keamanan"),
    ];

    for (slug, name, module, desc) in permissions {
        let perm_id = format!("perm_{}", slug.replace(':', "_"));
        sqlx::query(
            "INSERT INTO permissions (id, slug, name, module, description, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT DO NOTHING;"
        )
        .bind(&perm_id)
        .bind(slug)
        .bind(name)
        .bind(module)
        .bind(desc)
        .bind(&now)
        .execute(pool)
        .await?;

        // Map ke super_admin role
        sqlx::query(
            "INSERT INTO role_permissions (role_id, permission_id)
             VALUES ('role_super_admin', $1)
             ON CONFLICT DO NOTHING;"
        )
        .bind(&perm_id)
        .execute(pool)
        .await?;
    }

    // Default Settings
    sqlx::query(
        "INSERT INTO provider_settings (key, value) VALUES ('server_port', '19090') ON CONFLICT DO NOTHING;"
    ).execute(pool).await?;

    sqlx::query(
        "INSERT INTO provider_settings (key, value) VALUES ('auto_start_server', 'true') ON CONFLICT DO NOTHING;"
    ).execute(pool).await?;

    sqlx::query(
        "INSERT INTO provider_settings (key, value) VALUES ('master_private_key', $1) ON CONFLICT DO NOTHING;"
    )
    .bind(DEFAULT_MASTER_PRIVATE_KEY_HEX)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO provider_settings (key, value) VALUES ('master_public_key', $1) ON CONFLICT DO NOTHING;"
    )
    .bind(DEFAULT_MASTER_PUBLIC_KEY_HEX)
    .execute(pool)
    .await?;

    // Seed contoh rilis update jika belum ada
    let count_releases: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM provider_app_releases")
        .fetch_one(pool)
        .await?;

    if count_releases.0 == 0 {
        let rel_id = uuid::Uuid::new_v4().to_string();
        let sig = crate::crypto::ed25519::sign_message(
            DEFAULT_MASTER_PRIVATE_KEY_HEX,
            "1.2.0:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        ).unwrap_or_default();

        sqlx::query(
            "INSERT INTO provider_app_releases (
                id, version, release_name, changelog, release_date, download_url,
                checksum_sha256, signature_ed25519, is_mandatory, min_license_tier,
                has_db_migration, is_published, component, min_server_version, created_at
            ) VALUES ($1, '1.2.0', 'Pembaruan Notane Server v1.2.0',
                      '1. Peningkatan kecepatan simpan transaksi kasir\n2. Pembaruan struktur data laporan penjualan\n3. Peningkatan kestabilan sambungan jaringan toko',
                      '2026-10-01', 'https://update.notane.id/releases/v1.2.0/notane-server-bundle.zip',
                      'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
                      $2, 0, 'BASIC', 1, 1, 'server', NULL, $3)
            ON CONFLICT DO NOTHING;"
        )
        .bind(rel_id)
        .bind(sig)
        .bind(&now)
        .execute(pool)
        .await?;

        let client_rel_id = uuid::Uuid::new_v4().to_string();
        let client_sig = crate::crypto::ed25519::sign_message(
            DEFAULT_MASTER_PRIVATE_KEY_HEX,
            "1.2.0:f4c0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b866"
        ).unwrap_or_default();

        sqlx::query(
            "INSERT INTO provider_app_releases (
                id, version, release_name, changelog, release_date, download_url,
                checksum_sha256, signature_ed25519, is_mandatory, min_license_tier,
                has_db_migration, is_published, component, min_server_version, created_at
            ) VALUES ($1, '1.2.0', 'Pembaruan Aplikasi Kasir v1.2.0',
                      '1. Tampilan antarmuka kasir lebih cepat dan responsif\n2. Peningkatan cetak struk penjualan\n3. Perbaikan tampilan pencarian barang',
                      '2026-10-01', 'https://update.notane.id/releases/v1.2.0/notane-client-bundle.zip',
                      'f4c0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b866',
                      $2, 0, 'BASIC', 0, 1, 'client', '1.1.0', $3)
            ON CONFLICT DO NOTHING;"
        )
        .bind(client_rel_id)
        .bind(client_sig)
        .bind(&now)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub fn adapt_sql(query: &str) -> String {
    let mut out = String::with_capacity(query.len() + 16);
    let mut param_idx = 1;
    for c in query.chars() {
        if c == '?' {
            out.push('$');
            out.push_str(&param_idx.to_string());
            param_idx += 1;
        } else {
            out.push(c);
        }
    }
    if out.contains("INSERT OR IGNORE INTO") {
        out = out.replace("INSERT OR IGNORE INTO", "INSERT INTO");
        let trimmed = out.trim_end().trim_end_matches(';');
        out = format!("{} ON CONFLICT DO NOTHING;", trimmed);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapt_sql() {
        let q1 = adapt_sql("SELECT * FROM t WHERE a = ? AND b = ?");
        assert_eq!(q1, "SELECT * FROM t WHERE a = $1 AND b = $2");
        let q2 = adapt_sql("INSERT OR IGNORE INTO roles (id, name) VALUES (?, ?);");
        assert_eq!(q2, "INSERT INTO roles (id, name) VALUES ($1, $2) ON CONFLICT DO NOTHING;");
    }

    #[tokio::test]
    async fn test_dollar_param_on_sqlite() {
        sqlx::any::install_default_drivers();
        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:?cache=shared")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE test (id TEXT, val TEXT);")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO test (id, val) VALUES ($1, $2);")
            .bind("1")
            .bind("hello")
            .execute(&pool)
            .await
            .unwrap();
        let row: (String,) = sqlx::query_as("SELECT val FROM test WHERE id = $1;")
            .bind("1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.0, "hello");

        // Test LIKE and LIMIT OFFSET with $1, $2, $3
        let rows: Vec<(String,)> = sqlx::query_as("SELECT val FROM test WHERE val LIKE $1 LIMIT $2 OFFSET $3;")
            .bind("%ell%")
            .bind(10i64)
            .bind(0i64)
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "hello");

        // Test adapt_sql dynamic temporary binding
        sqlx::query(&adapt_sql("INSERT INTO test (id, val) VALUES (?, ?);"))
            .bind("2")
            .bind("world")
            .execute(&pool)
            .await
            .unwrap();
        let row2: (String,) = sqlx::query_as(&adapt_sql("SELECT val FROM test WHERE id = ?;"))
            .bind("2")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row2.0, "world");
    }
}


