# 22 - Manajemen PIN Otorisasi & Pemilihan Supervisor Penanggung Jawab

**Tanggal**: 2026-09-21  
**Status**: ✅ SELESAI  
**Kategori**: POS & Back-Office - Keamanan & Otorisasi

---

## Ringkasan Implementasi

Fitur ini menyempurnakan alur otorisasi supervisor di kasir POS dan manajemen keamanan akun staf. Kini setiap tindakan operasional kritis (seperti pembatalan/void transaksi penjualan, pembukaan shift, dan penutupan shift) mengharuskan kasir memilih supervisor penanggung jawab secara spesifik dan memvalidasi PIN supervisor yang dipilih, bukan sekadar PIN anonim. Selain itu, pengguna terautentikasi dapat mengganti password dan PIN mereka sendiri secara mandiri, serta admin dapat mengatur hak akses siapa saja yang berhak mengubah PIN pengguna lain.

---

## Fitur yang Diimplementasikan

### 1. Hak Akses & Manajemen PIN Mandiri
- **Permission Baru**: Ditambahkan permission `users:change_pin` pada skema migrasi `004_pin_management_permission.sql`.
- **Backend Endpoints**:
  - `POST /api/v1/auth/change-password` — Mengubah password akun sendiri dengan verifikasi password lama.
  - `POST /api/v1/auth/change-pin` — Mengubah PIN otorisasi akun sendiri dengan verifikasi password login.
  - `POST /api/v1/users/{id}/change-pin` — Mengubah PIN pengguna lain oleh admin/supervisor berizin.
  - `GET /api/v1/auth/supervisors` — Mendapatkan daftar staf yang memiliki izin supervisor/store manager untuk otorisasi transaksi.
- **Frontend Komponen & Modal**:
  - `UserProfileSecurityModal.vue` — Modal profil mandiri yang dapat dibuka dari navbar kasir (`PosLayout.vue`) atau backoffice (`AdminLayout.vue`) untuk mengganti password dan PIN kasir secara mandiri.
  - Tombol aksi PIN di `UsersPage.vue` dengan ikon dial-pad yang responsif terhadap izin `users:change_pin` atau `SUPERADMIN`.

### 2. Otorisasi Supervisor pada Pembatalan Transaksi Penjualan
- **Perbaikan Payload Void**:
  - Memperbaiki deserialisasi payload pembatalan penjualan (`CancelSalePayload`) agar `reason_id` opsional dan catatan supervisor tercatat rapi.
- **Modal Pembatalan Transaksi (`PosRegister.vue`)**:
  - Menampilkan dropdown daftar supervisor penanggung jawab (`GET /api/v1/auth/supervisors`).
  - Input PIN supervisor tervalidasi langsung terhadap akun supervisor yang dipilih.
  - Alasan pembatalan dan supervisor penanggung jawab tersimpan permanen di log transaksi.

### 3. Pemilihan Supervisor pada Shift Kasir (`ShiftModal.vue`)
- **Buka Shift (`OPEN`)**:
  - Kasir memilih supervisor penanggung jawab yang bertugas dan memasukkan PIN supervisor tersebut.
- **Tutup Shift (`CLOSE / Blind Settlement`)**:
  - Penutupan shift kasir mewajibkan kehadiran supervisor penanggung jawab yang diverifikasi dengan PIN.

---

## File yang Dibuat / Dimodifikasi

1. **Backend**:
   - `src-tauri/migrations/004_pin_management_permission.sql` [BARU]
   - `src-tauri/src/api/auth.rs` [MODIFIKASI]
   - `src-tauri/src/api/users.rs` [MODIFIKASI]
   - `src-tauri/src/api/sales.rs` [MODIFIKASI]
   - `src-tauri/src/services/pos_service.rs` [MODIFIKASI]
2. **Frontend**:
   - `src/components/common/UserProfileSecurityModal.vue` [BARU]
   - `src/components/pos/ShiftModal.vue` [MODIFIKASI]
   - `src/pages/PosRegister.vue` [MODIFIKASI]
   - `src/pages/UsersPage.vue` [MODIFIKASI]
   - `src/layouts/PosLayout.vue` [MODIFIKASI]
   - `src/layouts/AdminLayout.vue` [MODIFIKASI]
