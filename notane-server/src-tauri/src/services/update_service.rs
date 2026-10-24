use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::convert::TryInto;
use std::env;
use tracing::{info, warn};

use super::license_service::{LicenseService, LicenseState, NOTANE_MASTER_PUBLIC_KEY_HEX};

pub const APP_VERSION: &str = "1.1.0";
pub const APP_BUILD_DATE: &str = "2026-09-26";
pub const APP_NAME: &str = "Notane Retail POS Enterprise";

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
    pub release_date: Option<DateTime<Utc>>,
    pub download_url: Option<String>,
    pub checksum_sha256: Option<String>,
    pub signature: Option<String>,
    pub is_mandatory: bool,
    pub license_status: LicenseState,
    pub can_update: bool,
    pub message: String,
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
            version: APP_VERSION.to_string(),
            build_date: APP_BUILD_DATE.to_string(),
            target_os: env::consts::OS.to_string(),
            target_arch: env::consts::ARCH.to_string(),
            is_production: !cfg!(debug_assertions),
        }
    }

    /// Memeriksa ketersediaan pembaruan dari Server Provider Notane
    /// Akses pembaruan diproteksi oleh lisensi aktif pengguna
    pub async fn check_updates(pool: &PgPool) -> UpdateCheckResponse {
        let license_info = LicenseService::check_current_license(pool).await;
        let current_ver = APP_VERSION.to_string();

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
            };
        }

        // Hubungi Provider Update Server jika URL dikonfigurasi
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();

        let check_url = format!("{}/api/v1/updates/check", license_info.provider_server_url);
        let req_body = serde_json::json!({
            "current_version": current_ver,
            "os": env::consts::OS,
            "arch": env::consts::ARCH,
            "license_key": license_info.license_key,
            "hardware_id": license_info.current_hardware_id,
        });

        match client.post(&check_url).json(&req_body).send().await {
            Ok(res) if res.status().is_success() => {
                if let Ok(mut server_resp) = res.json::<UpdateCheckResponse>().await {
                    server_resp.can_update = true;
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
            release_name: Some("Notane POS v1.1.0 Enterprise LTS".to_string()),
            changelog: Some("- Sistem Isolasi Database PostgreSQL Terintegrasi\n- Keamanan Lisensi Asimetris Ed25519 & Node-Locking Anti-Baja\n- Migrasi Otomatis Skema Terkendali (Laravel Style)".to_string()),
            release_date: Some(Utc::now()),
            download_url: None,
            checksum_sha256: None,
            signature: None,
            is_mandatory: false,
            license_status: license_info.status,
            can_update: true,
            message: "Aplikasi Anda menggunakan versi terbaru (v1.1.0 Enterprise).".to_string(),
        }
    }

    /// Verifikasi tanda tangan kriptografi file pembaruan sebelum dieksekusi (Anti-Tamper & Anti-Malware)
    pub fn verify_update_payload(manifest_json: &str, signature_hex: &str) -> Result<bool, String> {
        let pub_key_bytes = hex::decode(NOTANE_MASTER_PUBLIC_KEY_HEX)
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

        verifying_key
            .verify(manifest_json.as_bytes(), &signature)
            .map_err(|_| "VERIFIKASI UPDATE GAGAL: Paket update tidak berasal dari Server Provider Notane resmi atau file telah diubah pihak ketiga!".to_string())?;

        Ok(true)
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
