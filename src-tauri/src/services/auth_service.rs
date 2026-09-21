use crate::models::user::{LoginRequest, LoginResponse, Role, SupervisorPinRequest, SupervisorPinResponse, User, UserProfile};
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
}

pub async fn list_supervisors(pool: &PgPool) -> Result<Vec<SupervisorItem>, String> {
    let rows = sqlx::query_as::<_, SupervisorItem>(
        "SELECT DISTINCT u.id, u.username, u.full_name, r.name as role_name 
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
    .map_err(|e| format!("Database error: {}", e))?;

    Ok(rows)
}

pub async fn list_store_managers(pool: &PgPool) -> Result<Vec<SupervisorItem>, String> {
    let rows = sqlx::query_as::<_, SupervisorItem>(
        "SELECT DISTINCT u.id, u.username, u.full_name, r.name as role_name 
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
            return Ok(SupervisorPinResponse {
                success: true,
                supervisor_id: spv.id,
                supervisor_name: spv.full_name,
                message: "Otorisasi Supervisor Berhasil".to_string(),
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
            return Ok(SupervisorPinResponse {
                success: true,
                supervisor_id: spv.id,
                supervisor_name: spv.full_name,
                message: "Otorisasi Supervisor Berhasil".to_string(),
            });
        }
    }

    Err("PIN Supervisor tidak valid atau tidak memiliki wewenang".to_string())
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
    })
}
