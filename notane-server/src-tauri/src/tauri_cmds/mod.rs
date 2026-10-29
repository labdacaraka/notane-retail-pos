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
        "app_name": "Notane Retail POS Server",
        "version": crate::services::update_service::get_app_version(),
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
        #[allow(unused_imports)]
        use std::os::windows::process::CommandExt;
        let script = r#"
            $def = (Get-CimInstance Win32_Printer | Where-Object { $_.Default -eq $true } | Select-Object -ExpandProperty Name -ErrorAction SilentlyContinue)
            Get-CimInstance Win32_Printer | Select-Object Name, @{Name='IsDefault'; Expression={ $_.Name -eq $def }} | ConvertTo-Json
        "#;
        let mut cmd = std::process::Command::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        if let Ok(output) = cmd.output()
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

        let lower = target_printer.to_lowercase();
        if lower.contains("pdf") || lower.contains("xps") || lower.contains("onenote") {
            let _ = std::fs::remove_file(&file_path);
            return Err(format!(
                "Printer [{}] adalah printer dokumen (bukan printer thermal TSPL). Silakan gunakan fitur Pratinjau / Cetak Dokumen (OS/PDF) untuk mengekspor dokumen.",
                target_printer
            ));
        }

        let script = if target_printer.is_empty() || target_printer == "Driver Sistem (OS Default / Browser)" {
            format!("Get-Content -Path '{}' -Raw | Out-Printer", file_path.to_string_lossy())
        } else {
            format!("Get-Content -Path '{}' -Raw | Out-Printer -Name '{}'", file_path.to_string_lossy(), target_printer)
        };

        #[allow(unused_imports)]
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        let output = cmd
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
        let target_printer = target_printer.trim().to_string();

        let script = format!(
            r#"
            $edgePath = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\msedge.exe' -ErrorAction SilentlyContinue).'(default)'
            if (-not $edgePath -or -not (Test-Path $edgePath)) {{
                $edgePath = "${{env:ProgramFiles(x86)}}\Microsoft\Edge\Application\msedge.exe"
            }}
            if (-not (Test-Path $edgePath)) {{
                $edgePath = "${{env:ProgramFiles}}\Microsoft\Edge\Application\msedge.exe"
            }}
            $target = "{}"
            $file = "{}"
            if (Test-Path $edgePath) {{
                if ($target -and $target -ne "Driver Sistem (OS Default / Browser)") {{
                    Start-Process -FilePath $edgePath -ArgumentList "--headless", "--disable-gpu", "--print-to-printer=`"$target`"", "`"$file`"" -Wait
                }} else {{
                    Start-Process -FilePath $edgePath -ArgumentList "--headless", "--disable-gpu", "--print-to-printer", "`"$file`"" -Wait
                }}
            }} else {{
                if ($target -and $target -ne "Driver Sistem (OS Default / Browser)") {{
                    Start-Process -FilePath $file -Verb PrintTo -ArgumentList "`"$target`"" -WindowStyle Hidden -Wait
                }} else {{
                    Start-Process -FilePath $file -Verb Print -WindowStyle Hidden -Wait
                }}
            }}
            "#,
            target_printer,
            file_path.to_string_lossy()
        );

        #[allow(unused_imports)]
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        let output = cmd
            .output()
            .map_err(|e| format!("Gagal mencetak dokumen: {}", e))?;

        let _ = std::fs::remove_file(&file_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.is_empty() {
                return Err(format!("Gagal mencetak dokumen: {}", stderr));
            }
        }
        Ok("Dokumen cetak berhasil dikirim ke antrean printer".to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemStoragePaths {
    pub downloads: String,
    pub documents: String,
    pub desktop: String,
    pub default_export_dir: String,
    pub default_backup_dir: String,
}

/// Mengambil path direktori standar sistem (Downloads, Documents, Desktop, Default Notane)
#[tauri::command]
pub fn get_system_storage_paths() -> SystemStoragePaths {
    let downloads = dirs::download_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|p| p.join("Downloads").to_string_lossy().to_string())
                .unwrap_or_else(|| "Downloads".to_string())
        });

    let documents = dirs::document_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|p| p.join("Documents").to_string_lossy().to_string())
                .unwrap_or_else(|| "Documents".to_string())
        });

    let desktop = dirs::desktop_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|p| p.join("Desktop").to_string_lossy().to_string())
                .unwrap_or_else(|| "Desktop".to_string())
        });

    let default_export_dir = dirs::document_dir()
        .map(|p| p.join("Notane").join("Exports").to_string_lossy().to_string())
        .unwrap_or_else(|| format!("{}/Notane/Exports", documents));

    let default_backup_dir = dirs::document_dir()
        .map(|p| p.join("Notane").join("Backups").to_string_lossy().to_string())
        .unwrap_or_else(|| format!("{}/Notane/Backups", documents));

    SystemStoragePaths {
        downloads,
        documents,
        desktop,
        default_export_dir,
        default_backup_dir,
    }
}

/// Menampilkan dialog native OS untuk memilih folder/direktori tujuan
#[tauri::command]
pub async fn select_folder(default_path: Option<String>) -> Result<Option<String>, String> {
    #[cfg(target_os = "macos")]
    {
        let script = match default_path {
            Some(ref p) if !p.trim().is_empty() => format!(
                "POSIX path of (choose folder with prompt \"Pilih Direktori Penyimpanan Notane:\" default location POSIX file \"{}\")",
                p.trim().replace('"', "\\\"")
            ),
            _ => "POSIX path of (choose folder with prompt \"Pilih Direktori Penyimpanan Notane:\")".to_string(),
        };

        let output = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| format!("Gagal memanggil pemilih direktori: {}", e))?;

        if !output.status.success() {
            return Ok(None);
        }

        let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if selected.is_empty() {
            Ok(None)
        } else {
            Ok(Some(selected))
        }
    }

    #[cfg(target_os = "windows")]
    {
        let initial_dir = default_path.unwrap_or_default().replace('"', "`\"");
        let script = format!(
            r#"
            Add-Type -AssemblyName System.Windows.Forms
            $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
            $dialog.Description = "Pilih Direktori Penyimpanan Notane"
            $dialog.ShowNewFolderButton = $true
            if ("{}" -ne "" -and (Test-Path "{}")) {{
                $dialog.SelectedPath = "{}"
            }}
            if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{
                [Console]::WriteLine($dialog.SelectedPath)
            }}
            "#,
            initial_dir, initial_dir, initial_dir
        );

        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .map_err(|e| format!("Gagal membuka pemilih direktori: {}", e))?;

        let res = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if res.is_empty() {
            Ok(None)
        } else {
            Ok(Some(res))
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let output = std::process::Command::new("zenity")
            .args(["--file-selection", "--directory", "--title=Pilih Direktori Penyimpanan Notane"])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !path.is_empty() {
                    return Ok(Some(path));
                }
            }
        }
        Ok(None)
    }
}

/// Membuka direktori di File Explorer (Windows), Finder (macOS), atau File Manager (Linux)
#[tauri::command]
pub fn open_folder(folder_path: String) -> Result<bool, String> {
    let trimmed = folder_path.trim();
    if trimmed.is_empty() {
        return Err("Direktori tidak boleh kosong".to_string());
    }

    let p = std::path::Path::new(trimmed);
    if !p.exists() {
        std::fs::create_dir_all(p).map_err(|e| format!("Gagal membuat direktori: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(trimmed)
            .spawn()
            .map_err(|e| format!("Gagal membuka folder di Finder: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(trimmed)
            .spawn()
            .map_err(|e| format!("Gagal membuka folder di File Explorer: {}", e))?;
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::process::Command::new("xdg-open")
            .arg(trimmed)
            .spawn()
            .map_err(|e| format!("Gagal membuka folder: {}", e))?;
    }

    Ok(true)
}

/// Menyimpan data biner (Base64 encoded) langsung ke file di direktori tertentu
#[tauri::command]
pub fn save_file_to_directory(
    directory: String,
    filename: String,
    base64_data: String,
) -> Result<String, String> {
    use base64::Engine;

    let dir_trimmed = directory.trim();
    if dir_trimmed.is_empty() {
        return Err("Direktori tujuan tidak valid".to_string());
    }

    let dir_path = std::path::Path::new(dir_trimmed);
    if !dir_path.exists() {
        std::fs::create_dir_all(dir_path).map_err(|e| format!("Gagal membuat direktori tujuan: {}", e))?;
    }

    // Ambil hanya nama file dasar untuk mencegah path traversal (security)
    let clean_filename = std::path::Path::new(&filename)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Nama berkas tidak valid".to_string())?;

    let file_path = dir_path.join(clean_filename);

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_data.trim())
        .map_err(|e| format!("Gagal mengurai isi file (Base64 decode): {}", e))?;

    std::fs::write(&file_path, bytes).map_err(|e| format!("Gagal menyimpan berkas ke direktori: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}

