# Dokumentasi Fitur: Riwayat Poin Pelanggan & Penyesuaian Manual (Batch 20)

## Ringkasan Fitur
Fitur ini menambahkan integrasi lengkap untuk pencatatan, penelusuran riwayat (history), serta penyesuaian (koreksi) manual saldo poin loyalti pelanggan/member pada aplikasi Notane Retail POS.

---

## 1. Database & Migrasi
- **Tabel Skema**: Tabel `customer_point_ledgers` yang telah ada digunakan sebagai single-source-of-truth pencatatan mutasi poin:
  - `id`: BIGSERIAL (Primary Key)
  - `customer_id`: UUID (Foreign Key ke `customers.id`)
  - `transaction_type`: `VARCHAR(30)` (`EARNED` = Masuk dari belanja kasir, `REDEEMED` = Keluar untuk potongan nota kasir, `CORRECTION` = Koreksi/penyesuaian manual, `EXPIRED` = Kedaluwarsa)
  - `reference_id`: UUID (Foreign Key ke `sales.id` untuk transaksi belanja POS)
  - `points_in`: INT NOT NULL DEFAULT 0
  - `points_out`: INT NOT NULL DEFAULT 0
  - `balance`: INT NOT NULL (Saldo poin setelah mutasi)
  - `notes`: TEXT (Keterangan mutasi)
  - `created_at`: TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
  - `created_by`: UUID (Foreign Key ke `users.id`)
- **Migrasi Baru**: [`src-tauri/migrations/030_customer_point_ledgers_indices.sql`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/migrations/030_customer_point_ledgers_indices.sql)
  - Menambahkan indeks B-tree pada `customer_id`, `created_at DESC`, `reference_id`, dan `transaction_type`.
  - Berhasil dieksekusi ke PostgreSQL dan dicatat sebagai **Batch 20**.

---

## 2. Backend API (Rust / Axum)
Diimplementasikan di [`src-tauri/src/api/master.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/src/api/master.rs):

1. **`GET /api/v1/customers/point-ledgers`** *(Tabel Riwayat Mutasi Poin Global)*:
   - Menyediakan query lintas pelanggan untuk tabel riwayat mutasi poin dengan server-side paging (`page`, `page_size`, `total_items`, `total_pages`).
   - Filter parameter: `search` (nama member, kode, nomor telepon, no. nota faktur POS, dan keterangan), `transaction_type` (`EARNED`, `REDEEMED`, `CORRECTION`), `start_date`, dan `end_date`.
   - Mengembalikan ringkasan statistik global: `total_active_points`, `total_earned`, dan `total_redeemed`.

2. **`GET /api/v1/customers/{id}/point-ledgers`** *(Riwayat Poin Per Pelanggan)*:
   - Mendukung pagination (`page`, `page_size`) dan filter tipe transaksi (`transaction_type`: `ALL`, `EARNED`, `REDEEMED`, `CORRECTION`).
   - Menyertakan data referensi nomor nota (`sales.invoice_number`) dan nama operator/kasir yang melayani (`users.full_name`).
   - Mengembalikan ringkasan statistik pelanggan: `current_balance`, `total_earned`, dan `total_redeemed`.

3. **`POST /api/v1/customers/{id}/adjust-points`**:
   - Diproteksi otorisasi peran: `customers:manage`, `loyalty:manage`, atau `master:write`.
   - Validasi semantik backend:
     - `adjustment_type` wajib bernilai `"ADD"` atau `"DEDUCT"`.
     - `points` wajib `> 0`.
     - `notes` wajib diisi tidak boleh kosong.
     - Penolakan bila pemotongan (`DEDUCT`) melebihi sisa saldo poin pelanggan yang ada saat ini.
   - Menggunakan transaksi SQL dengan row-level lock (`SELECT ... FOR UPDATE`) untuk menjamin konsistensi saldo.
   - Otomatis mencatat baris mutasi `CORRECTION` ke `customer_point_ledgers` dengan id user operator yang melakukan perubahan.

---

## 3. Frontend UI (Vue.js + Tailwind)

1. **Tab Khusus: Riwayat Mutasi Poin (Tabel dengan Paging Penuh)**:
   - Disediakan tab baru **`Riwayat Mutasi Poin (N)`** pada halaman [`src/pages/CustomersPage.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/pages/CustomersPage.vue).
   - Dirancang khusus untuk menangani volume data besar (*large datasets*) dengan server-side paging (`PaginationBar`).
   - Pilihan baris per halaman: 10, 25, 50, 100 data per halaman.
   - Dilengkapi 3 kartu statistik di bagian atas tab:
     1. *Total Saldo Poin Beredar* (Poin aktif seluruh member).
     2. *Total Poin Diberikan* (Akumulasi reward belanja kasir).
     3. *Total Poin Ditukarkan* (Total potongan nota yang telah digunakan).
   - Dilengkapi filter pencarian instan (nama/kode/nota), filter jenis mutasi (Belanja, Penukaran, Koreksi), dan rentang tanggal transaksi.

2. **Komponen Modal Riwayat Per Pelanggan**:
   - File: [`src/components/customers/CustomerPointHistoryModal.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/components/customers/CustomerPointHistoryModal.vue)
   - Menampilkan detail historis mutasi member terkait dengan kontrol paging (opsi 5, 10, 20, 50 baris per halaman) serta form penyesuaian/koreksi poin manual.
