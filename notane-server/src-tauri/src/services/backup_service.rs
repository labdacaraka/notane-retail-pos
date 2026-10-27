use std::io::{Read, Write};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

pub const APP_NAME: &str = "notane-retail-pos";
pub const CURRENT_SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CURRENT_SCHEMA_VERSION: i32 = 63;
pub const MIN_COMPATIBLE_SERVER_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableBackupSummary {
    pub table_name: String,
    pub record_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub app_name: String,
    pub server_version: String,
    pub min_server_version: String,
    pub schema_version: i32,
    pub exported_at: DateTime<Utc>,
    pub database_name: String,
    pub total_tables: usize,
    pub total_records: usize,
    pub sql_file: String,
    pub checksum_sha256: String,
    pub os: String,
    pub arch: String,
    pub notes: Option<String>,
    pub tables_summary: Vec<TableBackupSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityCheckResult {
    pub is_compatible: bool,
    pub current_server_version: String,
    pub current_schema_version: i32,
    pub required_min_version: String,
    pub backup_server_version: String,
    pub backup_schema_version: i32,
    pub error_message: Option<String>,
    pub warning_message: Option<String>,
    pub manifest: BackupManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreSummary {
    pub success: bool,
    pub message: String,
    pub total_tables: usize,
    pub total_records: usize,
    pub restored_at: DateTime<Utc>,
    pub manifest: BackupManifest,
}

pub fn check_compatibility(manifest: &BackupManifest) -> CompatibilityCheckResult {
    let current_version_str = CURRENT_SERVER_VERSION;
    let current_schema = CURRENT_SCHEMA_VERSION;

    if manifest.app_name != APP_NAME {
        return CompatibilityCheckResult {
            is_compatible: false,
            current_server_version: current_version_str.to_string(),
            current_schema_version: current_schema,
            required_min_version: manifest.min_server_version.clone(),
            backup_server_version: manifest.server_version.clone(),
            backup_schema_version: manifest.schema_version,
            error_message: Some(format!(
                "Berkas cadangan tidak valid: Dibuat untuk aplikasi '{}', bukan '{}'",
                manifest.app_name, APP_NAME
            )),
            warning_message: None,
            manifest: manifest.clone(),
        };
    }

    let parsed_current = semver::Version::parse(current_version_str).unwrap_or_else(|_| semver::Version::new(1, 0, 0));
    let parsed_min = semver::Version::parse(&manifest.min_server_version)
        .unwrap_or_else(|_| semver::Version::new(1, 0, 0));

    if parsed_current < parsed_min {
        return CompatibilityCheckResult {
            is_compatible: false,
            current_server_version: current_version_str.to_string(),
            current_schema_version: current_schema,
            required_min_version: manifest.min_server_version.clone(),
            backup_server_version: manifest.server_version.clone(),
            backup_schema_version: manifest.schema_version,
            error_message: Some(format!(
                "Versi Notane Server saat ini (v{}) lebih rendah dari batas minimum yang dibutuhkan berkas cadangan (v{}). Harap perbarui aplikasi Notane Server terlebih dahulu sebelum memulihkan data.",
                current_version_str, manifest.min_server_version
            )),
            warning_message: None,
            manifest: manifest.clone(),
        };
    }

    if manifest.schema_version > current_schema {
        return CompatibilityCheckResult {
            is_compatible: false,
            current_server_version: current_version_str.to_string(),
            current_schema_version: current_schema,
            required_min_version: manifest.min_server_version.clone(),
            backup_server_version: manifest.server_version.clone(),
            backup_schema_version: manifest.schema_version,
            error_message: Some(format!(
                "Skema database berkas cadangan (v{}) lebih baru daripada skema yang didukung server ini (v{}). Perbarui aplikasi Notane Server agar kompatibel dengan data ini.",
                manifest.schema_version, current_schema
            )),
            warning_message: None,
            manifest: manifest.clone(),
        };
    }

    let mut warning = None;
    if manifest.schema_version < current_schema {
        warning = Some(format!(
            "Berkas cadangan dibuat dari skema lebih lama (v{} vs v{}). Database akan dimigrasikan secara otomatis ke skema terbaru setelah dipulihkan.",
            manifest.schema_version, current_schema
        ));
    }

    CompatibilityCheckResult {
        is_compatible: true,
        current_server_version: current_version_str.to_string(),
        current_schema_version: current_schema,
        required_min_version: manifest.min_server_version.clone(),
        backup_server_version: manifest.server_version.clone(),
        backup_schema_version: manifest.schema_version,
        error_message: None,
        warning_message: warning,
        manifest: manifest.clone(),
    }
}

pub async fn export_database_to_zip(
    pool: &PgPool,
    notes: Option<String>,
) -> Result<(Vec<u8>, BackupManifest), String> {
    let db_name: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| "notane_pos".to_string());

    // 1. Ambil daftar tabel public selain tabel internal migrasi
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name 
         FROM information_schema.tables 
         WHERE table_schema = 'public' 
           AND table_type = 'BASE TABLE'
           AND table_name NOT IN ('_migrations', 'spatial_ref_sys')
         ORDER BY table_name ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Gagal membaca daftar tabel database: {}", e))?;

    let now = Utc::now();
    let mut sql_buf = String::with_capacity(1024 * 1024);

    sql_buf.push_str(&format!(
        "-- ============================================================================\n\
         -- NOTANE RETAIL SERVER - DATABASE BACKUP DUMP\n\
         -- App Name     : {}\n\
         -- Server Ver   : {}\n\
         -- Schema Ver   : {}\n\
         -- Exported At  : {}\n\
         -- Database     : {}\n\
         -- Total Tables : {}\n\
         -- ============================================================================\n\n\
         SET statement_timeout = 0;\n\
         SET lock_timeout = 0;\n\
         SET client_encoding = 'UTF8';\n\
         SET standard_conforming_strings = on;\n\
         SET check_function_bodies = false;\n\
         SET client_min_messages = warning;\n\
         SET row_security = off;\n\n\
         -- Nonaktifkan pengecekan foreign key & triggers selama pemulihan berlangsung\n\
         SET session_replication_role = 'replica';\n\n",
        APP_NAME,
        CURRENT_SERVER_VERSION,
        CURRENT_SCHEMA_VERSION,
        now.to_rfc3339(),
        db_name,
        tables.len()
    ));

    let mut total_records = 0usize;
    let mut tables_summary = Vec::with_capacity(tables.len());

    for table_name in &tables {
        // Ambil kolom-kolom tabel terurut
        let cols: Vec<String> = sqlx::query_scalar(
            "SELECT column_name 
             FROM information_schema.columns 
             WHERE table_schema = 'public' AND table_name = $1 
             ORDER BY ordinal_position ASC",
        )
        .bind(table_name)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal membaca kolom tabel '{}': {}", table_name, e))?;

        if cols.is_empty() {
            continue;
        }

        let quoted_cols: Vec<String> = cols.iter().map(|c| format!("\"{}\"", c)).collect();
        let cols_joined = quoted_cols.join(", ");

        // Hitung total baris
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM \"{}\"", table_name))
            .fetch_one(pool)
            .await
            .unwrap_or(0);

        let row_count = count as usize;
        total_records += row_count;
        tables_summary.push(TableBackupSummary {
            table_name: table_name.clone(),
            record_count: row_count,
        });

        sql_buf.push_str(&format!(
            "-- ------------------------------------------------------------\n\
             -- Data untuk tabel: \"{}\" ({} baris)\n\
             -- ------------------------------------------------------------\n\
             TRUNCATE TABLE \"{}\" CASCADE;\n",
            table_name, row_count, table_name
        ));

        if row_count > 0 {
            // Bentuk ekspresi ekspresi quote_nullable kolom PostgreSQL
            let val_exprs: Vec<String> = cols
                .iter()
                .map(|c| format!("quote_nullable(\"{}\"::text)", c))
                .collect();
            let select_tuple = format!("'(' || {} || ')'", val_exprs.join(" || ', ' || "));

            let query = format!("SELECT {} FROM \"{}\"", select_tuple, table_name);
            let rows: Vec<String> = sqlx::query_scalar(&query)
                .fetch_all(pool)
                .await
                .map_err(|e| format!("Gagal mengekstrak data dari tabel '{}': {}", table_name, e))?;

            // Tulis INSERT INTO dalam chunks (200 baris per statement)
            for chunk in rows.chunks(200) {
                sql_buf.push_str(&format!(
                    "INSERT INTO \"{}\" ({}) VALUES\n{};\n",
                    table_name,
                    cols_joined,
                    chunk.join(",\n")
                ));
            }
        }

        // Sinkronisasi sequence jika tabel memiliki kolom serial ID
        let has_id_col = cols.iter().any(|c| c == "id");
        if has_id_col {
            sql_buf.push_str(&format!(
                "DO $$\n\
                 DECLARE\n\
                     seq_name TEXT;\n\
                     max_id BIGINT;\n\
                 BEGIN\n\
                     seq_name := pg_get_serial_sequence('\"{}\"', 'id');\n\
                     IF seq_name IS NOT NULL THEN\n\
                         SELECT COALESCE(MAX(id), 1) INTO max_id FROM \"{}\";\n\
                         PERFORM setval(seq_name, max_id, true);\n\
                     END IF;\n\
                 END $$;\n\n",
                table_name, table_name
            ));
        } else {
            sql_buf.push('\n');
        }
    }

    sql_buf.push_str(
        "-- ============================================================================\n\
         -- Kembalikan constraints dan triggers ke mode normal\n\
         -- ============================================================================\n\
         SET session_replication_role = 'DEFAULT';\n"
    );

    // 2. Hitung Checksum SHA-256 dari sql_buf
    let mut hasher = Sha256::new();
    hasher.update(sql_buf.as_bytes());
    let checksum_sha256 = hex::encode(hasher.finalize());

    // 3. Bangun Manifest Cadangan
    let manifest = BackupManifest {
        app_name: APP_NAME.to_string(),
        server_version: CURRENT_SERVER_VERSION.to_string(),
        min_server_version: MIN_COMPATIBLE_SERVER_VERSION.to_string(),
        schema_version: CURRENT_SCHEMA_VERSION,
        exported_at: now,
        database_name: db_name,
        total_tables: tables_summary.len(),
        total_records,
        sql_file: "backup.sql".to_string(),
        checksum_sha256,
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        notes,
        tables_summary,
    };

    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Gagal memformat manifest json: {}", e))?;

    // 4. Kompresi berkas ke format ZIP
    let mut zip_bytes = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_bytes));
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);

        // A. Tambahkan manifest.json
        zip.start_file("manifest.json", options)
            .map_err(|e| format!("Gagal menambahkan manifest.json ke zip: {}", e))?;
        zip.write_all(manifest_json.as_bytes())
            .map_err(|e| format!("Gagal menulis manifest.json ke zip: {}", e))?;

        // B. Tambahkan backup.sql
        zip.start_file("backup.sql", options)
            .map_err(|e| format!("Gagal menambahkan backup.sql ke zip: {}", e))?;
        zip.write_all(sql_buf.as_bytes())
            .map_err(|e| format!("Gagal menulis backup.sql ke zip: {}", e))?;

        zip.finish()
            .map_err(|e| format!("Gagal menyelesaikan pembuatan berkas zip: {}", e))?;
    }

    Ok((zip_bytes, manifest))
}

pub fn inspect_backup_zip(zip_bytes: &[u8]) -> Result<CompatibilityCheckResult, String> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| format!("Format berkas tidak valid atau bukan file zip: {}", e))?;

    let mut manifest_file = archive
        .by_name("manifest.json")
        .map_err(|_| "Berkas cadangan tidak valid: File 'manifest.json' tidak ditemukan di dalam paket ZIP".to_string())?;

    let mut manifest_content = String::new();
    manifest_file
        .read_to_string(&mut manifest_content)
        .map_err(|e| format!("Gagal membaca manifest.json dari zip: {}", e))?;

    let manifest: BackupManifest = serde_json::from_str(&manifest_content)
        .map_err(|e| format!("Format manifest.json korup atau tidak valid: {}", e))?;

    Ok(check_compatibility(&manifest))
}

pub async fn import_database_from_zip(
    pool: PgPool,
    zip_bytes: Vec<u8>,
    user_id: Option<Uuid>,
) -> Result<RestoreSummary, String> {
    // 1. Ekstrak manifest dan berkas SQL di scope tersendiri agar ZipArchive di-drop sebelum await
    let (manifest, sql_content) = {
        let cursor = std::io::Cursor::new(&zip_bytes);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| format!("Format berkas tidak valid atau bukan file zip: {}", e))?;

        let manifest: BackupManifest = {
            let mut manifest_file = archive
                .by_name("manifest.json")
                .map_err(|_| "Berkas cadangan tidak valid: 'manifest.json' tidak ditemukan di dalam arsip ZIP".to_string())?;
            let mut manifest_content = String::new();
            manifest_file
                .read_to_string(&mut manifest_content)
                .map_err(|e| format!("Gagal membaca 'manifest.json': {}", e))?;
            serde_json::from_str(&manifest_content)
                .map_err(|e| format!("Format 'manifest.json' tidak valid: {}", e))?
        };

        let sql_content = {
            let mut sql_file = archive
                .by_name(&manifest.sql_file)
                .map_err(|_| format!("Berkas SQL '{}' tidak ditemukan di dalam arsip ZIP", manifest.sql_file))?;
            let mut content = String::new();
            sql_file
                .read_to_string(&mut content)
                .map_err(|e| format!("Gagal membaca berkas SQL '{}': {}", manifest.sql_file, e))?;
            content
        };

        (manifest, sql_content)
    };

    let compat = check_compatibility(&manifest);
    if !compat.is_compatible {
        return Err(compat.error_message.unwrap_or_else(|| {
            "Berkas cadangan tidak kompatibel dengan spesifikasi Notane Server ini".to_string()
        }));
    }

    // 2. Verifikasi Checksum SHA-256
    let mut hasher = Sha256::new();
    hasher.update(sql_content.as_bytes());
    let computed_checksum = hex::encode(hasher.finalize());

    if computed_checksum.to_lowercase() != manifest.checksum_sha256.to_lowercase() {
        return Err("Verifikasi integritas data gagal: Checksum SHA-256 tidak cocok. Berkas cadangan terindikasi rusak atau dimodifikasi.".to_string());
    }

    // 3. Eksekusi pemulihan SQL
    sqlx::raw_sql(&sql_content)
        .execute(&pool)
        .await
        .map_err(|e| format!("Gagal mengeksekusi instruksi SQL restorasi: {}", e))?;

    // 5. Catat log audit sistem pemulihan database
    let _ = sqlx::query(
        "INSERT INTO audit_logs (user_id, terminal_id, action, entity_name, entity_id, new_values)
         VALUES ($1, 'HOST-SERVER', 'DATABASE_RESTORE', 'database', $2, $3)"
    )
    .bind(user_id)
    .bind(&manifest.database_name)
    .bind(serde_json::to_value(&manifest).unwrap_or_default())
    .execute(&pool)
    .await;

    Ok(RestoreSummary {
        success: true,
        message: format!(
            "Database berhasil dipulihkan dari cadangan '{}' ({} tabel, {} data baris).",
            manifest.exported_at.format("%Y-%m-%d %H:%M:%S UTC"),
            manifest.total_tables,
            manifest.total_records
        ),
        total_tables: manifest.total_tables,
        total_records: manifest.total_records,
        restored_at: Utc::now(),
        manifest,
    })
}
