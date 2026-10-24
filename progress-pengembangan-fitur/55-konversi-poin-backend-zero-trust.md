# Progress 55: Konversi Poin Kasir POS Sisi Backend (Zero-Trust Security)

## Ringkasan Perubahan
Memperbaiki alur penggunaan poin reward saat checkout di Kasir (POS) agar mengikuti prinsip Zero-Trust Security. Nominal rupiah konversi poin tidak lagi dipercayai dari payload frontend, melainkan dihitung dan divalidasi langsung oleh backend berdasarkan konfigurasi resmi di database (`loyalty_settings.point_value_rupiah`).

## Detail Implementasi

### 1. DTO Model (`CheckoutPaymentInput`)
- File: `notane-server/src-tauri/src/models/sales.rs` & `notane-client/src-tauri/src/models/sales.rs`
- Menambahkan field `points: Option<i32>` dan anotasi `#[serde(default)] pub amount: Decimal`.
- Metode non-poin tetap mengirimkan `amount`, sedangkan metode poin mengirimkan `points` dan `amount: 0`.

### 2. Validasi Endpoint (`checkout_handler`)
- File: `notane-server/src-tauri/src/api/pos.rs` & `notane-client/src-tauri/src/api/pos.rs`
- Mengimpor `rust_decimal::prelude::*`.
- Memvalidasi secara spesifik:
  - Untuk metode `POINTS`: jumlah poin yang ditukarkan wajib $> 0$ poin.
  - Untuk metode pembayaran lainnya: `amount` wajib $> 0$.
  - Pelanggan (`customer_id`) wajib terdaftar jika menggunakan poin.

### 3. Logika Domain POS (`pos_service.rs`)
- File: `notane-server/src-tauri/src/services/pos_service.rs` & `notane-client/src-tauri/src/services/pos_service.rs`
- Menyelesaikan seluruh entri pembayaran ke struct `ResolvedPayment` **sebelum** validasi `total_paid < final_total`:
  - Mengambil konfigurasi aktif `loyalty_settings`.
  - Mengonversi jumlah poin ke nominal rupiah: $\text{point\_amount} = \text{points} \times \text{loyalty.point\_value\_rupiah}$.
  - Memvalidasi batas persentase maksimal penggunaan poin (`loyalty.max_point_usage_percent`).
  - Memvalidasi batas minimal penukaran (`loyalty.min_redeem_points`).
  - Mengunci baris member (`SELECT reward_points FROM customers ... FOR UPDATE`) untuk memastikan saldo riil mencukupi.
- Memotong poin member secara atomik dan mencatat riwayat transaksi ke `customer_point_ledgers` jenis `REDEEMED`.
- Mengembalikan nominal konversi resmi pada `response_payments` struk belanja.

### 4. Frontend Kasir (`QuickPayModal.vue` & `types/index.ts`)
- File: `notane-client/src/components/pos/QuickPayModal.vue` & `notane-client/src/types/index.ts` (tersinkron ke `notane-server/src/`)
- Menambahkan field `points?: number` pada tipe `PaymentEntry`.
- Pada saat checkout, payload `payments` untuk kategori `POINTS` mengirimkan `{ payment_method_id, amount: 0, points: appliedPoints.value }`.

## Verifikasi & Pengujian
- Pengujian E2E otomatis melalui Browser Subagent:
  - Pelanggan: Budi Santoso (`081299887766`)
  - Produk: 2 pcs Indomie Goreng Spesial (Net: Rp 6.660)
  - Poin yang ditukarkan: 33 poin (Backend mengonversi menjadi Rp 3.300)
  - Pembayaran tunai: Rp 5.000 (Kembalian Rp 1.640)
  - Hasil: Transaksi checkout sukses tanpa celah manipulasi harga, saldo poin member terpotong, dan struk tercetak dengan nilai konversi resmi.
