use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const OTA_SALT_PREFIX: &str = "NOTANE_OTA_ENCRYPTION_KEY_V1:";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtaDownloadPayload {
    pub license_key: String,
    pub machine_id: String,
    pub timestamp: i64,
    pub component: Option<String>,
    pub current_version: Option<String>,
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
}

pub fn derive_ota_key(master_pub_key_hex: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(OTA_SALT_PREFIX.as_bytes());
    hasher.update(master_pub_key_hex.trim().as_bytes());
    hasher.finalize().into()
}

pub fn encrypt_ota_payload(payload: &OtaDownloadPayload, master_pub_key_hex: &str) -> Result<String, String> {
    let key_bytes = derive_ota_key(master_pub_key_hex);
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| format!("Gagal inisialisasi cipher AES-256-GCM: {}", e))?;

    let json_bytes = serde_json::to_vec(payload)
        .map_err(|e| format!("Gagal serialisasi payload OTA: {}", e))?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, json_bytes.as_ref())
        .map_err(|e| format!("Gagal mengenkripsi token OTA: {}", e))?;

    let mut result = Vec::with_capacity(12 + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    Ok(hex::encode(result))
}

pub fn decrypt_ota_payload(token_hex: &str, master_pub_key_hex: &str) -> Result<OtaDownloadPayload, String> {
    let raw_bytes = hex::decode(token_hex.trim())
        .map_err(|e| format!("Token OTA hex invalid: {}", e))?;

    if raw_bytes.len() < 12 + 16 {
        return Err("Ukuran token OTA terlalu pendek atau rusak".to_string());
    }

    let (nonce_slice, ciphertext_slice) = raw_bytes.split_at(12);
    let key_bytes = derive_ota_key(master_pub_key_hex);
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| format!("Gagal inisialisasi cipher AES-256-GCM: {}", e))?;

    let nonce = Nonce::from_slice(nonce_slice);
    let decrypted_bytes = cipher
        .decrypt(nonce, ciphertext_slice)
        .map_err(|_| "Gagal mendekripsi token OTA: integritas data atau kunci tidak cocok".to_string())?;

    let payload: OtaDownloadPayload = serde_json::from_slice(&decrypted_bytes)
        .map_err(|e| format!("Format JSON payload OTA tidak valid: {}", e))?;

    Ok(payload)
}

