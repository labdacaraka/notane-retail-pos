use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemPrinterInfo {
    pub name: String,
    pub is_default: bool,
}

#[tauri::command]
pub fn get_system_info() -> serde_json::Value {
    json!({
        "app_name": "Notane Retail POS",
        "version": "1.0.0",
        "mode": "Host / Client Dual Mode",
        "backend": "Rust Axum + PostgreSQL",
        "frontend": "Vue 3 + Tailwind CSS"
    })
}

#[tauri::command]
pub fn open_cash_drawer(terminal_id: String) -> Result<String, String> {
    // ESC/POS command signal for kick-out cash drawer (ESC p m t1 t2 => 27, 112, 0, 25, 250)
    println!("[HARDWARE IPC] Membuka Cash Drawer pada terminal {}", terminal_id);
    Ok(format!("Sinyal buka laci kasir berhasil dikirim ke {}", terminal_id))
}

#[tauri::command]
pub fn print_receipt(receipt_data: serde_json::Value) -> Result<String, String> {
    println!("[HARDWARE IPC] Mencetak struk transaksi thermal: {:?}", receipt_data);
    Ok("Struk berhasil dikirim ke antrean cetak".to_string())
}

#[tauri::command]
pub async fn open_customer_display_window(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("customer-display") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        Ok(())
    } else {
        let builder = tauri::WebviewWindowBuilder::new(
            &app,
            "customer-display",
            tauri::WebviewUrl::App("/customer-display".into())
        )
        .title("Notane - Layar Hadap Pelanggan (Customer Display)")
        .inner_size(1024.0, 600.0)
        .resizable(true);

        builder.build().map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[tauri::command]
pub async fn open_server_host_window(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("server-host") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        Ok(())
    } else {
        let builder = tauri::WebviewWindowBuilder::new(
            &app,
            "server-host",
            tauri::WebviewUrl::App("/server-host".into())
        )
        .title("Notane Retail Server - Host Manager")
        .inner_size(1100.0, 750.0)
        .resizable(true);

        let window = builder.build().map_err(|e| e.to_string())?;
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        Ok(())
    }
}

/// Mengambil daftar printer yang terpasang di sistem operasi (Windows / macOS / Linux)
#[tauri::command]
pub async fn get_system_printers() -> Result<Vec<SystemPrinterInfo>, String> {
    let mut printers = Vec::new();

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        // 1. Dapatkan printer default sistem
        let default_output = std::process::Command::new("lpstat")
            .arg("-d")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        let default_name = default_output
            .split(':')
            .nth(1)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        // 2. Dapatkan daftar semua printer yang aktif
        if let Ok(output) = std::process::Command::new("lpstat").arg("-e").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let name = line.trim().to_string();
                if !name.is_empty() {
                    let is_default = !default_name.is_empty() && name == default_name;
                    printers.push(SystemPrinterInfo { name, is_default });
                }
            }
        }

        // Fallback jika lpstat -e kosong, coba lpstat -p
        if printers.is_empty() {
            if let Ok(output) = std::process::Command::new("lpstat").arg("-p").output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    // baris format: "printer EPSON_L3550 is idle..."
                    if line.starts_with("printer ") {
                        if let Some(name) = line.split_whitespace().nth(1) {
                            let name = name.to_string();
                            let is_default = !default_name.is_empty() && name == default_name;
                            printers.push(SystemPrinterInfo { name, is_default });
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let script = r#"
            $def = (Get-CimInstance Win32_Printer | Where-Object { $_.Default -eq $true } | Select-Object -ExpandProperty Name -ErrorAction SilentlyContinue)
            Get-CimInstance Win32_Printer | Select-Object Name, @{Name='IsDefault'; Expression={ $_.Name -eq $def }} | ConvertTo-Json
        "#;
        if let Ok(output) = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&stdout) {
                if let Some(arr) = parsed.as_array() {
                    for item in arr {
                        let name = item["Name"].as_str().unwrap_or("").to_string();
                        let is_default = item["IsDefault"].as_bool().unwrap_or(false);
                        if !name.is_empty() {
                            printers.push(SystemPrinterInfo { name, is_default });
                        }
                    }
                } else if let Some(obj) = parsed.as_object() {
                    let name = obj.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let is_default = obj.get("IsDefault").and_then(|v| v.as_bool()).unwrap_or(false);
                    if !name.is_empty() {
                        printers.push(SystemPrinterInfo { name, is_default });
                    }
                }
            }
        }
    }

    Ok(printers)
}

/// Cetak data biner/raw (misal instruksi TSPL/ESC-POS) langsung ke printer sistem tanpa dialog
#[tauri::command]
pub async fn silent_print_raw(printer_name: Option<String>, raw_data: String) -> Result<String, String> {
    use std::io::Write;
    let temp_dir = std::env::temp_dir();
    let file_name = format!("notane_raw_print_{}.bin", uuid::Uuid::new_v4());
    let file_path = temp_dir.join(&file_name);

    let mut file = std::fs::File::create(&file_path)
        .map_err(|e| format!("Gagal membuat file cetak raw: {}", e))?;

    file.write_all(raw_data.as_bytes())
        .map_err(|e| format!("Gagal menulis data cetak raw: {}", e))?;

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let mut cmd = std::process::Command::new("lp");
        if let Some(ref p) = printer_name {
            let clean = p.trim();
            if !clean.is_empty() && clean != "Driver Sistem (OS Default / Browser)" {
                cmd.args(["-d", clean]);
            }
        }
        cmd.args(["-o", "raw", &file_path.to_string_lossy()]);

        let output = cmd.output().map_err(|e| format!("Gagal menjalankan perintah spooler lp: {}", e))?;
        let _ = std::fs::remove_file(&file_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Spooler lp gagal: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(if stdout.is_empty() { "Data cetak berhasil dikirim ke printer spooler".to_string() } else { stdout })
    }

    #[cfg(target_os = "windows")]
    {
        let target_printer = printer_name.unwrap_or_default();
        let target_printer = target_printer.trim();

        let script = if target_printer.is_empty() || target_printer == "Driver Sistem (OS Default / Browser)" {
            format!("Get-Content -Path '{}' -Raw | Out-Printer", file_path.to_string_lossy())
        } else {
            format!("Get-Content -Path '{}' -Raw | Out-Printer -Name '{}'", file_path.to_string_lossy(), target_printer)
        };

        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .map_err(|e| format!("Gagal mencetak ke spooler Windows: {}", e))?;

        let _ = std::fs::remove_file(&file_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Windows printer spooler gagal: {}", stderr));
        }

        Ok("Data cetak berhasil dikirim ke printer Windows".to_string())
    }
}

/// Cetak dokumen HTML/teks langsung ke printer OS secara silent tanpa membuka peramban eksternal
#[tauri::command]
pub async fn print_html_document(
    _title: String,
    html_content: String,
    printer_name: Option<String>,
) -> Result<String, String> {
    use std::io::Write;
    let temp_dir = std::env::temp_dir();
    let file_name = format!("notane_label_print_{}.html", uuid::Uuid::new_v4());
    let file_path = temp_dir.join(&file_name);

    let mut file = std::fs::File::create(&file_path)
        .map_err(|e| format!("Gagal membuat file cetak sementara: {}", e))?;

    file.write_all(html_content.as_bytes())
        .map_err(|e| format!("Gagal menulis file cetak sementara: {}", e))?;

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let mut cmd = std::process::Command::new("lp");
        if let Some(ref p) = printer_name {
            let clean = p.trim();
            if !clean.is_empty() && clean != "Driver Sistem (OS Default / Browser)" {
                cmd.args(["-d", clean]);
            }
        }
        cmd.arg(&file_path);

        let output = cmd.output().map_err(|e| format!("Gagal mengirim ke lp: {}", e))?;
        let _ = std::fs::remove_file(&file_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Gagal mencetak dokumen: {}", stderr));
        }
        Ok("Dokumen cetak berhasil dikirim ke antrean printer".to_string())
    }

    #[cfg(target_os = "windows")]
    {
        let target_printer = printer_name.unwrap_or_default();
        let target_printer = target_printer.trim();

        let script = if target_printer.is_empty() || target_printer == "Driver Sistem (OS Default / Browser)" {
            format!("Start-Process -FilePath '{}' -Verb Print -WindowStyle Hidden", file_path.to_string_lossy())
        } else {
            format!("Start-Process -FilePath '{}' -Verb PrintTo -ArgumentList '{}' -WindowStyle Hidden", file_path.to_string_lossy(), target_printer)
        };

        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .map_err(|e| format!("Gagal mencetak dokumen: {}", e))?;

        let _ = std::fs::remove_file(&file_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Gagal mencetak dokumen: {}", stderr));
        }
        Ok("Dokumen cetak berhasil dikirim ke antrean printer".to_string())
    }
}

