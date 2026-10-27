pub mod api;
pub mod crypto;
pub mod db;
pub mod models;

#[cfg(feature = "gui")]
pub mod commands;

use api::{spawn_api_server, ApiState};
use crypto::ed25519::DEFAULT_MASTER_PRIVATE_KEY_HEX;
use std::collections::HashSet;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

#[derive(Clone)]
pub struct AppState {
    pub pool: db::DbPool,
    pub api_state: ApiState,
    pub active_sessions: Arc<RwLock<HashSet<String>>>,
}

pub async fn build_app_state() -> Result<AppState, Box<dyn std::error::Error>> {
    let pool = db::init_db(None).await?;

    let port: u16 = env::var("PROVIDER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(19090);

    let env_priv_key = env::var("PROVIDER_PRIVATE_KEY")
        .ok()
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty() && k.len() == 64);

    let master_private_key = if let Some(key) = env_priv_key {
        key
    } else {
        let db_key: Option<(String,)> = sqlx::query_as("SELECT value FROM provider_settings WHERE key = 'master_private_key'")
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

        db_key
            .map(|(k,)| k.trim().to_string())
            .filter(|k| !k.is_empty() && k.len() == 64)
            .unwrap_or_else(|| DEFAULT_MASTER_PRIVATE_KEY_HEX.to_string())
    };

    let active_sessions = Arc::new(RwLock::new(std::collections::HashSet::new()));

    let api_state = ApiState {
        pool: pool.clone(),
        master_private_key: Arc::new(RwLock::new(master_private_key)),
        is_running: Arc::new(RwLock::new(false)),
        port,
        server_handle: Arc::new(RwLock::new(None)),
        active_sessions: active_sessions.clone(),
    };

    // Auto-start API Server
    match spawn_api_server(api_state.clone()).await {
        Ok(handle) => {
            *api_state.server_handle.write().await = Some(handle);
            *api_state.is_running.write().await = true;
            info!("API Server Provider aktif secara otomatis pada port {}", port);
        }
        Err(e) => {
            error!("Gagal auto-start API Server Provider: {}", e);
        }
    }

    Ok(AppState { pool, api_state, active_sessions })
}

#[cfg(feature = "gui")]
pub fn run_app(context: tauri::Context<tauri::Wry>, state: AppState) {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::get_provider_status,
            commands::toggle_provider_server,
            commands::get_clients,
            commands::save_client,
            commands::update_client,
            commands::delete_client,
            commands::get_licenses,
            commands::generate_license,
            commands::update_license_status,
            commands::sign_license_token_manual,
            commands::get_device_activations,
            commands::unbind_device,
            commands::get_releases,
            commands::create_release,
            commands::delete_release,
            commands::get_audit_logs,
            commands::get_provider_settings,
            commands::save_provider_settings,
            commands::login_admin,
            commands::login_with_pin,
            commands::verify_admin_session,
            commands::logout_admin,
            commands::change_admin_credentials,
        ])
        .run(context)
        .expect("Error saat menjalankan aplikasi Notane Manager");
}
