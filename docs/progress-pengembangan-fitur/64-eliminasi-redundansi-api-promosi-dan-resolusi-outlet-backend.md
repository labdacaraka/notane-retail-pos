# 64. Eliminasi Redundansi API Promosi di Admin & Resolusi Otomatis Outlet di Backend

## Latar Belakang & Masalah
Ketika pengguna mengakses menu back-office admin (seperti `http://localhost:5175/admin/audit-logs`), terpantau terjadi pemanggilan berulang ke endpoint:
* `GET /api/v1/promotions/active?outlet_id=...`
* `GET /api/v1/flash-sales/active-promos?outlet_id=...`

Padahal:
1. Halaman admin / back-office (seperti Audit Logs, Pengaturan, dsb.) tidak memerlukan data promosi kasir.
2. Parameter query `?outlet_id=...` seharusnya tidak perlu dikirim secara eksplisit oleh klien dari frontend. Backend server dapat secara otomatis menentukan `outlet_id` efektif berdasarkan sesi shift kasir yang sedang terbuka (`cashier_shifts` dengan status `OPEN`), atau preferensi outlet aktif pengguna (`users.current_outlet_id`).

---

## Solusi & Implementasi Teknis

### 1. Eliminasi Pemanggilan Redundan di Frontend (Notane Client)
1. **Pemisahan di `locationStore.ts`** ([locationStore.ts](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-client/src/stores/locationStore.ts)):
   - Menghapus sinkronisasi otomatis `cartStore.setOutlet()` saat fungsi `fetchAccessibleOutlets()` dipanggil oleh `AdminLayout`.
   - Pada fungsi `switchLocation()`, sinkronisasi ke `cartStore` hanya dieksekusi jika pengguna sedang berada di halaman kasir (`/pos`).
2. **Proteksi Rute & Eliminasi Query Parameter di `cartStore.ts`** ([cartStore.ts](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-client/src/stores/cartStore.ts)):
   - Menambahkan penjaga rute pada fungsi `fetchActivePromos()`: jika URL bukan berawalan `/pos` (berada di area admin), fungsi langsung keluar tanpa melakukan HTTP request ke server.
   - Menghapus parameter query `{ params: { outlet_id: ... } }` pada pemanggilan:
     - `api.get('/flash-sales/active-promos')`
     - `api.get('/promotions/active')`

### 2. Resolusi Otomatis Outlet di Backend (Notane Server)
File: `notane-server/src-tauri/src/utils/jwt.rs`
- Dibuat fungsi pembantu `resolve_effective_outlet_id`:
  ```rust
  pub async fn resolve_effective_outlet_id(
      pool: &sqlx::PgPool,
      headers: &axum::http::HeaderMap,
      provided_outlet_id: Option<Uuid>,
  ) -> Option<Uuid>
  ```
  **Hierarki Resolusi:**
  1. Jika klien secara eksplisit mengirimkan `outlet_id`, gunakan nilai tersebut.
  2. Jika tidak dikirimkan, periksa apakah pengguna memiliki shift kasir yang sedang aktif:
     ```sql
     SELECT outlet_id FROM cashier_shifts 
     WHERE cashier_id = $1 AND status = 'OPEN' 
     ORDER BY start_time DESC LIMIT 1
     ```
  3. Jika tidak ada open shift, ambil preferensi lokasi outlet aktif pengguna:
     ```sql
     SELECT current_outlet_id FROM users WHERE id = $1
     ```
  4. Fallback ke outlet ritel aktif pertama di sistem jika data belum disetel.

### 3. Integrasi Handler API Promosi & Flash Sale
1. **Endpoint Promosi Ritel** ([promotions.rs](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/api/promotions.rs)):
   - `get_active_promotions_handler` membaca header request dan menggunakan `resolve_effective_outlet_id` untuk menyaring promosi yang berlaku untuk outlet terkait.
2. **Endpoint Flash Sale** ([flash_sales.rs](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/api/flash_sales.rs)):
   - `active_promos_handler` membaca header request dan menyalurkan outlet efektif ke fungsi `get_active_promos_for_outlet`.

---

## Verifikasi Hasil Pengujian
1. Akses menu admin (seperti `http://localhost:5175/admin/audit-logs`) kini bersih dari pemanggilan API promosi & flash sales.
2. Pemanggilan endpoint `GET /api/v1/promotions/active` dan `GET /api/v1/flash-sales/active-promos` tanpa query parameter `outlet_id` berhasil mengembalikan respon `200 OK` dengan resolusi otomatis dari backend.
