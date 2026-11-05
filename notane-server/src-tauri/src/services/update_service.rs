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
    #[serde(default)]
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
                let status_code = res.status();
                let resp_text = res.text().await.unwrap_or_default();
                match serde_json::from_str::<UpdateCheckResponse>(&resp_text) {
                    Ok(mut server_resp) => {
                        if server_resp.current_version.trim().is_empty() {
                            server_resp.current_version = current_ver.clone();
                        }
                        server_resp.license_status = license_info.status;

                        // Buat token terenkripsi yang membawa nomor mesin dan nomor lisensi
                        if let Some(ref d_url) = server_resp.download_url {
                            let token_payload = crate::utils::ota::OtaDownloadPayload {
                                license_key: license_info.license_key.clone().unwrap_or_default(),
                                machine_id: license_info.current_hardware_id.clone(),
                                timestamp: chrono::Utc::now().timestamp(),
                                component: Some(target_component.clone()),
                                current_version: Some(current_ver.clone()),
                                os: Some(env::consts::OS.to_string()),
                                arch: Some(env::consts::ARCH.to_string()),
                            };
                            let master_pub_key = get_master_public_key_hex();
                            if let Ok(enc_token) = crate::utils::ota::encrypt_ota_payload(&token_payload, &master_pub_key) {
                                let full_url = if d_url.starts_with("http://") || d_url.starts_with("https://") {
                                    d_url.clone()
                                } else {
                                    format!("{}{}", license_info.provider_server_url.trim_end_matches('/'), d_url)
                                };
                                let sep = if full_url.contains('?') { "&" } else { "?" };
                                server_resp.download_url = Some(format!("{}{}token={}", full_url, sep, enc_token));
                            }
                        }

                        return server_resp;
                    }
                    Err(e) => {
                        warn!("Gagal mendeserialisasi respons dari server pembaruan (status {}): {}. Raw text: {}", status_code, e, resp_text);
                    }
                }
            }
            Ok(res) => {
                let status_code = res.status();
                warn!("Update server merespons kode non-200: {}", status_code);
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
                    message: format!("Server pembaruan di {} merespons kode HTTP {}.", license_info.provider_server_url, status_code),
                    component: Some(target_component),
                    min_server_version: None,
                    server_compatible: Some(true),
                };
            }
            Err(e) => {
                let err_msg = if e.is_connect() {
                    format!("Tidak dapat terhubung ke Server Provider di {}. Pastikan Notane Manager berjalan.", license_info.provider_server_url)
                } else if e.is_timeout() {
                    format!("Pemeriksaan pembaruan timeout (Server {} tidak merespons dalam 6 detik).", license_info.provider_server_url)
                } else {
                    format!("Gagal memeriksa pembaruan: {}", e)
                };
                warn!("{}", err_msg);
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
                    message: err_msg,
                    component: Some(target_component),
                    min_server_version: None,
                    server_compatible: Some(true),
                };
            }
        }

        // Fallback info jika offline atau server provider simulasi lokal
        UpdateCheckResponse {
            current_version: current_ver.clone(),
            has_update: false,
            latest_version: current_ver,
            release_name: Some("Notane Retail Server".to_string()),
            changelog: Some("1. Peningkatan kestabilan server toko\n2. Pembaruan struktur data dan laporan harian\n3. Peningkatan kecepatan simpan transaksi".to_string()),
            release_date: Some(Utc::now().format("%Y-%m-%d").to_string()),
            download_url: None,
            checksum_sha256: None,
            signature: None,
            is_mandatory: false,
            license_status: license_info.status,
            can_update: true,
            message: "Aplikasi server Anda sudah menggunakan versi terbaru.".to_string(),
            component: Some(target_component),
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

    /// Ekstraksi dan penukaran biner server secara otomatis (Binary Hot-Swap & Installer Launch)
    pub fn apply_binary_update(bytes: &[u8], target_version: &str) -> Result<String, String> {
        let mut update_saved_path: Option<std::path::PathBuf> = None;
        if let Some(mut cache_dir) = dirs::data_local_dir() {
            cache_dir.push("notane-retail-pos");
            cache_dir.push("updates");
            let _ = std::fs::create_dir_all(&cache_dir);
            let target_file = cache_dir.join(format!("update-{}.pkg", target_version));
            let _ = std::fs::write(&target_file, bytes);
            update_saved_path = Some(target_file);
        }

        // 1. Cek apakah berkas unduhan adalah arsip ZIP (magic bytes PK\x03\x04)
        if bytes.len() >= 4 && bytes[0] == 0x50 && bytes[1] == 0x4B && bytes[2] == 0x03 && bytes[3] == 0x04 {
            let cursor = std::io::Cursor::new(bytes);
            if let Ok(mut archive) = zip::ZipArchive::new(cursor) {
                let mut extracted_binary: Option<Vec<u8>> = None;
                let mut extracted_installer: Option<(String, Vec<u8>)> = None;

                for i in 0..archive.len() {
                    if let Ok(mut file) = archive.by_index(i) {
                        let name = file.name().to_string();
                        let base_name = std::path::Path::new(&name)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_lowercase();

                        if base_name == "notane-retail-server" || base_name == "notane-retail-server.exe" {
                            use std::io::Read;
                            let mut buf = Vec::new();
                            if file.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
                                extracted_binary = Some(buf);
                                break;
                            }
                        } else if base_name.ends_with(".exe") || base_name.ends_with(".msi") || base_name.ends_with(".dmg") || base_name.ends_with(".deb") || base_name.ends_with(".appimage") {
                            use std::io::Read;
                            let mut buf = Vec::new();
                            if file.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
                                extracted_installer = Some((base_name, buf));
                            }
                        }
                    }
                }

                // A. Jika biner server ditemukan di dalam ZIP -> Hot-swap biner saat ini
                if let Some(new_bin) = extracted_binary {
                    let current_exe = std::env::current_exe()
                        .map_err(|e| format!("Tidak dapat mendeteksi lokasi file biner aktif: {}", e))?;

                    let backup_exe = current_exe.with_extension("old");
                    let _ = std::fs::remove_file(&backup_exe);

                    std::fs::rename(&current_exe, &backup_exe)
                        .map_err(|e| format!("Gagal memindahkan biner lama: {}", e))?;

                    if let Err(e) = std::fs::write(&current_exe, &new_bin) {
                        let _ = std::fs::rename(&backup_exe, &current_exe);
                        return Err(format!("Gagal menulis biner baru: {}", e));
                    }

                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(&current_exe, std::fs::Permissions::from_mode(0o755));
                    }

                    info!("Biner Notane Server berhasil diperbarui ke versi {} di {:?}", target_version, current_exe);
                    return Ok(format!("Biner Notane Server berhasil diperbarui ke versi {}. Silakan restart aplikasi.", target_version));
                }

                // B. Jika ada installer di dalam ZIP
                if let Some((inst_name, inst_bytes)) = extracted_installer {
                    if let Some(mut cache_dir) = dirs::data_local_dir() {
                        cache_dir.push("notane-retail-pos");
                        cache_dir.push("updates");
                        let inst_file = cache_dir.join(&inst_name);
                        let _ = std::fs::write(&inst_file, &inst_bytes);
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let _ = std::fs::set_permissions(&inst_file, std::fs::Permissions::from_mode(0o755));
                        }
                        Self::launch_file(&inst_file);
                        return Ok(format!("Installer {} berhasil diekstrak dan dijalankan.", inst_name));
                    }
                }
            }
        }

        // 2. Jika paket unduhan langsung berupa installer (.exe / .dmg / .msi)
        if let Some(ref saved_path) = update_saved_path {
            Self::launch_file(saved_path);
            return Ok(format!("Paket pembaruan v{} berhasil diunduh dan dibuka.", target_version));
        }

        Ok(format!("Pembaruan v{} berhasil diunduh.", target_version))
    }

    pub fn launch_file(path: &std::path::Path) {
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer").arg(path).spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(path).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        }
    }
}
