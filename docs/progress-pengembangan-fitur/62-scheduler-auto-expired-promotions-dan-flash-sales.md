# 62. Scheduler Otomatis Status Nonaktif Promo (Promotions) dan Flash Sales

## Latar Belakang & Masalah
Sebelumnya, penonaktifan masa berlaku pada fitur diskon promosi (`/admin/promotions`) dan flash sale (`/admin/flash-sales`) masih bersifat manual atau statis, sehingga apabila tanggal/waktu akhir periode promosi telah lewat, status item di database dan tabel UI masih tetap dapat berstatus "Aktif" jika belum diubah manual oleh pengguna. 

Diperlukan sistem otomatisasi (scheduler di background dan validasi proaktif backend) untuk mengubah status promo dan flash sale menjadi nonaktif (`is_active = false`) seketika masa berlakunya telah habis, serta mengirimkan event WebSocket agar halaman admin client terbarui secara realtime tanpa perlu refresh manual.

---

## Solusi & Implementasi Teknis

### 1. Unified Background Scheduler (Notane Server Backend)
File: `notane-server/src-tauri/src/api/ws.rs`
- Fungsi loop background scheduler (`start_voucher_expiry_scheduler`) diperluas menjadi pemantau kedaluwarsa menyeluruh untuk:
  1. **Voucher**: Mengubah `status = 'EXPIRED'` jika `valid_until <= CURRENT_TIMESTAMP` dan berstatus `ACTIVE`. Mengirim sinyal `VOUCHERS_EXPIRED`.
  2. **Promotions (Promo Ritel)**: Mengubah `is_active = false, updated_at = CURRENT_TIMESTAMP` jika `is_active = true AND end_date <= CURRENT_TIMESTAMP`. Mengirim sinyal broadcast WebSockets `PROMOTIONS_EXPIRED`.
  3. **Flash Sales (Flash Sale)**: Mengubah `is_active = false, updated_at = CURRENT_TIMESTAMP` jika `is_active = true` dan:
     - `end_date < CURRENT_DATE`, ATAU
     - `end_date = CURRENT_DATE AND is_all_day = false AND end_time IS NOT NULL AND end_time < CURRENT_TIME`.
     Mengirim sinyal broadcast WebSockets `FLASH_SALES_EXPIRED`.
- Scheduler berjalan periodik setiap 5 menit di background thread yang dikelola oleh `tokio::time::interval`.

### 2. Evaluasi Proaktif (Zero-Trust Backend Handlers)
Selain menunggu interval scheduler 5 menit, API backend juga menerapkan evaluasi proaktif real-time:
- **Promotions Handlers** (`notane-server/src-tauri/src/api/promotions.rs`):
  - `list_promotions_handler`: Menjalankan query update proaktif atas promo yang sudah lewat masa berlaku tepat sebelum mengambil daftar (`SELECT`), memastikan data selalu mutakhir seketika dibuka.
  - `create_promotion_handler` & `update_promotion_handler`: Mengecek apakah `payload.end_date <= Utc::now()`. Jika waktu sudah terlewati, backend memaksa `is_active = false`.
- **Flash Sale Service** (`notane-server/src-tauri/src/services/flash_sale_service.rs`):
  - `list_flash_sales`: Menjalankan query update proaktif atas flash sales yang sudah melewati tanggal atau jam berlaku tepat sebelum menghitung total dan mengambil pagination.
  - `create_flash_sale` & `update_flash_sale`: Mengevaluasi waktu akhir (`end_date` dan `end_time`). Jika sudah terlewat, otomatis menetapkan `is_active = false`.

### 3. Sinkronisasi Real-Time Frontend (Vue 3 / Notane Client)
- **Halaman Promotions** (`notane-client/src/pages/PromotionsPage.vue`):
  - Menambahkan listener event window `notane-ws-message` untuk aksi `PROMOTIONS_EXPIRED`, sehingga ketika scheduler mendeteksi promo kedaluwarsa, tabel promosi otomatis memuat ulang (`fetchPromotions()`).
  - Menambahkan validasi saat menyimpan (`savePromotion`): jika `end_date` yang dipilih berada di masa lalu, form otomatis mengeset status menjadi Nonaktif.
- **Halaman Flash Sales** (`notane-client/src/pages/FlashSalesPage.vue`):
  - Menambahkan listener event window `notane-ws-message` untuk aksi `FLASH_SALES_EXPIRED`, sehingga saat event dipancarkan oleh server, tabel otomatis melakukan refresh (`fetchFlashSales()`).
  - Menambahkan validasi pada `handleSave`: jika tanggal/jam akhir flash sale berada di masa lalu, otomatis mengubah `is_active` menjadi nonaktif.

---

## Verifikasi & Hasil Pengujian
1. **Startup Server Daemon**:
   - Log server langsung mendeteksi event yang kadaluwarsa saat dinyalakan:
     ```
     [PROMO SCHEDULER] Background scheduler kedaluwarsa promo, voucher, dan flash sale aktif (Interval 5 menit).
     [PROMO SCHEDULER] 2 event flash sale otomatis dinonaktifkan karena waktu promo telah habis.
     ```
2. **Kepatuhan Arsitektur**:
   - Seluruh logika bisnis, database queries, dan scheduler berjalan di `notane-server`.
   - `notane-client` mengonsumsi data via REST API dan subscribe ke event WebSocket terpusat.
