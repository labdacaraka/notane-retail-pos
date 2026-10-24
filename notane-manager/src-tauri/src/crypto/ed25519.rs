use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::convert::TryInto;

/// Master Private Key Hex default untuk Notane License Provider
pub const DEFAULT_MASTER_PRIVATE_KEY_HEX: &str =
    "40c97328401a31ed63b6ecd07bf1ea35a731e775cab79c730582b49069b0a5b9";

/// Master Public Key Hex default yang tersemat pada aplikasi Client & Server Notane
pub const DEFAULT_MASTER_PUBLIC_KEY_HEX: &str =
    "779b7d3fb80a25e9503bf7b85ffbd226ed9a4b8ff9de844872dd3a19357bb213";

/// Tanda tangani pesan menggunakan Private Key Ed25519, menghasilkan 64-byte Hex signature
pub fn sign_message(private_key_hex: &str, message: &str) -> Result<String, String> {
    let clean_hex = private_key_hex.trim();
    let priv_bytes = hex::decode(clean_hex)
        .map_err(|e| format!("Format private key hex tidak valid: {}", e))?;

    let priv_arr: [u8; 32] = priv_bytes
        .try_into()
        .map_err(|_| "Panjang private key Ed25519 harus tepat 32 bytes (64 karakter hex)".to_string())?;

    let signing_key = SigningKey::from_bytes(&priv_arr);
    let signature: Signature = signing_key.sign(message.as_bytes());

    Ok(hex::encode(signature.to_bytes()))
}

/// Verifikasi signature pesan menggunakan Public Key Ed25519
pub fn verify_signature(public_key_hex: &str, message: &str, signature_hex: &str) -> Result<bool, String> {
    let clean_pub_hex = public_key_hex.trim();
    let pub_bytes = hex::decode(clean_pub_hex)
        .map_err(|e| format!("Format public key hex tidak valid: {}", e))?;

    let pub_arr: [u8; 32] = pub_bytes
        .try_into()
        .map_err(|_| "Panjang public key Ed25519 harus tepat 32 bytes".to_string())?;

    let verifying_key = VerifyingKey::from_bytes(&pub_arr)
        .map_err(|e| format!("VerifyingKey Ed25519 tidak valid: {}", e))?;

    let sig_bytes = hex::decode(signature_hex.trim())
        .map_err(|e| format!("Format signature hex tidak valid: {}", e))?;

    let sig_arr: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| "Panjang signature Ed25519 harus tepat 64 bytes".to_string())?;

    let signature = Signature::from_bytes(&sig_arr);

    match verifying_key.verify(message.as_bytes(), &signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Hitung hash SHA-256 dari data byte
pub fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Ambil public key pasangan dari private key
pub fn derive_public_key(private_key_hex: &str) -> Result<String, String> {
    let clean_hex = private_key_hex.trim();
    let priv_bytes = hex::decode(clean_hex)
        .map_err(|e| format!("Format private key hex tidak valid: {}", e))?;

    let priv_arr: [u8; 32] = priv_bytes
        .try_into()
        .map_err(|_| "Panjang private key Ed25519 harus tepat 32 bytes".to_string())?;

    let signing_key = SigningKey::from_bytes(&priv_arr);
    let verifying_key = signing_key.verifying_key();
    Ok(hex::encode(verifying_key.to_bytes()))
}

/// Buat pasangan kunci baru (Keypair Generation)
pub fn generate_new_keypair() -> (String, String) {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let signing_key = SigningKey::from_bytes(&bytes);
    let verifying_key = signing_key.verifying_key();
    (
        hex::encode(signing_key.to_bytes()),
        hex::encode(verifying_key.to_bytes()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_keypair_match() {
        let derived_pub = derive_public_key(DEFAULT_MASTER_PRIVATE_KEY_HEX).unwrap();
        assert_eq!(derived_pub, DEFAULT_MASTER_PUBLIC_KEY_HEX);

        let msg = "test_license_payload";
        let sig = sign_message(DEFAULT_MASTER_PRIVATE_KEY_HEX, msg).unwrap();
        assert_eq!(sig.len(), 128);
        let ok = verify_signature(DEFAULT_MASTER_PUBLIC_KEY_HEX, msg, &sig).unwrap();
        assert!(ok);
    }
}
