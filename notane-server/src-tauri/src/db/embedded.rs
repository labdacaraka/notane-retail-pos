use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

static GLOBAL_PG: LazyLock<Arc<Mutex<Option<postgresql_embedded::PostgreSQL>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

#[derive(Clone)]
pub struct EmbeddedPgManager;

impl EmbeddedPgManager {
    pub fn new() -> Self {
        Self
    }

    /// Cek apakah port target (misal 15432) sudah ada service PostgreSQL yang aktif
    pub async fn is_port_active(host: &str, port: u16) -> bool {
        tokio::net::TcpStream::connect(format!("{}:{}", host, port)).await.is_ok()
    }

    /// Memulai PostgreSQL Embedded terisolasi jika belum ada server yang berjalan di port target
    pub async fn start_if_needed(&self, port: u16) -> Result<String, String> {
        // Jika port sudah aktif (misal via container atau instance terisolasi), gunakan langsung
        if Self::is_port_active("127.0.0.1", port).await {
            info!("Database PostgreSQL terdeteksi aktif pada port terisolasi {}", port);
            return Ok(format!("postgres://postgres:postgres@127.0.0.1:{}/notane_pos", port));
        }

        // Coba nyalakan container Podman atau Docker jika tersedia (hanya di lingkungan server/unix atau jika ada)
        #[cfg(not(windows))]
        let container_commands = &["/opt/podman/bin/podman", "podman", "docker"];
        #[cfg(windows)]
        let container_commands: &[&str] = &[]; // Hindari popup cmd di desktop Windows kasir

        for cmd in container_commands {
            for container in &["notane-postgres", "notane-retail-postgres"] {
                let mut c = tokio::process::Command::new(cmd);
                c.args(["start", container]);
                #[cfg(windows)]
                {
                    #[allow(unused_imports)]
                    use std::os::windows::process::CommandExt;
                    c.creation_flags(0x08000000); // CREATE_NO_WINDOW
                }
                if let Ok(output) = c.output().await {
                    if output.status.success() {
                        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                        if Self::is_port_active("127.0.0.1", port).await {
                            info!("Container database '{}' berhasil dinyalakan via {}", container, cmd);
                            return Ok(format!("postgres://postgres:postgres@127.0.0.1:{}/notane_pos", port));
                        }
                    }
                }
            }
        }

        info!("Port terisolasi {} belum aktif. Mempersiapkan Embedded PostgreSQL...", port);

        // Siapkan direktori terisolasi khusus Notane POS
        let home_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("./data"));
        
        let notane_dir = home_dir.join("notane-retail-pos").join("embedded_pg");
        let data_dir = notane_dir.join("data");
        std::fs::create_dir_all(&data_dir).map_err(|e| format!("Gagal membuat direktori data embedded: {}", e))?;

        let settings = postgresql_embedded::SettingsBuilder::default()
            .port(port)
            .data_dir(data_dir)
            .username("postgres")
            .password("postgres")
            .build();

        let mut pg = postgresql_embedded::PostgreSQL::new(settings);

        if let Err(e) = pg.setup().await {
            warn!("Setup embedded postgresql info: {}. Melanjutkan start...", e);
        }

        match pg.start().await {
            Ok(_) => {
                info!("Embedded PostgreSQL berhasil dijalankan pada port terisolasi {}", port);
                let _ = pg.create_database("notane_pos").await;
                let mut guard = GLOBAL_PG.lock().await;
                *guard = Some(pg);
                Ok(format!("postgres://postgres:postgres@127.0.0.1:{}/notane_pos", port))
            }
            Err(e) => {
                error!("Gagal menyalakan Embedded PostgreSQL: {}", e);
                Err(format!("Gagal start embedded postgres: {}", e))
            }
        }
    }

    /// Menghentikan instance embedded postgres secara aman
    pub async fn stop(&self) {
        let mut guard = GLOBAL_PG.lock().await;
        if let Some(pg) = guard.as_mut() {
            info!("Menghentikan Embedded PostgreSQL...");
            if let Err(e) = pg.stop().await {
                error!("Error saat menghentikan Embedded PostgreSQL: {}", e);
            }
        }
        *guard = None;
    }
}

impl Default for EmbeddedPgManager {
    fn default() -> Self {
        Self::new()
    }
}
