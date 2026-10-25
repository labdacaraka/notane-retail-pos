use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, Query, State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use std::net::SocketAddr;
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedDevice {
    pub device_id: String,
    pub device_name: String,
    pub device_type: String, // "CASHIER", "CUSTOMER_DISPLAY", "TABLET", "BACKOFFICE"
    pub client_ip: String,
    pub user_name: Option<String>,
    pub connected_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub status: String, // "ONLINE", "DISCONNECTED"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    pub is_running: bool,
    pub port: u16,
    pub control_port: u16,
    pub external_port: Option<u16>,
    pub started_at: Option<DateTime<Utc>>,
    pub uptime_seconds: u64,
    pub host_ips: Vec<String>,
    pub total_connected_devices: usize,
    pub pairing_code: String,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: Arc<RwLock<Option<PgPool>>>,
    pub tx: Arc<broadcast::Sender<String>>,
    pub devices: Arc<RwLock<HashMap<String, ConnectedDevice>>>,
    pub is_running: Arc<RwLock<bool>>,
    pub started_at: Arc<RwLock<Option<DateTime<Utc>>>>,
    pub port: u16,
    pub control_port: u16,
    pub pairing_code: Arc<RwLock<String>>,
    pub server_handle: Arc<RwLock<Option<tokio::task::AbortHandle>>>,
    pub db_url: Arc<RwLock<String>>,
    pub pg_port: u16,
    pub scheduler_started: Arc<AtomicBool>,
    pub voucher_scheduler_started: Arc<AtomicBool>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceConnectQuery {
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub device_type: Option<String>,
    pub user_name: Option<String>,
    pub pairing_code: Option<String>,
    pub is_host: Option<bool>,
}

impl AppState {
    pub fn new(pool: Option<PgPool>, port: u16, db_url: String, pg_port: u16) -> Self {
        let (tx, _rx) = broadcast::channel(200);
        let default_pairing = std::env::var("PAIRING_CODE").unwrap_or_else(|_| "NOTANE-8899".to_string());
        Self {
            pool: Arc::new(RwLock::new(pool)),
            tx: Arc::new(tx),
            devices: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            started_at: Arc::new(RwLock::new(None)),
            port,
            control_port: port + 1,
            pairing_code: Arc::new(RwLock::new(default_pairing)),
            server_handle: Arc::new(RwLock::new(None)),
            db_url: Arc::new(RwLock::new(db_url)),
            pg_port,
            scheduler_started: Arc::new(AtomicBool::new(false)),
            voucher_scheduler_started: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start_heartbeat_scheduler(&self) {
        if self.scheduler_started.swap(true, Ordering::SeqCst) {
            return;
        }
        let state = self.clone();
        tokio::spawn(async move {
            info!("[HEARTBEAT SCHEDULER] Background scheduler detak server aktif (Interval 2s). Mode Push ke seluruh subscriber client siap.");
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                if *state.is_running.read().await {
                    let total_devices = state
                        .devices
                        .read()
                        .await
                        .values()
                        .filter(|d| d.device_type != "HOST" && d.device_id != "HOST-SERVER")
                        .count();
                    let started = *state.started_at.read().await;
                    let uptime = started
                        .map(|t| (Utc::now() - t).num_seconds().max(0) as u64)
                        .unwrap_or(0);
                    let pairing = state.pairing_code.read().await.clone();

                    let beacon = serde_json::json!({
                        "type": "HOST_HEARTBEAT",
                        "timestamp": Utc::now().timestamp_millis(),
                        "server_time": Utc::now().to_rfc3339(),
                        "connected_devices": total_devices,
                        "is_running": true,
                        "uptime_seconds": uptime,
                        "pairing_code": pairing
                    });
                    let _ = state.tx.send(beacon.to_string());
                }
            }
        });
    }

    pub fn start_voucher_expiry_scheduler(&self) {
        if self.voucher_scheduler_started.swap(true, Ordering::SeqCst) {
            return;
        }
        let state = self.clone();
        tokio::spawn(async move {
            info!("[PROMO SCHEDULER] Background scheduler kedaluwarsa promo, voucher, dan flash sale aktif (Interval 5 menit).");
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let pool_guard = state.pool.read().await;
                if let Some(ref pool) = *pool_guard {
                    // 1. VOUCHERS AUTO-EXPIRY
                    match sqlx::query(
                        "UPDATE vouchers 
                         SET status = 'EXPIRED', updated_at = CURRENT_TIMESTAMP 
                         WHERE status = 'ACTIVE' AND valid_until <= CURRENT_TIMESTAMP",
                    )
                    .execute(pool)
                    .await
                    {
                        Ok(res) => {
                            let affected = res.rows_affected();
                            if affected > 0 {
                                info!(
                                    "[PROMO SCHEDULER] {} voucher otomatis diubah menjadi EXPIRED.",
                                    affected
                                );
                                let payload = serde_json::json!({
                                    "type": "VOUCHERS_EXPIRED",
                                    "count": affected,
                                    "timestamp": chrono::Utc::now()
                                });
                                let _ = state.tx.send(payload.to_string());
                            }
                        }
                        Err(e) => {
                            tracing::warn!("[PROMO SCHEDULER] Gagal memeriksa kedaluwarsa voucher: {}", e);
                        }
                    }

                    // 2. PROMOTIONS (BUY X GET Y, TEBUS MURAH, BUNDLING) AUTO-EXPIRY
                    match sqlx::query(
                        "UPDATE promotions 
                         SET is_active = false, updated_at = CURRENT_TIMESTAMP 
                         WHERE is_active = true AND end_date <= CURRENT_TIMESTAMP",
                    )
                    .execute(pool)
                    .await
                    {
                        Ok(res) => {
                            let affected = res.rows_affected();
                            if affected > 0 {
                                info!(
                                    "[PROMO SCHEDULER] {} program promosi ritel otomatis dinonaktifkan (end_date tercapai).",
                                    affected
                                );
                                let payload = serde_json::json!({
                                    "type": "PROMOTIONS_EXPIRED",
                                    "count": affected,
                                    "timestamp": chrono::Utc::now()
                                });
                                let _ = state.tx.send(payload.to_string());
                            }
                        }
                        Err(e) => {
                            tracing::warn!("[PROMO SCHEDULER] Gagal memeriksa kedaluwarsa promosi: {}", e);
                        }
                    }

                    // 3. FLASH SALES AUTO-EXPIRY
                    match sqlx::query(
                        "UPDATE flash_sales 
                         SET is_active = false, updated_at = CURRENT_TIMESTAMP 
                         WHERE is_active = true 
                           AND (
                               end_date < CURRENT_DATE 
                               OR (end_date = CURRENT_DATE AND is_all_day = false AND end_time IS NOT NULL AND end_time < CURRENT_TIME)
                           )",
                    )
                    .execute(pool)
                    .await
                    {
                        Ok(res) => {
                            let affected = res.rows_affected();
                            if affected > 0 {
                                info!(
                                    "[PROMO SCHEDULER] {} event flash sale otomatis dinonaktifkan karena waktu promo telah habis.",
                                    affected
                                );
                                let payload = serde_json::json!({
                                    "type": "FLASH_SALES_EXPIRED",
                                    "count": affected,
                                    "timestamp": chrono::Utc::now()
                                });
                                let _ = state.tx.send(payload.to_string());
                            }
                        }
                        Err(e) => {
                            tracing::warn!("[PROMO SCHEDULER] Gagal memeriksa kedaluwarsa flash sale: {}", e);
                        }
                    }

                    // 4. AUTO-RETENTION AUDIT LOG CLEANUP (Sesuai Konfigurasi Hari di receipt_settings)
                    match crate::services::audit_service::cleanup_expired_audit_logs(pool, None).await {
                        Ok(deleted) => {
                            if deleted > 0 {
                                info!(
                                    "[AUDIT SCHEDULER] {} baris log audit kadaluwarsa otomatis dibersihkan sesuai batas hari retensi.",
                                    deleted
                                );
                                let payload = serde_json::json!({
                                    "type": "AUDIT_LOGS_PURGED",
                                    "count": deleted,
                                    "timestamp": chrono::Utc::now()
                                });
                                let _ = state.tx.send(payload.to_string());
                            }
                        }
                        Err(e) => {
                            tracing::warn!("[AUDIT SCHEDULER] Gagal memeriksa retensi log audit: {}", e);
                        }
                    }
                }
            }
        });
    }

    pub async fn set_pool(&self, pool: PgPool) {
        let mut p = self.pool.write().await;
        *p = Some(pool);
    }

    pub async fn set_server_handle(&self, handle: tokio::task::AbortHandle) {
        let mut h = self.server_handle.write().await;
        *h = Some(handle);
    }

    pub async fn abort_server(&self) -> bool {
        let mut h = self.server_handle.write().await;
        if let Some(handle) = h.take() {
            handle.abort();
            true
        } else {
            false
        }
    }

    pub fn get_local_ips() -> Vec<String> {
        let mut ips = Vec::new();

        // 0. Ambil dari HOST_IP atau HOST_IPS jika diset via environment (misal di Docker/Podman/VPS)
        let mut custom_hosts = Vec::new();
        if let Ok(val) = std::env::var("HOST_IPS") {
            if !val.trim().is_empty() {
                custom_hosts.push(val);
            }
        }
        if let Ok(val) = std::env::var("HOST_IP") {
            if !val.trim().is_empty() {
                custom_hosts.push(val);
            }
        }
        for entry in custom_hosts {
            for ip in entry.split(',') {
                let clean = ip.trim().to_string();
                if !clean.is_empty() && !ips.contains(&clean) {
                    ips.push(clean);
                }
            }
        }

        // 1. Dapatkan IP utama via local_ip_address (cross-platform, offline-safe)
        if let Ok(my_local_ip) = local_ip_address::local_ip() {
            let ip_str = my_local_ip.to_string();
            if ip_str != "127.0.0.1" && !ip_str.starts_with("169.254.") && !ips.contains(&ip_str) {
                ips.push(ip_str);
            }
        }

        // 2. Scan seluruh adapter/network interfaces aktif di mesin komputer (Wi-Fi, Ethernet, LAN)
        if let Ok(network_interfaces) = local_ip_address::list_afinet_netifas() {
            for (_name, ip) in network_interfaces {
                if ip.is_ipv4() {
                    let ip_str = ip.to_string();
                    if ip_str != "127.0.0.1" && !ip_str.starts_with("169.254.") && !ips.contains(&ip_str) {
                        ips.push(ip_str);
                    }
                }
            }
        }

        // 3. Fallback jika masih kosong: coba probing via socket UDP dummy
        if ips.is_empty() {
            if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
                if socket.connect("8.8.8.8:80").is_ok() {
                    if let Ok(local_addr) = socket.local_addr() {
                        let ip_str = local_addr.ip().to_string();
                        if !ips.contains(&ip_str) {
                            ips.push(ip_str);
                        }
                    }
                }
            }
        }

        // 4. Pastikan 127.0.0.1 selalu ada sebagai opsi cadangan loopback
        if !ips.contains(&"127.0.0.1".to_string()) {
            ips.push("127.0.0.1".to_string());
        }

        ips
    }

    pub async fn get_status(&self) -> ServerStatus {
        let running = *self.is_running.read().await;
        let started = *self.started_at.read().await;
        let uptime = if running {
            started
                .map(|t| (Utc::now() - t).num_seconds().max(0) as u64)
                .unwrap_or(0)
        } else {
            0
        };
        let count = self.devices.read().await.len();
        let pairing = self.pairing_code.read().await.clone();

        let ext_port = std::env::var("SERVER_PORT")
            .or_else(|_| std::env::var("EXTERNAL_PORT"))
            .ok()
            .and_then(|p| p.parse::<u16>().ok());

        ServerStatus {
            is_running: running,
            port: self.port,
            control_port: self.control_port,
            external_port: ext_port,
            started_at: started,
            uptime_seconds: uptime,
            host_ips: Self::get_local_ips(),
            total_connected_devices: count,
            pairing_code: pairing,
        }
    }

    pub async fn regenerate_pairing_code(&self) -> String {
        let code = {
            let mut rng = rand::thread_rng();
            format!("NOTANE-{:04}", rng.gen_range(1000..9999))
        };
        let mut p = self.pairing_code.write().await;
        *p = code.clone();
        drop(p);

        // 1. Simpan ke database jika pool database tersedia
        let pool_guard = self.pool.read().await;
        if let Some(ref pool) = *pool_guard {
            let _ = sqlx::query("UPDATE receipt_settings SET pairing_code = $1")
                .bind(&code)
                .execute(pool)
                .await;
        }
        drop(pool_guard);

        // 2. Putuskan seluruh terminal client yang terhubung dengan kode lama
        let disconnect_payload = serde_json::json!({
            "type": "FORCE_DISCONNECT",
            "message": "Kode pairing telah diganti oleh Host Server. Masukkan kode pairing baru untuk menghubungkan kembali terminal ini."
        });
        let _ = self.tx.send(disconnect_payload.to_string());
        let mut devs = self.devices.write().await;
        devs.clear();
        drop(devs);

        // 3. Siarkan pembaruan kode pairing
        let payload = serde_json::json!({
            "type": "PAIRING_CODE_UPDATED",
            "pairing_code": code.clone(),
            "timestamp": Utc::now()
        });
        let _ = self.tx.send(payload.to_string());
        code
    }

    pub async fn set_pairing_code(&self, new_code: String) -> String {
        let mut p = self.pairing_code.write().await;
        *p = new_code.clone();
        drop(p);

        // 1. Simpan ke database jika pool database tersedia
        let pool_guard = self.pool.read().await;
        if let Some(ref pool) = *pool_guard {
            let _ = sqlx::query("UPDATE receipt_settings SET pairing_code = $1")
                .bind(&new_code)
                .execute(pool)
                .await;
        }
        drop(pool_guard);

        // 2. Putuskan seluruh terminal client kasir yang terhubung dengan kode lama
        let disconnect_payload = serde_json::json!({
            "type": "FORCE_DISCONNECT",
            "message": "Kode pairing telah diganti oleh Host Server. Masukkan kode pairing baru untuk menghubungkan kembali terminal ini."
        });
        let _ = self.tx.send(disconnect_payload.to_string());
        let mut devs = self.devices.write().await;
        // Hanya hapus terminal client eksternal, pertahankan koneksi host manager
        devs.retain(|_, d| d.device_type == "HOST" || d.device_id == "HOST-SERVER");
        drop(devs);

        // 3. Siarkan pembaruan kode pairing
        let payload = serde_json::json!({
            "type": "PAIRING_CODE_UPDATED",
            "pairing_code": new_code.clone(),
            "timestamp": Utc::now()
        });
        let _ = self.tx.send(payload.to_string());
        new_code
    }

    pub async fn register_device(&self, dev: ConnectedDevice) {
        let is_host = dev.device_type == "HOST" || dev.device_id == "HOST-SERVER";
        let mut devs = self.devices.write().await;
        info!(
            "[SERVER HOST] Device terhubung & tervalidasi: {} ({}) dari {}",
            dev.device_id, dev.device_type, dev.client_ip
        );
        devs.insert(dev.device_id.clone(), dev.clone());
        drop(devs);

        // Broadcast event ke seluruh terminal/host hanya jika bukan internal host manager
        if !is_host {
            let payload = serde_json::json!({
                "type": "DEVICE_CONNECTED",
                "device": dev,
                "timestamp": Utc::now()
            });
            let _ = self.tx.send(payload.to_string());
        }
    }

    pub async fn unregister_device(&self, device_id: &str) {
        let mut devs = self.devices.write().await;
        if let Some(dev) = devs.remove(device_id) {
            info!(
                "[SERVER HOST] Device terputus: {} ({})",
                dev.device_id, dev.device_type
            );
            drop(devs);

            let payload = serde_json::json!({
                "type": "DEVICE_DISCONNECTED",
                "device_id": device_id,
                "timestamp": Utc::now()
            });
            let _ = self.tx.send(payload.to_string());
        }
    }

    pub async fn update_heartbeat(&self, device_id: &str) {
        let mut devs = self.devices.write().await;
        if let Some(dev) = devs.get_mut(device_id) {
            dev.last_heartbeat = Utc::now();
            dev.status = "ONLINE".to_string();
        }
    }

    pub async fn kick_device(&self, device_id: &str) -> bool {
        let mut devs = self.devices.write().await;
        if devs.remove(device_id).is_some() {
            drop(devs);

            // Kirim sinyal putus khusus ke perangkat target
            let payload = serde_json::json!({
                "type": "FORCE_DISCONNECT",
                "device_id": device_id,
                "message": "Koneksi Anda diputuskan oleh Host Server."
            });
            let _ = self.tx.send(payload.to_string());
            true
        } else {
            false
        }
    }
}

pub fn ws_routes(state: AppState) -> Router {
    Router::new()
        .route("/events", get(ws_handler))
        .with_state(state)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    Query(query): Query<DeviceConnectQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !*state.is_running.read().await {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "Host Server sedang dihentikan (Stopped). Tidak dapat menerima koneksi baru."
        ).into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state, query, client_addr))
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    query: DeviceConnectQuery,
    client_addr: SocketAddr,
) {
    let (mut sender, mut receiver) = socket.split();

    // 1. Verifikasi Keamanan Jaringan & Identitas Host (Zero-Trust Anti-Spoofing)
    // Server HANYA mempercayai identitas HOST jika koneksi TCP fisiknya BENAR-BENAR berasal dari IP Loopback (127.0.0.1 / ::1).
    let is_physical_localhost = client_addr.ip().is_loopback();
    let claims_to_be_host = query.is_host.unwrap_or(false)
        || query.device_type.as_deref().map(|t| t.eq_ignore_ascii_case("HOST") || t.eq_ignore_ascii_case("SERVER")).unwrap_or(false)
        || query.device_id.as_deref().map(|d| d.eq_ignore_ascii_case("HOST-SERVER") || d.starts_with("HOST-")).unwrap_or(false);

    // Deteksi Upaya Spoofing: Jika perangkat dari LAN mengaku-ngaku sebagai HOST, tolak seketika!
    if claims_to_be_host && !is_physical_localhost {
        warn!(
            "[SECURITY ALERT] Upaya Pemalsuan Identitas (Host Spoofing)! Perangkat remote dari IP {} mencoba terhubung sebagai Host Server internal tanpa izin.",
            client_addr
        );
        let rejection = serde_json::json!({
            "type": "CONNECTION_REJECTED",
            "message": "Akses Ditolak: Perangkat dari jaringan eksternal/LAN dilarang menggunakan identitas Host Server!"
        });
        let _ = sender.send(Message::Text(rejection.to_string().into())).await;
        let _ = sender.send(Message::Close(None)).await;
        return;
    }

    let is_host_connection = is_physical_localhost && claims_to_be_host;

    // Untuk seluruh perangkat client kasir LAN (dan siapapun yang bukan internal localhost), wajib cocokkan pairing code!
    if !is_host_connection {
        let expected_code = state.pairing_code.read().await.clone();
        let provided_code = query.pairing_code.as_deref().unwrap_or("").trim();

        if provided_code != expected_code {
            warn!(
                "[SERVER HOST] Penolakan koneksi: Kode pencocokan tidak valid dari device '{:?}' di IP {}. Diberikan: '{}', Diharapkan: '{}'",
                query.device_id, client_addr, provided_code, expected_code
            );
            let rejection = serde_json::json!({
                "type": "CONNECTION_REJECTED",
                "message": "Kode Pencocokan (Pairing Code) salah atau tidak terdaftar pada Host Server. Akses ditolak!"
            });
            let _ = sender.send(Message::Text(rejection.to_string().into())).await;
            let _ = sender.send(Message::Close(None)).await;
            return;
        }
    }

    let device_id = query
        .device_id
        .unwrap_or_else(|| {
            if is_host_connection {
                "HOST-SERVER".to_string()
            } else {
                format!("TERMINAL-{}", &uuid::Uuid::new_v4().to_string()[..6])
            }
        });
    let device_name = query
        .device_name
        .unwrap_or_else(|| {
            if is_host_connection {
                "Host Server Manager".to_string()
            } else {
                format!("Perangkat {}", &device_id)
            }
        });
    let device_type = query
        .device_type
        .unwrap_or_else(|| {
            if is_host_connection {
                "HOST".to_string()
            } else {
                "CASHIER".to_string()
            }
        });
    let user_name = query.user_name;

    let device_record = ConnectedDevice {
        device_id: device_id.clone(),
        device_name,
        device_type,
        client_ip: if is_host_connection {
            "127.0.0.1 (Local Host)".to_string()
        } else {
            client_addr.ip().to_string()
        },
        user_name,
        connected_at: Utc::now(),
        last_heartbeat: Utc::now(),
        status: "ONLINE".to_string(),
    };

    state.register_device(device_record).await;

    // Kirim beacon inisial seketika kepada perangkat yang baru terhubung
    let started = *state.started_at.read().await;
    let uptime = started
        .map(|t| (Utc::now() - t).num_seconds().max(0) as u64)
        .unwrap_or(0);
    let pairing = state.pairing_code.read().await.clone();

    let initial_beacon = serde_json::json!({
        "type": "HOST_HEARTBEAT",
        "timestamp": Utc::now().timestamp_millis(),
        "server_time": Utc::now().to_rfc3339(),
        "connected_devices": state.devices.read().await.len(),
        "is_running": true,
        "uptime_seconds": uptime,
        "pairing_code": pairing
    });
    let _ = sender.send(Message::Text(initial_beacon.to_string().into())).await;

    let mut rx = state.tx.subscribe();

    let target_device_id = device_id.clone();
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // Cek jika server dihentikan atau force disconnect
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&msg) {
                let msg_type = parsed.get("type").and_then(|t| t.as_str());

                // Jika server dihentikan, putuskan seluruh socket client
                if msg_type == Some("SERVER_STOPPED") {
                    let _ = sender.send(Message::Text(msg.into())).await;
                    let _ = sender.send(Message::Close(None)).await;
                    break;
                }

                if msg_type == Some("FORCE_DISCONNECT") {
                    if parsed.get("device_id").and_then(|d| d.as_str()) == Some(&target_device_id) {
                        let _ = sender.send(Message::Text(msg.into())).await;
                        let _ = sender.send(Message::Close(None)).await;
                        break;
                    }
                }
            }

            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let state_recv = state.clone();
    let dev_id_recv = device_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            // Cek jika server dihentikan
            if !*state_recv.is_running.read().await {
                break;
            }
            match msg {
                Message::Text(text) => {
                    // Cek jika ini pesan heartbeat
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                        if parsed.get("type").and_then(|t| t.as_str()) == Some("HEARTBEAT") {
                            state_recv.update_heartbeat(&dev_id_recv).await;
                            continue;
                        }
                    }
                    info!("WebSocket client message [{}]: {}", dev_id_recv, text);
                    let _ = state_recv.tx.send(text.to_string());
                }
                Message::Ping(_) => {
                    state_recv.update_heartbeat(&dev_id_recv).await;
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }
        state_recv.unregister_device(&dev_id_recv).await;
    });

    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
            state.unregister_device(&device_id).await;
        },
        _ = (&mut recv_task) => {
            send_task.abort();
            state.unregister_device(&device_id).await;
        },
    };
}
