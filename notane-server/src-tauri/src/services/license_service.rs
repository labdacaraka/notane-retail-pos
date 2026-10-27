use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::convert::TryInto;
use std::env;
use tracing::{error, info, warn};

/// Master Public Key milik Provider Notane (Ed25519 32-byte)
/// Private Key HANYA disimpan di Provider Licensing Server, tidak pernah ada di client!
pub const NOTANE_MASTER_PUBLIC_KEY_HEX: &str =
    "779b7d3fb80a25e9503bf7b85ffbd226ed9a4b8ff9de844872dd3a19357bb213";

/// Payload data sertifikat lisensi
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePayload {
    pub license_key: String,
    pub client_name: String,
    pub outlet_id: Option<uuid::Uuid>,
    pub tier: String,
    pub max_terminals: i32,
    pub hardware_id: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub offline_grace_days: i32,
    pub features: Vec<String>,
}

/// Status Lisensi Aplikasi
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LicenseState {
    Active,
    GracePeriod,
    Expired,
    HardwareMismatch,
    ClockTampered,
    #[default]
    Unlicensed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfoResponse {
    pub status: LicenseState,
    pub is_valid: bool,
    pub license_key: Option<String>,
    pub client_name: Option<String>,
    pub tier: Option<String>,
    pub max_terminals: i32,
    pub current_hardware_id: String,
    pub licensed_hardware_id: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub days_remaining: Option<i64>,
    pub grace_days_remaining: Option<i64>,
    pub features: Vec<String>,
    pub message: String,
    pub provider_server_url: String,
}

pub struct LicenseService;

impl LicenseService {
    /// Menghasilkan Hardware ID unik dan terenkripsi untuk mesin ini (Node-Locking)
    /// Berasal dari kombinasi identitas CPU, OS Machine GUID / UUID, dan Hostname + Salt
    pub fn get_hardware_id() -> String {
        let os_family = env::consts::OS;
        let arch = env::consts::ARCH;

        // Ambil machine id lokal
        let machine_guid = Self::read_os_machine_guid();

        let mut hasher = Sha256::new();
        hasher.update(b"NOTANE_RETAIL_ENTERPRISE_NODE_SALT_2026::");
        hasher.update(os_family.as_bytes());
        hasher.update(b"::");
        hasher.update(arch.as_bytes());
        hasher.update(b"::");
        hasher.update(machine_guid.as_bytes());

        let hash_result = hex::encode(hasher.finalize());
        let chunk1 = &hash_result[0..4];
        let chunk2 = &hash_result[4..8];
        let chunk3 = &hash_result[8..12];
        let chunk4 = &hash_result[12..16];

        format!("NOTANE-HWID-{}-{}-{}-{}", chunk1, chunk2, chunk3, chunk4).to_uppercase()
    }

    fn read_os_machine_guid() -> String {
        // macOS: IOPlatformUUID
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("ioreg")
                .args(["-rd1", "-c", "IOPlatformExpertDevice"])
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    if line.contains("IOPlatformUUID") {
                        if let Some(pos) = line.find('=') {
                            let uuid = line[pos + 1..].replace(['"', ' ', '\t'], "");
                            return uuid;
                        }
                    }
                }
            }
        }

        // Linux: /etc/machine-id
        #[cfg(target_os = "linux")]
        {
            if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
                return id.trim().to_string();
            }
            if let Ok(id) = std::fs::read_to_string("/var/lib/dbus/machine-id") {
                return id.trim().to_string();
            }
        }

        // Windows: MachineGuid Registry (via fallback PowerShell / env)
        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = std::process::Command::new("powershell")
                .args(["-Command", "(Get-ItemProperty -Path 'HKLM:\\SOFTWARE\\Microsoft\\Cryptography').MachineGuid"])
                .output()
            {
                let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !id.is_empty() {
                    return id;
                }
            }
        }

        // Fallback: hostname & username
        let user = env::var("USER").or_else(|_| env::var("USERNAME")).unwrap_or_else(|_| "node".into());
        let host = env::var("HOSTNAME").or_else(|_| env::var("COMPUTERNAME")).unwrap_or_else(|_| "pos-host".into());
        format!("{}-{}", host, user)
    }

    /// Memvalidasi tanda tangan kriptografi Ed25519 dari lisensi
    pub fn verify_signature(token_json: &str, signature_hex: &str) -> Result<LicensePayload, String> {
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

        // Verifikasi asimetris: HANYA Server Provider yang memegang Private Key yang bisa membuat tanda tangan valid
        verifying_key
            .verify(token_json.as_bytes(), &signature)
            .map_err(|_| "VERIFIKASI GAGAL: Tanda tangan kriptografi lisensi tidak valid atau telah dimodifikasi (Anti-Cracking triggered)!".to_string())?;

        let payload: LicensePayload = serde_json::from_str(token_json)
            .map_err(|e| format!("Format payload JSON lisensi korup: {}", e))?;

        Ok(payload)
    }

    pub const DEFAULT_OFFICIAL_PROVIDER_URL: &str = "https://lisensi.notane.id";
    pub const DEFAULT_LOCAL_PROVIDER_URL: &str = "http://127.0.0.1:19090";

    /// URL Provider Resmi yang dibakar ke dalam biner saat proses kompilasi (Compile-Time Baked).
    /// Dapat disetel atau diganti saat proses build (misal di GitHub Actions CI/CD via NOTANE_RELEASE_PROVIDER_URL).
    pub const BAKED_PRODUCTION_PROVIDER_URL: Option<&'static str> = option_env!("NOTANE_RELEASE_PROVIDER_URL");

    /// Mengambil URL Provider Server dengan urutan prioritas:
    /// 1. Environment Variable Runtime (`PROVIDER_SERVER_URL` / `NOTANE_PROVIDER_SERVER_URL`):
    ///    Memungkinkan pengujian lisensi lokal / development (misal: http://127.0.0.1:19090 atau server development).
    /// 2. Nilai yang dibakar saat proses build di CI/CD (`NOTANE_RELEASE_PROVIDER_URL`).
    /// 3. Fallback resmi default:
    ///    - Production: `https://lisensi.notane.id`
    ///    - Development/Debug: `http://127.0.0.1:19090`
    pub fn get_provider_url() -> String {
        let env_url = env::var("PROVIDER_SERVER_URL")
            .or_else(|_| env::var("provider_server_url"))
            .or_else(|_| env::var("NOTANE_PROVIDER_SERVER_URL"))
            .or_else(|_| env::var("notane_provider_server_url"));

        if let Ok(val) = env_url {
            let trimmed = val.trim().trim_end_matches('/');
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }

        if let Some(baked_url) = Self::BAKED_PRODUCTION_PROVIDER_URL {
            let trimmed = baked_url.trim().trim_end_matches('/');
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }

        Self::DEFAULT_OFFICIAL_PROVIDER_URL.to_string()
    }

    /// Memeriksa status lisensi dari database lokal & mendeteksi manipulasi jam (Anti-Clock Rollback)
    pub async fn check_current_license(pool: &PgPool) -> LicenseInfoResponse {
        let current_hwid = Self::get_hardware_id();
        let provider_url = Self::get_provider_url();

        let row = sqlx::query(
            "SELECT 
                license_key, client_name, tier, max_terminals, hardware_id,
                license_token, signature, issued_at, expires_at, offline_grace_days,
                last_verified_at, last_monotonic_time, status
               FROM app_licenses 
               ORDER BY id DESC LIMIT 1"
        )
        .fetch_optional(pool)
        .await;

        let record = match row {
            Ok(Some(r)) => {
                let license_key: String = r.get("license_key");
                let client_name: String = r.get("client_name");
                let tier: String = r.get("tier");
                let max_terminals: i32 = r.get("max_terminals");
                let hardware_id: String = r.get("hardware_id");
                let license_token: String = r.get("license_token");
                let signature: String = r.get("signature");
                let expires_at: DateTime<Utc> = r.get("expires_at");
                let last_monotonic_time: i64 = r.get("last_monotonic_time");
                (license_key, client_name, tier, max_terminals, hardware_id, license_token, signature, expires_at, last_monotonic_time)
            }
            _ => {
                return LicenseInfoResponse {
                    status: LicenseState::Unlicensed,
                    is_valid: false,
                    license_key: None,
                    client_name: None,
                    tier: None,
                    max_terminals: 1,
                    current_hardware_id: current_hwid,
                    licensed_hardware_id: None,
                    expires_at: None,
                    days_remaining: None,
                    grace_days_remaining: None,
                    features: vec!["pos_offline_trial".to_string()],
                    message: "Aplikasi belum teraktivasi lisensi resmi. Silakan hubungi Provider Notane Retail POS untuk mengaktifkan lisensi mesin ini.".to_string(),
                    provider_server_url: provider_url,
                };
            }
        };

        let (rec_license_key, rec_client_name, rec_tier, rec_max_terminals, rec_hardware_id, rec_license_token, rec_signature, rec_expires_at, rec_last_monotonic_time) = record;

        // 1. Verifikasi tanda tangan kriptografi
        let payload = match Self::verify_signature(&rec_license_token, &rec_signature) {
            Ok(p) => p,
            Err(err) => {
                error!("Pelanggaran integritas lisensi: {}", err);
                return LicenseInfoResponse {
                    status: LicenseState::Unlicensed,
                    is_valid: false,
                    license_key: Some(rec_license_key),
                    client_name: Some(rec_client_name),
                    tier: Some(rec_tier),
                    max_terminals: rec_max_terminals,
                    current_hardware_id: current_hwid,
                    licensed_hardware_id: Some(rec_hardware_id),
                    expires_at: Some(rec_expires_at),
                    days_remaining: None,
                    grace_days_remaining: None,
                    features: vec![],
                    message: "Sertifikat lisensi lokal terdeteksi rusak atau dimodifikasi secara ilegal.".to_string(),
                    provider_server_url: provider_url,
                };
            }
        };

        // 2. Node Locking: Validasi kecocokan Hardware ID
        if payload.hardware_id != current_hwid {
            warn!("Hardware ID Mismatch! Mesin: {}, Lisensi: {}", current_hwid, payload.hardware_id);
            return LicenseInfoResponse {
                status: LicenseState::HardwareMismatch,
                is_valid: false,
                license_key: Some(payload.license_key),
                client_name: Some(payload.client_name),
                tier: Some(payload.tier),
                max_terminals: payload.max_terminals,
                current_hardware_id: current_hwid,
                licensed_hardware_id: Some(payload.hardware_id),
                expires_at: Some(payload.expires_at),
                days_remaining: None,
                grace_days_remaining: None,
                features: vec![],
                message: "Lisensi ini terdaftar untuk mesin perangkat keras lain (Node-Locked). Hubungi provider untuk transfer perangkat.".to_string(),
                provider_server_url: provider_url,
            };
        }

        // 3. Anti-Clock-Rollback: Periksa apakah jam sistem dimundurkan oleh pengguna
        let now = Utc::now();
        let now_sec = now.timestamp();
        if now_sec < rec_last_monotonic_time {
            error!("Anti-Clock-Rollback terpicu: Waktu sistem ({}) lebih lampau dari transaksi terakhir ({})", now_sec, rec_last_monotonic_time);
            return LicenseInfoResponse {
                status: LicenseState::ClockTampered,
                is_valid: false,
                license_key: Some(payload.license_key),
                client_name: Some(payload.client_name),
                tier: Some(payload.tier),
                max_terminals: payload.max_terminals,
                current_hardware_id: current_hwid,
                licensed_hardware_id: Some(payload.hardware_id),
                expires_at: Some(payload.expires_at),
                days_remaining: None,
                grace_days_remaining: None,
                features: vec![],
                message: "Terdeteksi manipulasi jam sistem operasi (Clock Rollback). Sinkronkan jam komputer Anda ke waktu internet.".to_string(),
                provider_server_url: provider_url,
            };
        }

        // Perbarui monotonic time check
        let _ = sqlx::query(
            "UPDATE app_licenses SET last_monotonic_time = $1, updated_at = CURRENT_TIMESTAMP WHERE id = (SELECT id FROM app_licenses ORDER BY id DESC LIMIT 1)"
        )
        .bind(now_sec)
        .execute(pool)
        .await;

        // 4. Evaluasi Tanggal Kedaluwarsa & Masa Toleransi Offline (Offline Grace Period)
        let diff = payload.expires_at.signed_duration_since(now);
        let days_remaining = diff.num_days();

        if days_remaining >= 0 {
            // Lisensi Aktif Normal
            LicenseInfoResponse {
                status: LicenseState::Active,
                is_valid: true,
                license_key: Some(payload.license_key),
                client_name: Some(payload.client_name),
                tier: Some(payload.tier),
                max_terminals: payload.max_terminals,
                current_hardware_id: current_hwid,
                licensed_hardware_id: Some(payload.hardware_id),
                expires_at: Some(payload.expires_at),
                days_remaining: Some(days_remaining),
                grace_days_remaining: None,
                features: payload.features,
                message: format!("Lisensi Aktif (Sisa {} hari).", days_remaining),
                provider_server_url: provider_url,
            }
        } else {
            // Masa aktif habis, periksa apakah masih dalam toleransi grace period
            let grace_limit = payload.expires_at + chrono::Duration::days(payload.offline_grace_days as i64);
            let grace_diff = grace_limit.signed_duration_since(now);
            let grace_days_left = grace_diff.num_days();

            if grace_days_left >= 0 {
                LicenseInfoResponse {
                    status: LicenseState::GracePeriod,
                    is_valid: true, // Masih diizinkan transaksi kasir darurat
                    license_key: Some(payload.license_key),
                    client_name: Some(payload.client_name),
                    tier: Some(payload.tier),
                    max_terminals: payload.max_terminals,
                    current_hardware_id: current_hwid,
                    licensed_hardware_id: Some(payload.hardware_id),
                    expires_at: Some(payload.expires_at),
                    days_remaining: Some(0),
                    grace_days_remaining: Some(grace_days_left),
                    features: payload.features,
                    message: format!("PERINGATAN: Masa aktif lisensi habis! Masa tenggang darurat (Grace Period) tersisa {} hari.", grace_days_left),
                    provider_server_url: provider_url,
                }
            } else {
                LicenseInfoResponse {
                    status: LicenseState::Expired,
                    is_valid: false,
                    license_key: Some(payload.license_key),
                    client_name: Some(payload.client_name),
                    tier: Some(payload.tier),
                    max_terminals: payload.max_terminals,
                    current_hardware_id: current_hwid,
                    licensed_hardware_id: Some(payload.hardware_id),
                    expires_at: Some(payload.expires_at),
                    days_remaining: Some(days_remaining),
                    grace_days_remaining: Some(0),
                    features: vec![],
                    message: "Lisensi telah kedaluwarsa sepenuhnya. Layanan terkunci. Silakan perpanjang langganan lisensi Anda.".to_string(),
                    provider_server_url: provider_url,
                }
            }
        }
    }

    /// Aktivasi lisensi baru ke dalam database lokal
    pub async fn activate_license(
        pool: &PgPool,
        token_json: &str,
        signature_hex: &str,
    ) -> Result<LicenseInfoResponse, String> {
        let payload = Self::verify_signature(token_json, signature_hex)?;
        let current_hwid = Self::get_hardware_id();

        if payload.hardware_id != current_hwid {
            return Err(format!(
                "Lisensi tidak cocok untuk hardware ini. Terdaftar untuk: {}, ID mesin ini: {}",
                payload.hardware_id, current_hwid
            ));
        }

        let now = Utc::now();
        let now_sec = now.timestamp();

        sqlx::query(
            r#"INSERT INTO app_licenses (
                license_key, client_name, outlet_id, tier, max_terminals,
                hardware_id, license_token, signature, issued_at, expires_at,
                offline_grace_days, last_verified_at, last_monotonic_time, status
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, CURRENT_TIMESTAMP, $12, 'ACTIVE'
            )
            ON CONFLICT (license_key) DO UPDATE SET
                client_name = EXCLUDED.client_name,
                tier = EXCLUDED.tier,
                max_terminals = EXCLUDED.max_terminals,
                license_token = EXCLUDED.license_token,
                signature = EXCLUDED.signature,
                expires_at = EXCLUDED.expires_at,
                offline_grace_days = EXCLUDED.offline_grace_days,
                last_verified_at = CURRENT_TIMESTAMP,
                last_monotonic_time = EXCLUDED.last_monotonic_time,
                status = 'ACTIVE',
                updated_at = CURRENT_TIMESTAMP"#
        )
        .bind(&payload.license_key)
        .bind(&payload.client_name)
        .bind(payload.outlet_id)
        .bind(&payload.tier)
        .bind(payload.max_terminals)
        .bind(&payload.hardware_id)
        .bind(token_json)
        .bind(signature_hex.trim())
        .bind(payload.issued_at)
        .bind(payload.expires_at)
        .bind(payload.offline_grace_days)
        .bind(now_sec)
        .execute(pool)
        .await
        .map_err(|e| format!("Gagal menyimpan aktivasi lisensi ke database: {}", e))?;

        info!("Aktivasi lisensi berhasil: {} ({})", payload.license_key, payload.client_name);
        Ok(Self::check_current_license(pool).await)
    }
}
