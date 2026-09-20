use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::net::UdpSocket;
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
    pub started_at: Option<DateTime<Utc>>,
    pub uptime_seconds: u64,
    pub host_ips: Vec<String>,
    pub total_connected_devices: usize,
    pub pairing_code: String,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub tx: Arc<broadcast::Sender<String>>,
    pub devices: Arc<RwLock<HashMap<String, ConnectedDevice>>>,
    pub is_running: Arc<RwLock<bool>>,
    pub started_at: Arc<RwLock<Option<DateTime<Utc>>>>,
    pub port: u16,
    pub control_port: u16,
    pub pairing_code: Arc<RwLock<String>>,
    pub server_handle: Arc<RwLock<Option<tokio::task::AbortHandle>>>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceConnectQuery {
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub device_type: Option<String>,
    pub user_name: Option<String>,
    pub pairing_code: Option<String>,
}

impl AppState {
    pub fn new(pool: PgPool, port: u16) -> Self {
        let (tx, _rx) = broadcast::channel(200);
        let default_pairing = std::env::var("PAIRING_CODE").unwrap_or_else(|_| "NOTANE-8899".to_string());
        Self {
            pool,
            tx: Arc::new(tx),
            devices: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(true)),
            started_at: Arc::new(RwLock::new(Some(Utc::now()))),
            port,
            control_port: port + 1,
            pairing_code: Arc::new(RwLock::new(default_pairing)),
            server_handle: Arc::new(RwLock::new(None)),
        }
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
        // Coba deteksi IP LAN melalui UDP socket dummy (tidak mengirim data nyata ke internet)
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(local_addr) = socket.local_addr() {
                    ips.push(local_addr.ip().to_string());
                }
            }
        }
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

        ServerStatus {
            is_running: running,
            port: self.port,
            control_port: self.control_port,
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

        let payload = serde_json::json!({
            "type": "PAIRING_CODE_UPDATED",
            "pairing_code": code.clone(),
            "timestamp": Utc::now()
        });
        let _ = self.tx.send(payload.to_string());
        code
    }

    pub async fn register_device(&self, dev: ConnectedDevice) {
        let mut devs = self.devices.write().await;
        info!(
            "[SERVER HOST] Device terhubung & tervalidasi: {} ({}) dari {}",
            dev.device_id, dev.device_type, dev.client_ip
        );
        devs.insert(dev.device_id.clone(), dev.clone());
        drop(devs);

        // Broadcast event ke seluruh terminal/host
        let payload = serde_json::json!({
            "type": "DEVICE_CONNECTED",
            "device": dev,
            "timestamp": Utc::now()
        });
        let _ = self.tx.send(payload.to_string());
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
    Query(query): Query<DeviceConnectQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if !*state.is_running.read().await {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "Host Server sedang dihentikan (Stopped). Tidak dapat menerima koneksi baru."
        ).into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state, query))
}

async fn handle_socket(socket: WebSocket, state: AppState, query: DeviceConnectQuery) {
    let (mut sender, mut receiver) = socket.split();

    // 1. Verifikasi Kode Pencocokan (Pairing Code)
    let expected_code = state.pairing_code.read().await.clone();
    let provided_code = query.pairing_code.as_deref().unwrap_or("").trim();

    if provided_code != expected_code {
        warn!(
            "[SERVER HOST] Penolakan koneksi: Kode pencocokan tidak valid dari device '{:?}'. Diberikan: '{}', Diharapkan: '{}'",
            query.device_id, provided_code, expected_code
        );
        let rejection = serde_json::json!({
            "type": "CONNECTION_REJECTED",
            "message": "Kode Pencocokan (Pairing Code) salah atau tidak terdaftar pada Host Server. Akses ditolak!"
        });
        let _ = sender.send(Message::Text(rejection.to_string().into())).await;
        let _ = sender.send(Message::Close(None)).await;
        return;
    }

    let device_id = query
        .device_id
        .unwrap_or_else(|| format!("TERMINAL-{}", &uuid::Uuid::new_v4().to_string()[..6]));
    let device_name = query
        .device_name
        .unwrap_or_else(|| format!("Perangkat {}", &device_id));
    let device_type = query
        .device_type
        .unwrap_or_else(|| "CASHIER".to_string());
    let user_name = query.user_name;

    let device_record = ConnectedDevice {
        device_id: device_id.clone(),
        device_name,
        device_type,
        client_ip: "LAN Client".to_string(),
        user_name,
        connected_at: Utc::now(),
        last_heartbeat: Utc::now(),
        status: "ONLINE".to_string(),
    };

    state.register_device(device_record).await;

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
