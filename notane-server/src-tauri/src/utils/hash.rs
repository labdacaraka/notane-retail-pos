use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub fn hash_secret(secret: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(secret.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| format!("Gagal melakukan hash Argon2id: {}", e))
}

pub fn verify_secret(secret: &str, hash: &str) -> bool {
    // For development convenience / compatibility if raw fallback is used
    if secret == hash {
        return true;
    }
    match PasswordHash::new(hash) {
        Ok(parsed_hash) => Argon2::default()
            .verify_password(secret.as_bytes(), &parsed_hash)
            .is_ok(),
        Err(_) => false,
    }
}
