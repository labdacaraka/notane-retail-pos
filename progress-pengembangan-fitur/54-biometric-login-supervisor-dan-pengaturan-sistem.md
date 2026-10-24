# Progress 54: Biometric Login Otorisasi Supervisor (Sidik Jari & Pengenalan Wajah) dan Integrasi Penuh Fitur POS

## Ringkasan Fitur
Implementasi sistem verifikasi biometrik terpadu untuk otorisasi supervisor pada seluruh alur kasir POS dan back-office **Notane Retail POS** (`notane-client`) dan backend **Notane Retail Server** (`notane-server`). Sistem ini mengizinkan otorisasi menggunakan sensor sidik jari (WebAuthn / Touch ID / Windows Hello) serta pengenalan wajah langsung melalui kamera terminal kasir POS, didukung oleh pengaturan mode fleksibel (`OFF`, `FINGERPRINT`, `FACE_RECOGNITION`, `ANY`) dan kebijakan (`OPTIONAL` vs `STRICT`).

---

## 1. Komponen Reusable Otorisasi Supervisor (`SupervisorAuthSection.vue`)
Dibuat komponen antarmuka mandiri [SupervisorAuthSection.vue](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-client/src/components/pos/SupervisorAuthSection.vue) yang diintegrasikan ke seluruh modal dan alur operasional:
1. **Deteksi Mode & Kebijakan Dinamis:**
   - Membaca pengaturan biometrik aktif (`supervisor_biometric_mode`, `supervisor_biometric_policy`, `face_match_threshold`).
   - Jika mode `FINGERPRINT` atau `ANY`: menampilkan tombol cepat **"Sentuh Sidik Jari"**.
   - Jika mode `FACE_RECOGNITION` atau `ANY`: menampilkan tombol **"Pindai Wajah"**.
   - Jika policy `STRICT`: menonaktifkan input PIN keypad dengan notifikasi kepatuhan keamanan.
   - Jika policy `OPTIONAL` atau `OFF`: mengizinkan fallback input 6-digit PIN.
2. **Status Terverifikasi (Verified Banner):**
   - Menampilkan kartu indikator hijau dengan ikon dan detail: `"✓ Terotorisasi: [Nama Supervisor] ([Metode: Sidik Jari / Wajah / PIN])"`.
   - Tombol ganti/ulang otorisasi.
3. **Pemindai Kamera Live Inline HUD:**
   - Overlay frame target wajah beranimasi.
   - Deteksi real-time dan perhitungan skor kemiripan (similarity score).
   - Pengiriman otomatis untuk otorisasi biometrik supervisor.

---

## 2. Fitur Kasir & Operasional yang Terintegrasi
1. **Buka Shift Kasir (`ShiftModal.vue` - OPEN):**
   - Kasir memilih cabang dan memasukkan modal awal floating cash.
   - Otorisasi supervisor penanggung jawab buka shift kini mendukung pilihan **Sensor Sidik Jari**, **Pindai Wajah**, atau PIN.
   - Mengirimkan `supervisor_auth_token` terenkripsi ke backend `/shifts/open`.
2. **Tutup Shift & Settlement (`ShiftModal.vue` - CLOSE):**
   - Kasir menghitung uang fisik di laci (blind count).
   - Approval tutup shift dan cetak Z-Report diverifikasi lewat biometrik supervisor atau PIN.
3. **Setor Kas Laci / Cash Drop (`CashDropModal.vue`):**
   - Serah terima fisik uang ke brankas toko diotorisasi langsung dengan sensor sidik jari atau kamera wajah supervisor.
4. **Retur Penjualan POS (`SalesReturnModal.vue`):**
   - Pengembalian barang dan uang tunai/kredit toko diverifikasi oleh supervisor menggunakan biometrik.
5. **Void / Pembatalan Faktur Penjualan (`SalesPage.vue`):**
   - Modal pembatalan nota penjualan kini dilengkapi pemindai biometrik supervisor terpadu.
6. **Buka Cash Drawer & Kosongkan Keranjang (`PosRegister.vue`):**
   - Buka laci kas tanpa transaksi (`OPEN_DRAWER`) dan pengosongan keranjang belanja (`CLEAR_CART`) dilindungi otorisasi biometrik supervisor.

---

## 3. Keamanan Backend & Zero-Trust Protocol
1. **Penerbitan Token Otorisasi Supervisor:**
   - Setiap keberhasilan verifikasi sidik jari atau wajah pada `/auth/supervisor-biometric-approve` menghasilkan short-lived cryptographically signed `auth_token` JWT.
2. **Fungsi Sentral `verify_supervisor_auth` (`services/auth_service.rs`):**
   - Memvalidasi keabsahan token JWT supervisor atau verifikasi hash PIN Argon2id.
   - Memastikan akun yang menyetujui memiliki hak akses peran (`SUPERVISOR`, `STORE_MANAGER`, atau `SUPERADMIN`).
   - Mencegah bypass: jika kebijakan toko `STRICT`, otorisasi dengan PIN manual langsung ditolak fail-closed.
3. **Dukungan API Endpoints:**
   - `/shifts/open` & `/shifts/close`: Mendukung `supervisor_auth_token`.
   - `/pos/cash-drops`: Mendukung `supervisor_auth_token`.
   - `/sales/:id/void`: Mendukung `supervisor_auth_token`.
   - `/sales/returns`: Mendukung `supervisor_auth_token`.

---

## 4. Hasil Pengujian Live (End-to-End)
- [x] Otorisasi Biometrik Sidik Jari untuk Buka Shift berhasil dicatat (`supervisor_approval_id` tersimpan di database).
- [x] Otorisasi Biometrik Sidik Jari untuk Tutup Shift berhasil mencatat settlement dan menghitung discrepancy.
- [x] Otorisasi Biometrik untuk Cash Drop berhasil mencatat nomor drop dan nama supervisor.
- [x] Searchable Select2 Cabang & Lokasi Shift terhubung langsung ke Server-Side API `/me/outlets` dengan live filtering.
- [x] Searchable Select2 Supervisor terfilter dinamis per cabang yang dipilih (`/auth/supervisors?outlet_id=...&search=...`).
- [x] Build frontend `notane-client` dan `notane-server` lolos dengan 0 error type check.
- [x] Container server backend `notane-riang-market-server` berjalan healthy dengan biner terbaru.

