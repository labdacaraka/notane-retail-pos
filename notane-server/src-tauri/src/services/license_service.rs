use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Row};
use std::convert::TryInto;
use std::env;
use std::path::PathBuf;
use tracing::{error, info, warn};

/// Master Public Key milik Provider Notane (Ed25519 32-byte)
/// Didefinisikan secara terenkripsi compile-time (XOR masked) agar kebal dari AI / decompiler string inspection.
const MASKED_MASTER_PUBKEY: [u8; 64] = [
    0x6d, 0x08, 0xa5, 0x70, 0xbf, 0x1f, 0x72, 0x98, 0x38, 0x07, 0xac, 0x73, 0xba, 0x4e, 0x24, 0xc7,
    0x6f, 0x0f, 0xaf, 0x70, 0xee, 0x4c, 0x23, 0xc6, 0x6f, 0x59, 0xfa, 0x70, 0xec, 0x49, 0x73, 0xc8,
    0x3f, 0x5b, 0xa5, 0x73, 0xbc, 0x19, 0x79, 0x98, 0x3c, 0x06, 0xf8, 0x77, 0xb0, 0x4f, 0x75, 0xc6,
    0x6d, 0x0d, 0xf8, 0x76, 0xbb, 0x1a, 0x70, 0xc7, 0x69, 0x0a, 0xab, 0x70, 0xea, 0x49, 0x70, 0xcd,
];
const MASK_KEY: [u8; 8] = [0x5a, 0x3f, 0x9c, 0x12, 0x88, 0x7b, 0x41, 0xfe];

#[inline(never)]
pub fn get_master_public_key_hex() -> String {
    MASKED_MASTER_PUBKEY
        .iter()
        .enumerate()
        .map(|(i, &b)| (b ^ MASK_KEY[i % 8]) as char)
        .collect()
}

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

/// Struktur file vault lisensi terenkripsi lokal (Zero Database Storage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedLicenseVault {
    pub license_token: String,
    pub signature: String,
    pub last_monotonic_time: i64,
    pub activated_at: DateTime<Utc>,
    #[serde(default)]
    pub last_sync_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_sync_status: Option<String>,
    #[serde(default)]
    pub consecutive_failures: i32,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_sync_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consecutive_sync_failures: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offline_grace_days_remaining: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LicenseSyncLogItem {
    pub id: i64,
    pub sync_type: String,
    pub status: String,
    pub http_status: Option<i32>,
    pub latency_ms: Option<i32>,
    pub provider_url: String,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseSyncResult {
    pub success: bool,
    pub status: String,
    pub http_status: Option<u16>,
    pub latency_ms: Option<u128>,
    pub message: String,
    pub provider_url: String,
    pub license_info: LicenseInfoResponse,
}

pub struct LicenseService;

impl LicenseService {
    /// Menghasilkan Hardware ID unik dan terenkripsi untuk mesin ini (Node-Locking)
    /// Berasal dari kombinasi identitas CPU, OS Machine GUID / UUID, dan Hostname + Salt
    pub fn get_hardware_id() -> String {
        static CACHED_HWID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        CACHED_HWID.get_or_init(|| {
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
        }).clone()
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
            #[allow(unused_imports)]
            use std::os::windows::process::CommandExt;
            let mut cmd = std::process::Command::new("powershell");
            cmd.args(["-NoProfile", "-NonInteractive", "-Command", "(Get-ItemProperty -Path 'HKLM:\\SOFTWARE\\Microsoft\\Cryptography').MachineGuid"]);
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            if let Ok(output) = cmd.output() {
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
    pub const BAKED_PRODUCTION_PROVIDER_URL: Option<&'static str> = option_env!("NOTANE_RELEASE_PROVIDER_URL");

    /// Mengambil URL Provider Server dengan urutan prioritas environment/baked
    pub fn get_provider_url() -> String {
        // 1. Environment variable (.env atau sistem host saat development / server container)
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

        // 2. Compile-time baked URL (saat proses rilis binary CI/CD)
        if let Some(baked_url) = Self::BAKED_PRODUCTION_PROVIDER_URL {
            let trimmed = baked_url.trim().trim_end_matches('/');
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }

        // 3. Default Official Cloud Production URL
        Self::DEFAULT_OFFICIAL_PROVIDER_URL.to_string()
    }

    /// Mengembalikan path file vault terenkripsi primer
    pub fn get_vault_file_path() -> PathBuf {
        if let Some(mut base_dir) = dirs::data_local_dir() {
            base_dir.push("notane-retail-pos");
            let _ = std::fs::create_dir_all(&base_dir);
            base_dir.push(".license_vault.enc");
            base_dir
        } else {
            PathBuf::from(".license_vault.enc")
        }
    }

    /// Menghasilkan daftar kandidat lokasi file vault (App Data lokal & Root Project)
    pub fn get_candidate_vault_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        // 1. Lokasi App Data terisolasi
        if let Some(mut base_dir) = dirs::data_local_dir() {
            base_dir.push("notane-retail-pos");
            paths.push(base_dir.join(".license_vault.enc"));
        }
        // 2. Lokasi direktori kerja / project root
        paths.push(PathBuf::from(".license_vault.enc"));
        paths
    }

    /// Turunkan kunci simetris 256-bit AES-GCM dari Hardware ID mesin + Salt internal
    /// Kunci ini menjamin file lisensi terikat secara absolut pada hardware (Node-Locked)
    fn derive_vault_key() -> [u8; 32] {
        let hwid = Self::get_hardware_id();
        let mut hasher = Sha256::new();
        hasher.update(b"NOTANE_RETAIL_ENTERPRISE_VAULT_KEY_SALT_2026::");
        hasher.update(hwid.as_bytes());
        hasher.update(b"::ANTI_REVERSE_AES_GCM_NODE_LOCK");
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        key
    }

    /// Menyimpan data lisensi terenkripsi ke file vault disk dan sinkronisasi ke DB cache
    pub async fn save_license_vault_with_db(
        vault: &EncryptedLicenseVault,
        pool: Option<&PgPool>,
    ) -> Result<(), String> {
        let plaintext = serde_json::to_vec(vault)
            .map_err(|e| format!("Gagal serialisasi data vault: {}", e))?;

        let key = Self::derive_vault_key();
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| format!("Gagal inisialisasi cipher AES-256-GCM: {}", e))?;

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| format!("Enkripsi vault lisensi gagal: {}", e))?;

        let mut payload = nonce_bytes.to_vec();
        payload.extend_from_slice(&ciphertext);

        // 1. Tulis ke file disk primer
        let primary_path = Self::get_vault_file_path();
        if let Some(parent) = primary_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&primary_path, &payload);

        // 2. Tulis ke direktori project root jika berbeda
        let local_path = PathBuf::from(".license_vault.enc");
        if local_path != primary_path {
            let _ = std::fs::write(&local_path, &payload);
        }

        // 3. Sinkronisasi ke PostgreSQL license_encrypted_cache jika pool tersedia
        if let Some(p) = pool {
            let current_hwid = Self::get_hardware_id();
            let key_opt = Self::verify_signature(&vault.license_token, &vault.signature)
                .ok()
                .map(|pay| pay.license_key);

            let _ = sqlx::query(
                "INSERT INTO license_encrypted_cache (
                    id, encrypted_vault, hardware_id, license_key, last_monotonic_time, activated_at, updated_at
                 ) VALUES (
                    1, $1, $2, $3, $4, $5, CURRENT_TIMESTAMP
                 )
                 ON CONFLICT (id) DO UPDATE SET
                    encrypted_vault = EXCLUDED.encrypted_vault,
                    hardware_id = EXCLUDED.hardware_id,
                    license_key = EXCLUDED.license_key,
                    last_monotonic_time = EXCLUDED.last_monotonic_time,
                    activated_at = EXCLUDED.activated_at,
                    updated_at = CURRENT_TIMESTAMP"
            )
            .bind(&payload)
            .bind(&current_hwid)
            .bind(key_opt)
            .bind(vault.last_monotonic_time)
            .bind(vault.activated_at)
            .execute(p)
            .await;
        }

        info!("File vault lisensi berhasil disimpan dan dienkripsi (AES-256-GCM + Hardware Lock + Dual Storage)");
        Ok(())
    }

    /// Menyimpan data lisensi terenkripsi ke file vault lokal (sinkronus fallback)
    pub fn save_license_vault(vault: &EncryptedLicenseVault) -> Result<(), String> {
        let plaintext = serde_json::to_vec(vault)
            .map_err(|e| format!("Gagal serialisasi data vault: {}", e))?;

        let key = Self::derive_vault_key();
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| format!("Gagal inisialisasi cipher AES-256-GCM: {}", e))?;

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| format!("Enkripsi vault lisensi gagal: {}", e))?;

        let mut payload = nonce_bytes.to_vec();
        payload.extend_from_slice(&ciphertext);

        let primary_path = Self::get_vault_file_path();
        if let Some(parent) = primary_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&primary_path, &payload)
            .map_err(|e| format!("Gagal menulis file vault lisensi ke {:?}: {}", primary_path, e))?;

        let local_path = PathBuf::from(".license_vault.enc");
        if local_path != primary_path {
            let _ = std::fs::write(&local_path, &payload);
        }

        Ok(())
    }

    /// Membaca dan mendekripsi data lisensi dari file vault disk
    pub fn load_license_vault() -> Result<Option<EncryptedLicenseVault>, String> {
        let candidate_paths = Self::get_candidate_vault_paths();
        let mut file_content = None;

        for path in candidate_paths {
            if path.exists() {
                if let Ok(bytes) = std::fs::read(&path) {
                    if !bytes.is_empty() {
                        file_content = Some(bytes);
                        break;
                    }
                }
            }
        }

        let bytes = match file_content {
            Some(b) => b,
            None => return Ok(None),
        };

        if bytes.len() < 12 {
            return Err("File lisensi korup atau tidak valid (ukuran terlalu kecil).".to_string());
        }

        let (nonce_bytes, ciphertext) = bytes.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let key = Self::derive_vault_key();
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| format!("Gagal inisialisasi cipher AES-256-GCM: {}", e))?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| "VERIFIKASI FILE LISENSI GAGAL: File lisensi terenkripsi tidak cocok dengan Hardware ID mesin ini atau telah dimanipulasi secara ilegal!".to_string())?;

        let vault: EncryptedLicenseVault = serde_json::from_slice(&plaintext)
            .map_err(|e| format!("Struktur JSON vault lisensi tidak valid: {}", e))?;

        Ok(Some(vault))
    }

    /// Membaca lisensi dengan mekanisme fail-safe Dual-Storage (Disk + PostgreSQL DB Cache)
    pub async fn load_license_vault_with_db(pool: &PgPool) -> Result<Option<EncryptedLicenseVault>, String> {
        // 1. Coba baca dari file lokal disk terlebih dahulu
        let disk_vault = Self::load_license_vault();
        if let Ok(Some(vault)) = disk_vault {
            // Pastikan database juga memiliki salinan cache jika belum ada
            let db_has_cache: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM license_encrypted_cache WHERE id = 1)")
                .fetch_one(pool)
                .await
                .unwrap_or(false);

            if !db_has_cache {
                let _ = Self::save_license_vault_with_db(&vault, Some(pool)).await;
            }
            return Ok(Some(vault));
        }

        // 2. Jika file disk tidak ada / terhapus (misal container Docker di-recreate), pulihkan dari database PostgreSQL!
        let db_row: Option<Vec<u8>> = sqlx::query_scalar("SELECT encrypted_vault FROM license_encrypted_cache WHERE id = 1")
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

        if let Some(bytes) = db_row {
            if bytes.len() >= 12 {
                let (nonce_bytes, ciphertext) = bytes.split_at(12);
                let nonce = Nonce::from_slice(nonce_bytes);
                let key = Self::derive_vault_key();
                let cipher = Aes256Gcm::new_from_slice(&key)
                    .map_err(|e| format!("Gagal inisialisasi cipher AES-256-GCM: {}", e))?;

                match cipher.decrypt(nonce, ciphertext) {
                    Ok(plaintext) => {
                        if let Ok(vault) = serde_json::from_slice::<EncryptedLicenseVault>(&plaintext) {
                            // Pulihkan kembali file ke disk agar akses berikutnya cepat
                            let primary_path = Self::get_vault_file_path();
                            if let Some(parent) = primary_path.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            let _ = std::fs::write(&primary_path, &bytes);
                            let local_path = PathBuf::from(".license_vault.enc");
                            if local_path != primary_path {
                                let _ = std::fs::write(&local_path, &bytes);
                            }
                            info!("Berhasil memulihkan lisensi terenkripsi dari database cache ke disk lokal.");
                            return Ok(Some(vault));
                        }
                    }
                    Err(_) => {
                        return Err("VERIFIKASI FILE LISENSI GAGAL: File lisensi terenkripsi tidak cocok dengan Hardware ID mesin ini atau telah dimanipulasi secara ilegal!".to_string());
                    }
                }
            }
        }

        // 3. Fallback jika masih ada sisa tabel legacy database lama
        let has_legacy_table: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'app_licenses')"
        )
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        if has_legacy_table {
            if let Ok(Some(r)) = sqlx::query(
                "SELECT license_token, signature, last_monotonic_time, created_at FROM app_licenses ORDER BY id DESC LIMIT 1"
            )
            .fetch_optional(pool)
            .await
            {
                let token: String = r.get("license_token");
                let sig: String = r.get("signature");
                let mono: i64 = r.get("last_monotonic_time");
                let act_at: DateTime<Utc> = r.get("created_at");

                let legacy_vault = EncryptedLicenseVault {
                    license_token: token,
                    signature: sig,
                    last_monotonic_time: mono,
                    activated_at: act_at,
                    last_sync_at: None,
                    last_sync_status: Some("INITIALIZED".to_string()),
                    consecutive_failures: 0,
                };

                let _ = Self::save_license_vault_with_db(&legacy_vault, Some(pool)).await;
                let _ = sqlx::query("DROP TABLE IF EXISTS app_licenses CASCADE").execute(pool).await;

                return Ok(Some(legacy_vault));
            }
        }

        Ok(None)
    }

    /// Memeriksa status lisensi dari file vault terenkripsi & toleransi sinkronisasi offline (Anti-Clock Rollback & Grace Period)
    pub async fn check_current_license(pool: &PgPool) -> LicenseInfoResponse {
        let current_hwid = Self::get_hardware_id();
        let provider_url = Self::get_provider_url();

        // 1. Coba baca dari file vault terenkripsi (Disk + DB Cache)
        let vault_opt = match Self::load_license_vault_with_db(pool).await {
            Ok(v) => v,
            Err(err_msg) => {
                error!("Gagal membaca vault lisensi terenkripsi: {}", err_msg);
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
                    features: vec![],
                    message: format!("Integritas lisensi rusak: {}. Silakan lakukan aktivasi ulang.", err_msg),
                    provider_server_url: provider_url,
                    last_sync_at: None,
                    sync_status: None,
                    consecutive_sync_failures: None,
                    offline_grace_days_remaining: None,
                };
            }
        };

        let mut vault = match vault_opt {
            Some(v) => v,
            None => {
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
                    last_sync_at: None,
                    sync_status: None,
                    consecutive_sync_failures: None,
                    offline_grace_days_remaining: None,
                };
            }
        };

        // 2. Verifikasi tanda tangan kriptografi Ed25519
        let payload = match Self::verify_signature(&vault.license_token, &vault.signature) {
            Ok(p) => p,
            Err(err) => {
                error!("Pelanggaran integritas lisensi: {}", err);
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
                    features: vec![],
                    message: "Sertifikat lisensi lokal terdeteksi rusak atau dimodifikasi secara ilegal.".to_string(),
                    provider_server_url: provider_url,
                    last_sync_at: None,
                    sync_status: None,
                    consecutive_sync_failures: None,
                    offline_grace_days_remaining: None,
                };
            }
        };

        // 3. Node Locking: Validasi kecocokan Hardware ID
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
                last_sync_at: None,
                sync_status: None,
                consecutive_sync_failures: None,
                offline_grace_days_remaining: None,
            };
        }

        // 4. Anti-Clock-Rollback: Periksa apakah jam sistem dimundurkan
        let now = Utc::now();
        let now_sec = now.timestamp();
        if now_sec < vault.last_monotonic_time {
            error!("Anti-Clock-Rollback terpicu: Waktu sistem ({}) lebih lampau dari transaksi terakhir ({})", now_sec, vault.last_monotonic_time);
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
                last_sync_at: None,
                sync_status: None,
                consecutive_sync_failures: None,
                offline_grace_days_remaining: None,
            };
        }

        // Perbarui monotonic time jika berjalan maju (> 60 detik)
        if now_sec > vault.last_monotonic_time + 60 {
            vault.last_monotonic_time = now_sec;
            let _ = Self::save_license_vault_with_db(&vault, Some(pool)).await;
        }

        // 5. Anti-Tamper: Ambil status sinkronisasi provider langsung dari dalam vault terenkripsi (AES-256-GCM + HWID)
        // Data ini kebal dari manipulasi pgAdmin karena disegel secara kriptografis di dalam ciphertext ciphertext vault.
        let last_sync_at = vault.last_sync_at;
        let sync_status = vault.last_sync_status.clone().unwrap_or_else(|| "INITIALIZED".to_string());
        let consecutive_failures = vault.consecutive_failures;

        // Jika lisensi telah dicabut resmi oleh provider
        if sync_status == "REVOKED" {
            return LicenseInfoResponse {
                status: LicenseState::Unlicensed,
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
                message: "Lisensi telah dicabut atau dinonaktifkan oleh Provider Notane.".to_string(),
                provider_server_url: provider_url,
                last_sync_at,
                sync_status: Some(sync_status),
                consecutive_sync_failures: Some(consecutive_failures),
                offline_grace_days_remaining: Some(0),
            };
        }

        // 6. Evaluasi Kedaluwarsa Sertifikat & Kebijakan Toleransi Offline (Offline Grace Period)
        let diff = payload.expires_at.signed_duration_since(now);
        let days_remaining = diff.num_days();

        let grace_limit_days = if payload.offline_grace_days > 0 { payload.offline_grace_days as i64 } else { 7 };
        let last_sync_ref = last_sync_at.unwrap_or(vault.activated_at);
        let days_since_sync = (now - last_sync_ref).num_days();
        let offline_grace_left = (grace_limit_days - days_since_sync).max(0);

        if days_remaining < 0 {
            // Sertifikat lisensi sendiri sudah habis masa berlakunya
            let grace_limit = payload.expires_at + chrono::Duration::days(grace_limit_days);
            let grace_diff = grace_limit.signed_duration_since(now);
            let grace_days_left = grace_diff.num_days();

            if grace_days_left >= 0 {
                LicenseInfoResponse {
                    status: LicenseState::GracePeriod,
                    is_valid: true,
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
                    message: format!("PERINGATAN: Masa aktif lisensi telah habis! Masa tenggang darurat tersisa {} hari.", grace_days_left),
                    provider_server_url: provider_url,
                    last_sync_at,
                    sync_status: Some(sync_status),
                    consecutive_sync_failures: Some(consecutive_failures),
                    offline_grace_days_remaining: Some(0),
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
                    last_sync_at,
                    sync_status: Some(sync_status),
                    consecutive_sync_failures: Some(consecutive_failures),
                    offline_grace_days_remaining: Some(0),
                }
            }
        } else {
            // Sertifikat lisensi valid. Sekarang evaluasi toleransi jaringan offline provider:
            if sync_status == "PROVIDER_UNREACHABLE" {
                if days_since_sync <= grace_limit_days {
                    // TETAP AKTIF NORMAL! Server provider mati/down, namun aplikasi tetap bisa digunakan penuh
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
                        message: format!("Lisensi Aktif (Mode Offline Aman: Server lisensi belum terhubung sejak {} hari lalu. Sisa toleransi offline: {} hari).", days_since_sync, offline_grace_left),
                        provider_server_url: provider_url,
                        last_sync_at,
                        sync_status: Some("PROVIDER_UNREACHABLE".to_string()),
                        consecutive_sync_failures: Some(consecutive_failures),
                        offline_grace_days_remaining: Some(offline_grace_left),
                    }
                } else if days_since_sync <= grace_limit_days + 3 {
                    // Toleransi offline terlewati: Peringatan grace period darurat
                    LicenseInfoResponse {
                        status: LicenseState::GracePeriod,
                        is_valid: true,
                        license_key: Some(payload.license_key),
                        client_name: Some(payload.client_name),
                        tier: Some(payload.tier),
                        max_terminals: payload.max_terminals,
                        current_hardware_id: current_hwid,
                        licensed_hardware_id: Some(payload.hardware_id),
                        expires_at: Some(payload.expires_at),
                        days_remaining: Some(days_remaining),
                        grace_days_remaining: Some((grace_limit_days + 3 - days_since_sync).max(0)),
                        features: payload.features,
                        message: format!("PERINGATAN: Server gagal terhubung ke Provider lisensi selama {} hari (batas toleransi {} hari). Sambungkan koneksi internet sebelum layanan terkunci.", days_since_sync, grace_limit_days),
                        provider_server_url: provider_url,
                        last_sync_at,
                        sync_status: Some("PROVIDER_UNREACHABLE".to_string()),
                        consecutive_sync_failures: Some(consecutive_failures),
                        offline_grace_days_remaining: Some(0),
                    }
                } else {
                    // Melebihi seluruh toleransi: Kunci lisensi untuk mencegah manipulasi hosts file / freeze permanen
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
                        message: format!("Lisensi Terkunci: Server POS tidak dapat memverifikasi lisensi ke Provider selama lebih dari {} hari. Silakan periksa jaringan internet toko Anda.", days_since_sync),
                        provider_server_url: provider_url,
                        last_sync_at,
                        sync_status: Some("PROVIDER_UNREACHABLE".to_string()),
                        consecutive_sync_failures: Some(consecutive_failures),
                        offline_grace_days_remaining: Some(0),
                    }
                }
            } else {
                // Status ONLINE_VERIFIED atau INITIALIZED
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
                    message: format!("Lisensi Aktif Terverifikasi (Sisa {} hari).", days_remaining),
                    provider_server_url: provider_url,
                    last_sync_at,
                    sync_status: Some(sync_status),
                    consecutive_sync_failures: Some(consecutive_failures),
                    offline_grace_days_remaining: Some(grace_limit_days),
                }
            }
        }
    }

    /// Aktivasi lisensi baru ke dalam file vault terenkripsi lokal dan PostgreSQL DB cache
    pub async fn activate_license(
        pool: &PgPool,
        token_json: &str,
        signature_hex: &str,
    ) -> Result<LicenseInfoResponse, String> {
        let payload = Self::verify_signature(token_json, signature_hex)?;
        let current_hwid = Self::get_hardware_id();
        let provider_url = Self::get_provider_url();

        if payload.hardware_id != current_hwid {
            return Err(format!(
                "Lisensi tidak cocok untuk hardware ini. Terdaftar untuk: {}, ID mesin ini: {}",
                payload.hardware_id, current_hwid
            ));
        }

        let now = Utc::now();
        let now_sec = now.timestamp();

        let vault = EncryptedLicenseVault {
            license_token: token_json.to_string(),
            signature: signature_hex.trim().to_string(),
            last_monotonic_time: now_sec,
            activated_at: now,
            last_sync_at: Some(now),
            last_sync_status: Some("ONLINE_VERIFIED".to_string()),
            consecutive_failures: 0,
        };

        // Simpan ke disk & database cache
        Self::save_license_vault_with_db(&vault, Some(pool)).await?;

        // Tandai cache sebagai ONLINE_VERIFIED
        let _ = sqlx::query(
            "UPDATE license_encrypted_cache
             SET last_sync_at = CURRENT_TIMESTAMP,
                 last_sync_status = 'ONLINE_VERIFIED',
                 consecutive_failures = 0,
                 last_failure_reason = NULL,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = 1"
        )
        .execute(pool)
        .await;

        // Catat log audit aktivasi
        let _ = sqlx::query(
            "INSERT INTO license_sync_logs (sync_type, status, http_status, latency_ms, provider_url, message)
             VALUES ('ACTIVATION', 'SUCCESS', 200, 0, $1, $2)"
        )
        .bind(&provider_url)
        .bind(format!("Aktivasi lisensi resmi berhasil untuk {}", payload.client_name))
        .execute(pool)
        .await;

        // Pastikan tabel lama app_licenses dibersihkan jika ada
        let _ = sqlx::query("DROP TABLE IF EXISTS app_licenses CASCADE").execute(pool).await;

        // Otomatis tandai setup awal selesai di receipt_settings saat lisensi berhasil diaktivasi
        let _ = sqlx::query(
            r#"UPDATE receipt_settings
               SET is_initial_setup_completed = TRUE,
                   initial_setup_completed_at = COALESCE(initial_setup_completed_at, CURRENT_TIMESTAMP),
                   updated_at = CURRENT_TIMESTAMP"#
        )
        .execute(pool)
        .await;

        info!("Aktivasi lisensi berhasil: {} ({})", payload.license_key, payload.client_name);
        Ok(Self::check_current_license(pool).await)
    }

    /// Melakukan sinkronisasi lisensi ke Server Provider secara asinkron
    pub async fn sync_with_provider(
        pool: &PgPool,
        sync_type: &str,
    ) -> Result<LicenseSyncResult, String> {
        let current_hwid = Self::get_hardware_id();
        let provider_url = Self::get_provider_url();

        let mut vault = match Self::load_license_vault_with_db(pool).await {
            Ok(Some(v)) => v,
            Ok(None) => return Err("Aplikasi belum memiliki lisensi resmi untuk disinkronkan".to_string()),
            Err(e) => return Err(format!("Gagal membaca lisensi lokal: {}", e)),
        };

        let payload = match Self::verify_signature(&vault.license_token, &vault.signature) {
            Ok(p) => p,
            Err(e) => return Err(format!("Integritas lisensi lokal tidak valid: {}", e)),
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .map_err(|e| format!("Gagal inisialisasi HTTP client: {}", e))?;

        let verify_endpoint = format!("{}/api/v1/license/verify", provider_url);
        let start = std::time::Instant::now();
        let req_body = serde_json::json!({
            "license_key": payload.license_key,
            "hardware_id": current_hwid,
            "app_version": crate::services::update_service::get_app_version()
        });

        match client.post(&verify_endpoint).json(&req_body).send().await {
            Ok(resp) => {
                let latency_ms = start.elapsed().as_millis() as i32;
                let status_code = resp.status().as_u16();

                if resp.status().is_success() {
                    let body: serde_json::Value = resp.json().await.unwrap_or_else(|_| serde_json::json!({}));
                    let status_str = body.get("status").and_then(|v| v.as_str()).unwrap_or("ACTIVE");

                    if status_str == "REVOKED" || status_str == "SUSPENDED" {
                        vault.last_sync_status = Some("REVOKED".to_string());
                        let _ = Self::save_license_vault_with_db(&vault, Some(pool)).await;

                        let _ = sqlx::query(
                            "UPDATE license_encrypted_cache
                             SET last_sync_status = 'REVOKED', last_sync_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
                             WHERE id = 1"
                        ).execute(pool).await;

                        let _ = sqlx::query(
                            "INSERT INTO license_sync_logs (sync_type, status, http_status, latency_ms, provider_url, message)
                             VALUES ($1, 'REVOKED', $2, $3, $4, 'Lisensi ditandai REVOKED/SUSPENDED oleh provider')"
                        )
                        .bind(sync_type)
                        .bind(status_code as i32)
                        .bind(latency_ms)
                        .bind(&provider_url)
                        .execute(pool).await;

                        let lic_info = Self::check_current_license(pool).await;
                        return Ok(LicenseSyncResult {
                            success: false,
                            status: "REVOKED".to_string(),
                            http_status: Some(status_code),
                            latency_ms: Some(latency_ms as u128),
                            message: "Lisensi telah dicabut atau dinonaktifkan oleh server provider".to_string(),
                            provider_url,
                            license_info: lic_info,
                        });
                    }

                    // Sinkronisasi sukses!
                    vault.last_sync_at = Some(Utc::now());
                    vault.last_sync_status = Some("ONLINE_VERIFIED".to_string());
                    vault.consecutive_failures = 0;
                    let _ = Self::save_license_vault_with_db(&vault, Some(pool)).await;

                    let _ = sqlx::query(
                        "UPDATE license_encrypted_cache
                         SET last_sync_at = CURRENT_TIMESTAMP,
                             last_sync_status = 'ONLINE_VERIFIED',
                             consecutive_failures = 0,
                             last_failure_reason = NULL,
                             updated_at = CURRENT_TIMESTAMP
                         WHERE id = 1"
                    ).execute(pool).await;

                    let _ = sqlx::query(
                        "INSERT INTO license_sync_logs (sync_type, status, http_status, latency_ms, provider_url, message)
                         VALUES ($1, 'SUCCESS', $2, $3, $4, 'Verifikasi berkala lisensi provider berhasil')"
                    )
                    .bind(sync_type)
                    .bind(status_code as i32)
                    .bind(latency_ms)
                    .bind(&provider_url)
                    .execute(pool).await;

                    let lic_info = Self::check_current_license(pool).await;
                    info!("[LICENSE SYNC] Sinkronisasi lisensi ke provider ({}) berhasil ({} ms)", provider_url, latency_ms);

                    Ok(LicenseSyncResult {
                        success: true,
                        status: "ONLINE_VERIFIED".to_string(),
                        http_status: Some(status_code),
                        latency_ms: Some(latency_ms as u128),
                        message: format!("Sinkronisasi lisensi resmi berhasil ({} ms)", latency_ms),
                        provider_url,
                        license_info: lic_info,
                    })
                } else if resp.status() == reqwest::StatusCode::NOT_FOUND || resp.status() == reqwest::StatusCode::FORBIDDEN {
                    // Provider menjawab 404 atau 403 (Lisensi tidak terdaftar)
                    vault.last_sync_status = Some("REVOKED".to_string());
                    let _ = Self::save_license_vault_with_db(&vault, Some(pool)).await;

                    let _ = sqlx::query(
                        "UPDATE license_encrypted_cache
                         SET last_sync_status = 'REVOKED', last_sync_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
                         WHERE id = 1"
                    ).execute(pool).await;

                    let _ = sqlx::query(
                        "INSERT INTO license_sync_logs (sync_type, status, http_status, latency_ms, provider_url, message)
                         VALUES ($1, 'REVOKED', $2, $3, $4, 'Lisensi ditolak oleh server provider (404/403)')"
                    )
                    .bind(sync_type)
                    .bind(status_code as i32)
                    .bind(latency_ms)
                    .bind(&provider_url)
                    .execute(pool).await;

                    let lic_info = Self::check_current_license(pool).await;
                    Ok(LicenseSyncResult {
                        success: false,
                        status: "REVOKED".to_string(),
                        http_status: Some(status_code),
                        latency_ms: Some(latency_ms as u128),
                        message: "Lisensi ditolak oleh server provider (Kunci tidak valid atau telah dihapus)".to_string(),
                        provider_url,
                        license_info: lic_info,
                    })
                } else {
                    // Server merespons 500/502/503 dll (Server provider bermasalah/down sementara)
                    vault.last_sync_status = Some("PROVIDER_UNREACHABLE".to_string());
                    vault.consecutive_failures += 1;
                    let _ = Self::save_license_vault_with_db(&vault, Some(pool)).await;

                    let err_msg = format!("Server provider merespons HTTP {}", resp.status());
                    let _ = sqlx::query(
                        "UPDATE license_encrypted_cache
                         SET last_sync_status = 'PROVIDER_UNREACHABLE',
                             consecutive_failures = consecutive_failures + 1,
                             last_failed_sync_at = CURRENT_TIMESTAMP,
                             last_failure_reason = $1,
                             updated_at = CURRENT_TIMESTAMP
                         WHERE id = 1"
                    )
                    .bind(&err_msg)
                    .execute(pool).await;

                    let _ = sqlx::query(
                        "INSERT INTO license_sync_logs (sync_type, status, http_status, latency_ms, provider_url, message)
                         VALUES ($1, 'UNREACHABLE', $2, $3, $4, $5)"
                    )
                    .bind(sync_type)
                    .bind(status_code as i32)
                    .bind(latency_ms)
                    .bind(&provider_url)
                    .bind(&err_msg)
                    .execute(pool).await;

                    let lic_info = Self::check_current_license(pool).await;
                    warn!("[LICENSE SYNC] Server provider down sementara (HTTP {}). Lisensi tetap aktif dalam mode toleransi offline.", status_code);

                    Ok(LicenseSyncResult {
                        success: false,
                        status: "PROVIDER_UNREACHABLE".to_string(),
                        http_status: Some(status_code),
                        latency_ms: Some(latency_ms as u128),
                        message: format!("Server provider merespons HTTP {}. Sistem beralih ke toleransi mode offline.", status_code),
                        provider_url,
                        license_info: lic_info,
                    })
                }
            }
            Err(e) => {
                let err_detail = if e.is_timeout() {
                    "Koneksi timeout (server provider tidak merespons dalam 6 detik)".to_string()
                } else if e.is_connect() {
                    format!("Gagal terhubung ke {} (Connection refused / Host unreachable)", provider_url)
                } else {
                    format!("Gangguan koneksi ke provider: {}", e)
                };

                vault.last_sync_status = Some("PROVIDER_UNREACHABLE".to_string());
                vault.consecutive_failures += 1;
                let _ = Self::save_license_vault_with_db(&vault, Some(pool)).await;

                let _ = sqlx::query(
                    "UPDATE license_encrypted_cache
                     SET last_sync_status = 'PROVIDER_UNREACHABLE',
                         consecutive_failures = consecutive_failures + 1,
                         last_failed_sync_at = CURRENT_TIMESTAMP,
                         last_failure_reason = $1,
                         updated_at = CURRENT_TIMESTAMP
                     WHERE id = 1"
                )
                .bind(&err_detail)
                .execute(pool).await;

                let _ = sqlx::query(
                    "INSERT INTO license_sync_logs (sync_type, status, http_status, latency_ms, provider_url, message)
                     VALUES ($1, 'UNREACHABLE', NULL, NULL, $2, $3)"
                )
                .bind(sync_type)
                .bind(&provider_url)
                .bind(&err_detail)
                .execute(pool).await;

                let lic_info = Self::check_current_license(pool).await;
                warn!("[LICENSE SYNC] Provider tidak terjangkau ({}). Lisensi tetap aktif dalam toleransi mode offline.", err_detail);

                Ok(LicenseSyncResult {
                    success: false,
                    status: "PROVIDER_UNREACHABLE".to_string(),
                    http_status: None,
                    latency_ms: None,
                    message: "Server provider lisensi sedang tidak dapat dijangkau. Lisensi toko Anda tetap aktif aman dalam mode offline.".to_string(),
                    provider_url,
                    license_info: lic_info,
                })
            }
        }
    }

    /// Menjalankan background task scheduler untuk sinkronisasi lisensi berkala setiap 6 jam
    pub fn start_license_sync_scheduler(pool: PgPool) {
        static SCHEDULER_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if SCHEDULER_RUNNING.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return;
        }

        tokio::spawn(async move {
            // Berikan jeda 15 detik setelah boot agar inisialisasi database dan socket OS stabil
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

            info!("[LICENSE SCHEDULER] Menjalankan verifikasi awal status lisensi dengan provider...");
            let _ = Self::sync_with_provider(&pool, "BOOT_PING").await;

            // Interval berkala setiap 6 jam
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(6 * 3600));
            loop {
                interval.tick().await;
                info!("[LICENSE SCHEDULER] Menjalankan sinkronisasi berkala (6 jam) ke server provider...");
                let _ = Self::sync_with_provider(&pool, "SCHEDULED_PING").await;
            }
        });
    }

    /// Mengambil riwayat log sinkronisasi lisensi provider
    pub async fn get_sync_logs(pool: &PgPool, limit: i64) -> Result<Vec<LicenseSyncLogItem>, String> {
        let safe_limit = limit.clamp(1, 100);
        let rows = sqlx::query_as::<_, LicenseSyncLogItem>(
            "SELECT id, sync_type, status, http_status, latency_ms, provider_url, message, created_at
             FROM license_sync_logs
             ORDER BY id DESC
             LIMIT $1"
        )
        .bind(safe_limit)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal mengambil riwayat sinkronisasi lisensi: {}", e))?;

        Ok(rows)
    }

    /// Memeriksa apakah suatu fitur modul aktif pada lisensi server ini
    pub async fn has_feature(pool: &PgPool, feature_name: &str) -> bool {
        let lic = Self::check_current_license(pool).await;
        if !lic.is_valid {
            return false;
        }
        lic.features.iter().any(|f| f == "*" || f.eq_ignore_ascii_case("all") || f.eq_ignore_ascii_case(feature_name))
    }
}
