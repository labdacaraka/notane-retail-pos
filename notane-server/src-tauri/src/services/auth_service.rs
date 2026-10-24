use crate::models::user::{
    EnrollBiometricRequest, LoginRequest, LoginResponse, Role, SupervisorBiometricApproveRequest,
    SupervisorPinRequest, SupervisorPinResponse, User, UserProfile,
};
use crate::utils::hash::verify_secret;
use crate::utils::jwt::create_jwt;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn authenticate_user(
    pool: &PgPool,
    req: LoginRequest,
) -> Result<LoginResponse, String> {
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE username = $1 AND is_active = true",
    )
    .bind(&req.username)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Username atau password salah".to_string())?;

    if !verify_secret(&req.password, &user.password_hash) {
        return Err("Username atau password salah".to_string());
    }

    let role = sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE id = $1")
        .bind(user.role_id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Gagal memuat peran pengguna: {}", e))?;

    let permissions = sqlx::query_scalar::<_, String>(
        "SELECT p.code FROM permissions p 
         JOIN role_permissions rp ON p.id = rp.permission_id 
         WHERE rp.role_id = $1",
    )
    .bind(user.role_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Gagal memuat permission: {}", e))?;

    let token = create_jwt(
        user.id,
        &user.username,
        user.role_id,
        &role.name,
        req.terminal_id.clone(),
        12, // 12 hours shift token
    )?;

    let outlet_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT outlet_id FROM user_outlets WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let profile = UserProfile {
        id: user.id,
        role_id: user.role_id,
        role_name: role.name,
        username: user.username,
        full_name: user.full_name,
        email: user.email,
        phone: user.phone,
        permissions,
        outlet_ids,
        has_biometric: user.has_biometric,
        has_face_recognition: user.has_face_recognition,
        face_photo_preview: user.face_photo_preview,
    };

    Ok(LoginResponse {
        token,
        user: profile,
    })
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct SupervisorItem {
    pub id: Uuid,
    pub username: String,
    pub full_name: String,
    pub role_name: String,
    pub has_biometric: bool,
    pub has_face_recognition: bool,
    pub face_photo_preview: Option<String>,
    pub biometric_credential_id: Option<String>,
}

pub async fn list_supervisors(
    pool: &PgPool,
    outlet_id: Option<Uuid>,
    search: Option<String>,
) -> Result<Vec<SupervisorItem>, String> {
    let search_clean = search
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s.to_lowercase()));

    if let Some(oid) = outlet_id {
        let rows = if let Some(pattern) = search_clean {
            sqlx::query_as::<_, SupervisorItem>(
                "SELECT DISTINCT u.id, u.username, u.full_name, r.name as role_name,
                        COALESCE(u.has_biometric, false) as has_biometric,
                        COALESCE(u.has_face_recognition, false) as has_face_recognition,
                        u.face_photo_preview,
                        u.biometric_credential_id
                 FROM users u 
                 JOIN roles r ON u.role_id = r.id 
                 LEFT JOIN outlet_supervisors os ON os.user_id = u.id AND os.outlet_id = $1
                 LEFT JOIN outlets o ON o.id = $1 AND o.manager_id = u.id
                 WHERE u.is_active = true 
                   AND (os.outlet_id IS NOT NULL OR o.id IS NOT NULL OR r.name = 'SUPERADMIN')
                   AND (LOWER(u.full_name) LIKE $2 OR LOWER(u.username) LIKE $2)
                 ORDER BY u.full_name ASC",
            )
            .bind(oid)
            .bind(pattern)
            .fetch_all(pool)
            .await
        } else {
            sqlx::query_as::<_, SupervisorItem>(
                "SELECT DISTINCT u.id, u.username, u.full_name, r.name as role_name,
                        COALESCE(u.has_biometric, false) as has_biometric,
                        COALESCE(u.has_face_recognition, false) as has_face_recognition,
                        u.face_photo_preview,
                        u.biometric_credential_id
                 FROM users u 
                 JOIN roles r ON u.role_id = r.id 
                 LEFT JOIN outlet_supervisors os ON os.user_id = u.id AND os.outlet_id = $1
                 LEFT JOIN outlets o ON o.id = $1 AND o.manager_id = u.id
                 WHERE u.is_active = true 
                   AND (os.outlet_id IS NOT NULL OR o.id IS NOT NULL OR r.name = 'SUPERADMIN')
                 ORDER BY u.full_name ASC",
            )
            .bind(oid)
            .fetch_all(pool)
            .await
        };

        return rows.map_err(|e| format!("Database error: {}", e));
    }

    // Default / Fallback: Ambil semua supervisor toko & superadmin
    let rows = if let Some(pattern) = search_clean {
        sqlx::query_as::<_, SupervisorItem>(
            "SELECT DISTINCT u.id, u.username, u.full_name, r.name as role_name,
                    COALESCE(u.has_biometric, false) as has_biometric,
                    COALESCE(u.has_face_recognition, false) as has_face_recognition,
                    u.face_photo_preview,
                    u.biometric_credential_id
             FROM users u 
             JOIN roles r ON u.role_id = r.id 
             LEFT JOIN role_permissions rp ON r.id = rp.role_id 
             LEFT JOIN permissions p ON rp.permission_id = p.id 
             WHERE (p.code IN ('auth:supervisor', 'auth:store_manager') OR r.name IN ('SUPERVISOR', 'STORE_MANAGER', 'SUPERADMIN')) 
               AND u.is_active = true 
               AND (LOWER(u.full_name) LIKE $1 OR LOWER(u.username) LIKE $1)
             ORDER BY u.full_name ASC",
        )
        .bind(pattern)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, SupervisorItem>(
            "SELECT DISTINCT u.id, u.username, u.full_name, r.name as role_name,
                    COALESCE(u.has_biometric, false) as has_biometric,
                    COALESCE(u.has_face_recognition, false) as has_face_recognition,
                    u.face_photo_preview,
                    u.biometric_credential_id
             FROM users u 
             JOIN roles r ON u.role_id = r.id 
             LEFT JOIN role_permissions rp ON r.id = rp.role_id 
             LEFT JOIN permissions p ON rp.permission_id = p.id 
             WHERE (p.code IN ('auth:supervisor', 'auth:store_manager') OR r.name IN ('SUPERVISOR', 'STORE_MANAGER', 'SUPERADMIN')) 
               AND u.is_active = true 
             ORDER BY u.full_name ASC",
        )
        .fetch_all(pool)
        .await
    };

    rows.map_err(|e| format!("Database error: {}", e))
}

pub async fn list_store_managers(pool: &PgPool) -> Result<Vec<SupervisorItem>, String> {
    let rows = sqlx::query_as::<_, SupervisorItem>(
        "SELECT DISTINCT u.id, u.username, u.full_name, r.name as role_name,
                COALESCE(u.has_biometric, false) as has_biometric,
                COALESCE(u.has_face_recognition, false) as has_face_recognition,
                u.face_photo_preview,
                u.biometric_credential_id
         FROM users u 
         JOIN roles r ON u.role_id = r.id 
         LEFT JOIN role_permissions rp ON r.id = rp.role_id 
         LEFT JOIN permissions p ON rp.permission_id = p.id 
         WHERE (p.code = 'auth:store_manager' OR r.name IN ('STORE_MANAGER', 'SUPERADMIN')) 
           AND u.is_active = true 
         ORDER BY u.full_name ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    Ok(rows)
}

pub async fn verify_supervisor_pin(
    pool: &PgPool,
    req: SupervisorPinRequest,
) -> Result<SupervisorPinResponse, String> {
    if let Some(spv_id) = req.supervisor_id {
        let spv = sqlx::query_as::<_, User>(
            "SELECT DISTINCT u.* FROM users u 
             JOIN roles r ON u.role_id = r.id 
             LEFT JOIN role_permissions rp ON r.id = rp.role_id 
             LEFT JOIN permissions p ON rp.permission_id = p.id 
             WHERE u.id = $1 
               AND (p.code IN ('auth:supervisor', 'auth:store_manager') OR r.name IN ('SUPERVISOR', 'STORE_MANAGER', 'SUPERADMIN')) 
               AND u.is_active = true 
             LIMIT 1",
        )
        .bind(spv_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Supervisor tidak ditemukan atau tidak memiliki wewenang otorisasi".to_string())?;

        if verify_secret(&req.pin, &spv.pin_hash) {
            let role_name = sqlx::query_scalar::<_, String>("SELECT name FROM roles WHERE id = $1")
                .bind(spv.role_id)
                .fetch_optional(pool)
                .await
                .unwrap_or(None)
                .unwrap_or_else(|| "SUPERVISOR".to_string());
            let auth_token = create_jwt(spv.id, &spv.username, spv.role_id, &role_name, None, 2).ok();

            return Ok(SupervisorPinResponse {
                success: true,
                supervisor_id: spv.id,
                supervisor_name: spv.full_name,
                message: "Otorisasi Supervisor Berhasil".to_string(),
                auth_token,
            });
        } else {
            return Err(format!("PIN untuk supervisor {} tidak valid", spv.full_name));
        }
    }

    // Fallback jika supervisor_id tidak ditentukan secara eksplisit
    let supervisors = sqlx::query_as::<_, User>(
        "SELECT DISTINCT u.* FROM users u 
         JOIN roles r ON u.role_id = r.id 
         LEFT JOIN role_permissions rp ON r.id = rp.role_id 
         LEFT JOIN permissions p ON rp.permission_id = p.id 
         WHERE (p.code IN ('auth:supervisor', 'auth:store_manager') OR r.name IN ('SUPERVISOR', 'STORE_MANAGER', 'SUPERADMIN')) 
           AND u.is_active = true",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    for spv in supervisors {
        if verify_secret(&req.pin, &spv.pin_hash) {
            let role_name = sqlx::query_scalar::<_, String>("SELECT name FROM roles WHERE id = $1")
                .bind(spv.role_id)
                .fetch_optional(pool)
                .await
                .unwrap_or(None)
                .unwrap_or_else(|| "SUPERVISOR".to_string());
            let auth_token = create_jwt(spv.id, &spv.username, spv.role_id, &role_name, None, 2).ok();

            return Ok(SupervisorPinResponse {
                success: true,
                supervisor_id: spv.id,
                supervisor_name: spv.full_name,
                message: "Otorisasi Supervisor Berhasil".to_string(),
                auth_token,
            });
        }
    }

    Err("PIN Supervisor tidak valid atau tidak memiliki wewenang".to_string())
}

pub async fn verify_supervisor_biometric(
    pool: &PgPool,
    req: SupervisorBiometricApproveRequest,
) -> Result<SupervisorPinResponse, String> {
    // 1. Ambil setting biometrik aktif dari receipt_settings
    let (bio_mode, _bio_policy, face_threshold) = sqlx::query_as::<_, (String, String, rust_decimal::Decimal)>(
        "SELECT supervisor_biometric_mode, supervisor_biometric_policy, face_match_threshold 
         FROM receipt_settings ORDER BY created_at ASC LIMIT 1"
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None)
    .unwrap_or_else(|| ("ANY".to_string(), "OPTIONAL".to_string(), rust_decimal::Decimal::from(75)));

    if bio_mode == "OFF" {
        return Err("Autentikasi biometrik supervisor dinonaktifkan di pengaturan sistem toko".to_string());
    }

    if bio_mode == "FINGERPRINT" && req.biometric_type != "FINGERPRINT" {
        return Err("Sistem saat ini hanya mengizinkan biometrik sidik jari untuk supervisor".to_string());
    }

    if bio_mode == "FACE_RECOGNITION" && req.biometric_type != "FACE" {
        return Err("Sistem saat ini hanya mengizinkan pemindai wajah untuk supervisor".to_string());
    }

    if let Some(spv_id) = req.supervisor_id {
        let spv = sqlx::query_as::<_, User>(
            "SELECT DISTINCT u.* FROM users u 
             JOIN roles r ON u.role_id = r.id 
             LEFT JOIN role_permissions rp ON r.id = rp.role_id 
             LEFT JOIN permissions p ON rp.permission_id = p.id 
             WHERE u.id = $1 
               AND (p.code IN ('auth:supervisor', 'auth:store_manager') OR r.name IN ('SUPERVISOR', 'STORE_MANAGER', 'SUPERADMIN')) 
               AND u.is_active = true 
             LIMIT 1",
        )
        .bind(spv_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Supervisor tidak ditemukan atau tidak memiliki wewenang otorisasi".to_string())?;

        match req.biometric_type.as_str() {
            "FINGERPRINT" => {
                if !spv.has_biometric {
                    return Err(format!("Supervisor {} belum mendaftarkan sensor sidik jari", spv.full_name));
                }
                if let Some(ref cred_id) = req.credential_id {
                    if let Some(ref registered_cred) = spv.biometric_credential_id {
                        if cred_id != registered_cred {
                            return Err("Kredensial sidik jari tidak cocok dengan supervisor terdaftar".to_string());
                        }
                    }
                }
            }
            "FACE" => {
                if !spv.has_face_recognition {
                    return Err(format!("Supervisor {} belum mendaftarkan pemindai wajah", spv.full_name));
                }

                let enrolled_desc_str = spv.face_descriptor.as_deref().ok_or_else(|| {
                    format!("Template biometrik wajah supervisor {} tidak ditemukan di sistem", spv.full_name)
                })?;

                let live_desc_str = req.live_face_descriptor.as_deref().ok_or_else(|| {
                    "Data pemindaian wajah langsung (live face descriptor) wajib disertakan oleh kamera terminal".to_string()
                })?;

                // Parse vector deskriptor wajah (JSON Array of f64)
                let enrolled_vec: Vec<f64> = serde_json::from_str(enrolled_desc_str)
                    .map_err(|_| "Format template wajah terdaftar rusak di database".to_string())?;

                let live_vec: Vec<f64> = serde_json::from_str(live_desc_str)
                    .map_err(|_| "Format live face descriptor tidak valid".to_string())?;

                if enrolled_vec.is_empty() || live_vec.is_empty() || enrolled_vec.len() != live_vec.len() {
                    return Err("Dimensi tensor biometrik wajah tidak cocok dengan data pendaftaran".to_string());
                }

                // Hitung Cosine Similarity secara aman di server (Zero-Trust)
                let mut dot_product = 0.0;
                let mut norm_a = 0.0;
                let mut norm_b = 0.0;

                for (a, b) in live_vec.iter().zip(enrolled_vec.iter()) {
                    dot_product += a * b;
                    norm_a += a * a;
                    norm_b += b * b;
                }

                if norm_a <= 0.0 || norm_b <= 0.0 {
                    return Err("Vektor biometrik wajah kosong atau tidak memiliki kontras yang valid".to_string());
                }

                let cosine = dot_product / (norm_a.sqrt() * norm_b.sqrt());
                // Normalisasi cosine similarity (-1.0..1.0 atau 0.5..1.0) ke skala persentase 0..100%
                let calculated_similarity = (((cosine - 0.5).max(0.0) / 0.5) * 100.0).clamp(0.0, 100.0);

                let threshold_f64 = face_threshold.to_string().parse::<f64>().unwrap_or(75.0);
                if calculated_similarity < threshold_f64 {
                    return Err(format!(
                        "Pencocokan biometrik wajah gagal oleh server. Kemiripan terhitung ({:.1}%) di bawah standar keamanan toko ({:.1}%)",
                        calculated_similarity, threshold_f64
                    ));
                }
            }
            _ => return Err("Jenis biometrik tidak dikenali".to_string()),
        }

        let role_name = sqlx::query_scalar::<_, String>("SELECT name FROM roles WHERE id = $1")
            .bind(spv.role_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None)
            .unwrap_or_else(|| "SUPERVISOR".to_string());
        let auth_token = create_jwt(spv.id, &spv.username, spv.role_id, &role_name, None, 2).ok();

        return Ok(SupervisorPinResponse {
            success: true,
            supervisor_id: spv.id,
            supervisor_name: spv.full_name,
            message: format!("Otorisasi Biometrik ({}) Berhasil", req.biometric_type),
            auth_token,
        });
    }

    // Lookup otomatis berdasarkan credential_id (Touch ID / WebAuthn)
    if req.biometric_type == "FINGERPRINT" {
        if let Some(ref cred_id) = req.credential_id {
            let spv = sqlx::query_as::<_, User>(
                "SELECT DISTINCT u.* FROM users u 
                 JOIN roles r ON u.role_id = r.id 
                 LEFT JOIN role_permissions rp ON r.id = rp.role_id 
                 LEFT JOIN permissions p ON rp.permission_id = p.id 
                 WHERE u.biometric_credential_id = $1 
                   AND (p.code IN ('auth:supervisor', 'auth:store_manager') OR r.name IN ('SUPERVISOR', 'STORE_MANAGER', 'SUPERADMIN')) 
                   AND u.is_active = true 
                 LIMIT 1",
            )
            .bind(cred_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

            if let Some(s) = spv {
                let role_name = sqlx::query_scalar::<_, String>("SELECT name FROM roles WHERE id = $1")
                    .bind(s.role_id)
                    .fetch_optional(pool)
                    .await
                    .unwrap_or(None)
                    .unwrap_or_else(|| "SUPERVISOR".to_string());
                let auth_token = create_jwt(s.id, &s.username, s.role_id, &role_name, None, 2).ok();

                return Ok(SupervisorPinResponse {
                    success: true,
                    supervisor_id: s.id,
                    supervisor_name: s.full_name,
                    message: "Otorisasi Biometrik Sidik Jari Berhasil".to_string(),
                    auth_token,
                });
            }
        }
    }

    Err("Otorisasi biometrik gagal: Data tidak cocok dengan supervisor manapun".to_string())
}

pub async fn verify_supervisor_auth(
    pool: &PgPool,
    supervisor_id: Option<Uuid>,
    supervisor_pin: Option<&str>,
    supervisor_auth_token: Option<&str>,
    action: &str,
) -> Result<Uuid, String> {
    // 1. Ambil setting biometrik toko
    let (bio_mode, bio_policy) = sqlx::query_as::<_, (String, String)>(
        "SELECT supervisor_biometric_mode, supervisor_biometric_policy 
         FROM receipt_settings ORDER BY created_at ASC LIMIT 1"
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None)
    .unwrap_or_else(|| ("ANY".to_string(), "OPTIONAL".to_string()));

    // 2. Jika supervisor_auth_token diberikan (dari verifikasi biometrik / PIN terotorisasi)
    if let Some(token) = supervisor_auth_token {
        if !token.trim().is_empty() {
            let claims = crate::utils::jwt::verify_jwt(token.trim())?;
            let is_spv = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(
                    SELECT 1 FROM users u 
                    JOIN roles r ON u.role_id = r.id 
                    LEFT JOIN role_permissions rp ON r.id = rp.role_id 
                    LEFT JOIN permissions p ON rp.permission_id = p.id 
                    WHERE u.id = $1 
                      AND (p.code IN ('auth:supervisor', 'auth:store_manager') OR r.name IN ('SUPERVISOR', 'STORE_MANAGER', 'SUPERADMIN')) 
                      AND u.is_active = true
                )"
            )
            .bind(claims.sub)
            .fetch_one(pool)
            .await
            .unwrap_or(false);

            if !is_spv {
                return Err("Token otorisasi bukan berasal dari akun supervisor yang berwenang".to_string());
            }

            if let Some(spv_id) = supervisor_id {
                if spv_id != claims.sub {
                    return Err("ID Supervisor tidak sesuai dengan kredensial token otorisasi".to_string());
                }
            }

            return Ok(claims.sub);
        }
    }

    // 3. Jika kebijakan STRICT dan mode bukan OFF, tolak PIN manual
    if bio_mode != "OFF" && bio_policy == "STRICT" {
        return Err("Kebijakan sistem toko mewajibkan otorisasi biometrik supervisor (PIN keypad dinonaktifkan)".to_string());
    }

    // 4. Jika supervisor_pin diberikan
    if let Some(pin) = supervisor_pin {
        if !pin.trim().is_empty() {
            let spv_res = verify_supervisor_pin(
                pool,
                SupervisorPinRequest {
                    supervisor_id,
                    pin: pin.to_string(),
                    action: Some(action.to_string()),
                },
            )
            .await?;
            return Ok(spv_res.supervisor_id);
        }
    }

    Err("Otorisasi supervisor (Biometrik atau PIN) wajib dilakukan untuk melanjutkan".to_string())
}

pub async fn enroll_user_biometric(
    pool: &PgPool,
    user_id: Uuid,
    req: EnrollBiometricRequest,
) -> Result<String, String> {
    match req.biometric_type.as_str() {
        "FINGERPRINT" => {
            let cred_id = req
                .credential_id
                .ok_or_else(|| "Credential ID sidik jari wajib diisi".to_string())?;
            sqlx::query(
                "UPDATE users 
                 SET biometric_credential_id = $1, biometric_public_key = $2, has_biometric = true, updated_at = CURRENT_TIMESTAMP 
                 WHERE id = $3",
            )
            .bind(&cred_id)
            .bind(&req.public_key)
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?;
            Ok("Sensor sidik jari / Touch ID berhasil didaftarkan".to_string())
        }
        "FACE" => {
            let desc = req
                .face_descriptor
                .ok_or_else(|| "Data template deskriptor wajah wajib diisi".to_string())?;
            sqlx::query(
                "UPDATE users 
                 SET face_descriptor = $1, face_photo_preview = $2, has_face_recognition = true, updated_at = CURRENT_TIMESTAMP 
                 WHERE id = $3",
            )
            .bind(&desc)
            .bind(&req.face_photo_preview)
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?;
            Ok("Pemindai wajah berhasil didaftarkan dan diaktifkan".to_string())
        }
        _ => Err("Tipe biometrik tidak didukung".to_string()),
    }
}

pub async fn reset_user_biometric(
    pool: &PgPool,
    user_id: Uuid,
    biometric_type: Option<String>,
) -> Result<String, String> {
    match biometric_type.as_deref() {
        Some("FINGERPRINT") => {
            sqlx::query(
                "UPDATE users 
                 SET biometric_credential_id = NULL, biometric_public_key = NULL, has_biometric = false, updated_at = CURRENT_TIMESTAMP 
                 WHERE id = $1",
            )
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?;
            Ok("Data sidik jari berhasil dihapus".to_string())
        }
        Some("FACE") => {
            sqlx::query(
                "UPDATE users 
                 SET face_descriptor = NULL, face_photo_preview = NULL, has_face_recognition = false, updated_at = CURRENT_TIMESTAMP 
                 WHERE id = $1",
            )
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?;
            Ok("Data pemindai wajah berhasil dihapus".to_string())
        }
        _ => {
            sqlx::query(
                "UPDATE users 
                 SET biometric_credential_id = NULL, biometric_public_key = NULL, 
                     face_descriptor = NULL, face_photo_preview = NULL, 
                     has_biometric = false, has_face_recognition = false, 
                     updated_at = CURRENT_TIMESTAMP 
                 WHERE id = $1",
            )
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?;
            Ok("Seluruh data biometrik berhasil direset".to_string())
        }
    }
}

pub async fn get_user_profile(pool: &PgPool, user_id: Uuid) -> Result<UserProfile, String> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Pengguna tidak ditemukan".to_string())?;

    let role = sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE id = $1")
        .bind(user.role_id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Role error: {}", e))?;

    let permissions = sqlx::query_scalar::<_, String>(
        "SELECT p.code FROM permissions p 
         JOIN role_permissions rp ON p.id = rp.permission_id 
         WHERE rp.role_id = $1",
    )
    .bind(user.role_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Permission error: {}", e))?;

    let outlet_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT outlet_id FROM user_outlets WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    Ok(UserProfile {
        id: user.id,
        role_id: user.role_id,
        role_name: role.name,
        username: user.username,
        full_name: user.full_name,
        email: user.email,
        phone: user.phone,
        permissions,
        outlet_ids,
        has_biometric: user.has_biometric,
        has_face_recognition: user.has_face_recognition,
        face_photo_preview: user.face_photo_preview,
    })
}
