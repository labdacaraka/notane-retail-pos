use sha2::{Digest, Sha256};

/// Menghasilkan hash kata sandi menggunakan SHA-256 dan 16-byte random salt
pub fn hash_secret(secret: &str) -> String {
    use rand::RngCore;
    let mut salt_bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut salt_bytes);
    let salt_hex = hex::encode(salt_bytes);

    let mut hasher = Sha256::new();
    hasher.update(b"NOTANE_MANAGER_AUTH_SALT_2026::");
    hasher.update(salt_hex.as_bytes());
    hasher.update(b"::");
    hasher.update(secret.as_bytes());
    let hash_hex = hex::encode(hasher.finalize());

    format!("{}${}", salt_hex, hash_hex)
}

/// Memverifikasi kata sandi terhadap hash tersimpan
pub fn verify_secret(secret: &str, stored_hash: &str) -> bool {
    let parts: Vec<&str> = stored_hash.split('$').collect();
    if parts.len() != 2 {
        return false;
    }

    let salt_hex = parts[0];
    let expected_hash = parts[1];

    let mut hasher = Sha256::new();
    hasher.update(b"NOTANE_MANAGER_AUTH_SALT_2026::");
    hasher.update(salt_hex.as_bytes());
    hasher.update(b"::");
    hasher.update(secret.as_bytes());
    let actual_hash = hex::encode(hasher.finalize());

    actual_hash == expected_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let password = "admin_secure_pass";
        let hash = hash_secret(password);
        assert!(verify_secret(password, &hash));
        assert!(!verify_secret("wrong_pass", &hash));
    }
}
