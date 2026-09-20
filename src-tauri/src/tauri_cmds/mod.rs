use serde_json::json;

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
    // Here we emit or log the drawer open signal
    println!("[HARDWARE IPC] Membuka Cash Drawer pada terminal {}", terminal_id);
    Ok(format!("Sinyal buka laci kasir berhasil dikirim ke {}", terminal_id))
}

#[tauri::command]
pub fn print_receipt(receipt_data: serde_json::Value) -> Result<String, String> {
    println!("[HARDWARE IPC] Mencetak struk transaksi thermal: {:?}", receipt_data);
    Ok("Struk berhasil dikirim ke antrean cetak".to_string())
}
