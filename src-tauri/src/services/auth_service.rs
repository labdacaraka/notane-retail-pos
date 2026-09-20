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

    let profile = UserProfile {
        id: user.id,
        role_id: user.role_id,
        role_name: role.name,
        username: user.username,
        full_name: user.full_name,
        email: user.email,
        phone: user.phone,
        permissions,
    };

    Ok(LoginResponse {
        token,
        user: profile,
    })
}

pub async fn verify_supervisor_pin(
    pool: &PgPool,
    req: SupervisorPinRequest,
) -> Result<SupervisorPinResponse, String> {
    // Cari user dengan peran SUPERVISOR, STORE_MANAGER, atau SUPERADMIN
    let supervisors = sqlx::query_as::<_, User>(
        "SELECT u.* FROM users u 
         JOIN roles r ON u.role_id = r.id 
         WHERE r.name IN ('SUPERVISOR', 'STORE_MANAGER', 'SUPERADMIN') AND u.is_active = true",
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

    Ok(UserProfile {
        id: user.id,
        role_id: user.role_id,
        role_name: role.name,
        username: user.username,
        full_name: user.full_name,
        email: user.email,
        phone: user.phone,
        permissions,
    })
}
