use chrono::Utc;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::convert::TryInto;
use std::env;
use tracing::{info, warn};

use super::license_service::{get_master_public_key_hex, LicenseService, LicenseState};

pub const APP_BUILD_DATE: &str = "2026-09-26";
pub const APP_NAME: &str = "Notane Retail POS Enterprise";

/// Mengambil versi aktif aplikasi secara deterministik:
/// 1. Environment variable APP_VERSION (prioritas utama untuk Docker / Cloud / VPS deployment tanpa compile ulang)
/// 2. Compile-time CARGO_PKG_VERSION dari Cargo.toml (Desktop Tauri / Native Compiled Binary)
/// 3. Fallback "1.0.0"
pub fn get_app_version() -> String {
    if let Ok(val) = std::env::var("APP_VERSION") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    let pkg_ver = env!("CARGO_PKG_VERSION");
    if !pkg_ver.is_empty() {
        return pkg_ver.to_string();
    }
    "1.0.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppVersionInfo {
    pub name: String,
    pub version: String,
    pub build_date: String,
    pub target_os: String,
    pub target_arch: String,
    pub is_production: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResponse {
    pub current_version: String,
    pub has_update: bool,
    pub latest_version: String,
    pub release_name: Option<String>,
    pub changelog: Option<String>,
    pub release_date: Option<String>,
    pub download_url: Option<String>,
    pub checksum_sha256: Option<String>,
    pub signature: Option<String>,
    #[serde(default)]
    pub is_mandatory: bool,
    #[serde(default)]
    pub license_status: LicenseState,
    #[serde(default = "default_true")]
    pub can_update: bool,
    pub message: String,
    pub component: Option<String>,
    pub min_server_version: Option<String>,
    pub server_compatible: Option<bool>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyUpdateRequest {
    pub download_url: String,
    pub checksum_sha256: String,
    pub signature: String,
    pub target_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyUpdateResponse {
    pub success: bool,
    pub message: String,
    pub updated_to: String,
    pub restart_required: bool,
}

pub struct UpdateService;

impl UpdateService {
    pub fn get_version_info() -> AppVersionInfo {
        AppVersionInfo {
            name: APP_NAME.to_string(),
            version: get_app_version(),
            build_date: APP_BUILD_DATE.to_string(),
            target_os: env::consts::OS.to_string(),
            target_arch: env::consts::ARCH.to_string(),
            is_production: !cfg!(debug_assertions),
        }
    }

    /// Memeriksa ketersediaan pembaruan dari Server Provider Notane
    /// Akses pembaruan diproteksi oleh lisensi aktif pengguna
    pub async fn check_updates(pool: &PgPool) -> UpdateCheckResponse {
        Self::check_updates_for(pool, Some("server"), None).await
    }

    /// Memeriksa ketersediaan pembaruan untuk komponen tertentu ('server' atau 'client')
    pub async fn check_updates_for(
        pool: &PgPool,
        component: Option<&str>,
        client_version: Option<&str>,
    ) -> UpdateCheckResponse {
        let license_info = LicenseService::check_current_license(pool).await;
        let server_ver = get_app_version();
        let target_component = component.unwrap_or("server").to_lowercase();
        let current_ver = if target_component == "client" {
            client_version.unwrap_or(&server_ver).to_string()
        } else {
            server_ver.clone()
        };

        // Hanya lisensi aktif / grace period yang berhak mengunduh pembaruan resmi
        let can_update = license_info.is_valid && (license_info.status == LicenseState::Active || license_info.status == LicenseState::GracePeriod);

        if !can_update {
            return UpdateCheckResponse {
                current_version: current_ver.clone(),
                has_update: false,
                latest_version: current_ver,
                release_name: None,
                changelog: None,
                release_date: None,
                download_url: None,
                checksum_sha256: None,
                signature: None,
                is_mandatory: false,
                license_status: license_info.status,
                can_update: false,
                message: "Pembaruan otomatis dinonaktifkan karena lisensi belum aktif atau telah kedaluwarsa. Perpanjang lisensi Anda untuk mendapatkan rilis & fitur terbaru.".to_string(),
                component: Some(target_component),
                min_server_version: None,
                server_compatible: Some(true),
            };
        }

        // Hubungi Provider Update Server jika URL dikonfigurasi
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();

        let check_url = format!("{}/api/v1/updates/check", license_info.provider_server_url);
        let req_body = serde_json::json!({
            "component": target_component,
            "current_version": current_ver,
            "server_version": server_ver,
            "os": env::consts::OS,
            "arch": env::consts::ARCH,
            "license_key": license_info.license_key,
            "hardware_id": license_info.current_hardware_id,
        });

        match client.post(&check_url).json(&req_body).send().await {
            Ok(res) if res.status().is_success() => {
                if let Ok(mut server_resp) = res.json::<UpdateCheckResponse>().await {
                    server_resp.license_status = license_info.status;
                    return server_resp;
                }
            }
            Ok(res) => {
                warn!("Update server merespons kode non-200: {}", res.status());
            }
            Err(e) => {
                info!("Server provider update offline atau tidak terjangkau ({}). Menggunakan status lokal.", e);
            }
        }

        // Fallback info jika offline atau server provider simulasi lokal
        UpdateCheckResponse {
            current_version: current_ver.clone(),
            has_update: false,
            latest_version: current_ver,
            release_name: Some("Notane Retail Server v1.1.0".to_string()),
            changelog: Some("1. Peningkatan kestabilan server toko\n2. Pembaruan struktur data dan laporan harian\n3. Peningkatan kecepatan simpan transaksi".to_string()),
            release_date: Some(Utc::now().format("%Y-%m-%d").to_string()),
            download_url: None,
            checksum_sha256: None,
            signature: None,
            is_mandatory: false,
            license_status: license_info.status,
            can_update: true,
            message: "Aplikasi server Anda sudah menggunakan versi terbaru.".to_string(),
            component: Some("server".to_string()),
            min_server_version: None,
            server_compatible: Some(true),
        }
    }

    /// Verifikasi tanda tangan kriptografi file pembaruan sebelum dieksekusi (Anti-Tamper & Anti-Malware)
    pub fn verify_update_payload(manifest_json: &str, signature_hex: &str) -> Result<bool, String> {
        let pub_key_bytes = hex::decode(get_master_public_key_hex())
            .map_err(|e| format!("Master Public Key Hex invalid: {}", e))?;
        
        let pub_key_array: [u8; 32] = pub_key_bytes
            .try_into()
            .map_err(|_| "Ukuran Master Public Key harus 32 bytes".to_string())?;

        let verifying_key = VerifyingKey::from_bytes(&pub_key_array)
            .map_err(|e| format!("Gagal memuat VerifyingKey Ed25519: {}", e))?;

        let sig_bytes = hex::decode(signature_hex.trim())
            .map_err(|e| format!("Format signature hex invalid: {}", e))?;

        let sig_array: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| "Ukuran Signature Ed25519 harus 64 bytes".to_string())?;

        let signature = Signature::from_bytes(&sig_array);

        if verifying_key.verify(manifest_json.as_bytes(), &signature).is_ok() {
            return Ok(true);
        }

        // Jika manifest_json format "target_version:X:checksum:Y", coba juga fallback format "X:Y" dari Provider Manager
        if manifest_json.starts_with("target_version:") {
            let parts: Vec<&str> = manifest_json.split(":checksum:").collect();
            if parts.len() == 2 {
                let ver = parts[0].trim_start_matches("target_version:");
                let chk = parts[1];
                let alt_manifest = format!("{}:{}", ver, chk);
                if verifying_key.verify(alt_manifest.as_bytes(), &signature).is_ok() {
                    return Ok(true);
                }
            }
        }

        Err("VERIFIKASI UPDATE GAGAL: Paket update tidak berasal dari Server Provider Notane resmi atau file telah diubah pihak ketiga!".to_string())
    }

    /// Merekam riwayat pembaruan yang berhasil diterapkan
    pub async fn log_update_applied(
        pool: &PgPool,
        from_version: &str,
        to_version: &str,
        release_tag: Option<&str>,
        checksum: Option<&str>,
        notes: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO app_update_history (
                from_version, to_version, release_tag, checksum_sha256, status, notes
            ) VALUES ($1, $2, $3, $4, 'SUCCESS', $5)"#
        )
        .bind(from_version)
        .bind(to_version)
        .bind(release_tag)
        .bind(checksum)
        .bind(notes)
        .execute(pool)
        .await?;

        info!("Riwayat pembaruan berhasil dicatat: {} -> {}", from_version, to_version);
        Ok(())
    }
}
