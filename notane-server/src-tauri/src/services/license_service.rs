use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::convert::TryInto;
use std::env;
use std::path::PathBuf;
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

/// Struktur file vault lisensi terenkripsi lokal (Zero Database Storage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedLicenseVault {
    pub license_token: String,
    pub signature: String,
    pub last_monotonic_time: i64,
    pub activated_at: DateTime<Utc>,
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
    pub const BAKED_PRODUCTION_PROVIDER_URL: Option<&'static str> = option_env!("NOTANE_RELEASE_PROVIDER_URL");

    /// Mengambil URL Provider Server dengan urutan prioritas runtime/baked
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

    /// Menyimpan data lisensi terenkripsi ke file vault
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

        // Simpan juga ke direktori project ./.license_vault.enc jika berbeda dan dapat ditulis
        let local_path = PathBuf::from(".license_vault.enc");
        if local_path != primary_path {
            let _ = std::fs::write(&local_path, &payload);
        }

        info!("File vault lisensi berhasil disimpan dan dienkripsi (AES-256-GCM + Hardware Lock)");
        Ok(())
    }

    /// Membaca dan mendekripsi data lisensi dari file vault
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

    /// Memeriksa status lisensi dari file vault terenkripsi & mendeteksi manipulasi jam (Anti-Clock Rollback)
    pub async fn check_current_license(pool: &PgPool) -> LicenseInfoResponse {
        let current_hwid = Self::get_hardware_id();
        let provider_url = Self::get_provider_url();

        // 1. Coba baca dari file vault terenkripsi
        let vault_opt = match Self::load_license_vault() {
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
                };
            }
        };

        // 2. Jika belum ada di file vault, periksa apakah ada migrasi sisa dari tabel database lama
        let mut vault = match vault_opt {
            Some(v) => v,
            None => {
                // Coba migrasikan satu kali jika tabel app_licenses masih ada di DB lama
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
                    };

                    if let Err(e) = Self::save_license_vault(&legacy_vault) {
                        warn!("Gagal menyimpan migrasi lisensi ke file terenkripsi: {}", e);
                    } else {
                        info!("Berhasil memigrasikan lisensi lama dari database ke file vault terenkripsi.");
                    }

                    // Hapus tabel lama dari database untuk membersihkan
                    let _ = sqlx::query("DROP TABLE IF EXISTS app_licenses CASCADE").execute(pool).await;

                    legacy_vault
                } else {
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
            }
        };

        // 3. Verifikasi tanda tangan kriptografi Ed25519
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
                };
            }
        };

        // 4. Node Locking: Validasi kecocokan Hardware ID
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

        // 5. Anti-Clock-Rollback: Periksa apakah jam sistem dimundurkan oleh pengguna
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
            };
        }

        // Perbarui monotonic time check jika waktu berjalan maju (> 60 detik)
        if now_sec > vault.last_monotonic_time + 60 {
            vault.last_monotonic_time = now_sec;
            let _ = Self::save_license_vault(&vault);
        }

        // 6. Evaluasi Tanggal Kedaluwarsa & Masa Toleransi Offline (Offline Grace Period)
        let diff = payload.expires_at.signed_duration_since(now);
        let days_remaining = diff.num_days();

        if days_remaining >= 0 {
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
            let grace_limit = payload.expires_at + chrono::Duration::days(payload.offline_grace_days as i64);
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

    /// Aktivasi lisensi baru ke dalam file vault terenkripsi lokal
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

        let vault = EncryptedLicenseVault {
            license_token: token_json.to_string(),
            signature: signature_hex.trim().to_string(),
            last_monotonic_time: now_sec,
            activated_at: now,
        };

        // Simpan ke file terenkripsi lokal
        Self::save_license_vault(&vault)?;

        // Pastikan tabel app_licenses di database benar-benar dibersihkan / di-drop
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
}

