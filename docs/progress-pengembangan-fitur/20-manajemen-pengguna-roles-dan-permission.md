# 20 - Manajemen Pengguna, Roles, dan Permission (RBAC)

**Tanggal**: 2026-09-20  
**Status**: ✅ SELESAI  
**Kategori**: Back-Office - Keamanan Sistem

---

## Ringkasan Implementasi

Fitur ini menambahkan sistem manajemen pengguna dan hak akses berbasis peran (Role-Based Access Control / RBAC) yang lengkap ke dalam aplikasi Back-Office. Seluruh fungsi CRUD untuk users, roles, dan permission matrix kini tersedia.

---

## Fitur yang Diimplementasikan

### 1. Backend (Rust Axum) — `src-tauri/src/api/users.rs`

**API Endpoints Users:**
- `GET /api/v1/users` — List semua pengguna (mendukung filter: search, role_id, is_active)
- `GET /api/v1/users/{id}` — Detail pengguna beserta permissions dari role-nya
- `POST /api/v1/users` — Buat pengguna baru (password & PIN di-hash Argon2id otomatis)
- `PUT /api/v1/users/{id}` — Update data pengguna (nama, role, kontak)
- `DELETE /api/v1/users/{id}` — Soft-delete (nonaktifkan, tidak dihapus dari DB)
- `POST /api/v1/users/{id}/reset-password` — Reset password + opsional PIN
- `PUT /api/v1/users/{id}/toggle-active` — Aktifkan/nonaktifkan pengguna

**API Endpoints Roles:**
- `GET /api/v1/roles` — List semua role (termasuk jumlah pengguna & permission)
- `GET /api/v1/roles/{id}` — Detail role + daftar permissions yang dimiliki
- `POST /api/v1/roles` — Buat role baru
- `PUT /api/v1/roles/{id}` — Update nama & deskripsi role
- `DELETE /api/v1/roles/{id}` — Soft-delete role (dilindungi jika masih ada user aktif)
- `GET /api/v1/roles/{id}/permissions` — Matrix permission untuk role tertentu (all permissions + has_permission flag)
- `PUT /api/v1/roles/{id}/permissions` — Set/replace seluruh permission untuk sebuah role (atomic transaction)

**API Endpoints Permissions:**
- `GET /api/v1/permissions` — List semua permission yang tersedia di sistem

### 2. Frontend (Vue 3) — `src/pages/UsersPage.vue`

**Tab 1: Pengguna**
- Tabel pengguna dengan search real-time, filter role, dan filter status aktif/nonaktif
- Kolom: Avatar inisial, Nama Lengkap, @Username, Role badge berwarna, Kontak, Status, Tanggal dibuat
- Aksi per baris: Edit data, Reset password/PIN, Toggle aktif/nonaktif
- Modal tambah pengguna baru (validasi password match, PIN minimal 4 digit)
- Modal edit pengguna (username read-only setelah dibuat)
- Modal reset password dengan opsional reset PIN supervisor

**Tab 2: Roles & Permissions**
- Panel kiri: Daftar role dengan jumlah pengguna & hak akses, tombol buat/edit/hapus role
- Panel kanan: Permission matrix (checkbox) dikelompokkan per modul sistem (POS, Shift, Inventory, dll)
- SUPERADMIN ditampilkan dengan indikator khusus (tidak bisa dimodifikasi)
- Tombol "Simpan Perubahan" untuk atomic save seluruh permissions role
- Role baru dapat dibuat langsung dari modal

---

## File yang Dibuat / Dimodifikasi

| File | Perubahan |
|------|-----------|
| `src-tauri/src/api/users.rs` | **[BARU]** Module API lengkap untuk users, roles, permissions |
| `src-tauri/src/api/mod.rs` | Register `pub mod users` + `users::user_management_routes()` ke router |
| `src/pages/UsersPage.vue` | **[BARU]** Halaman management users, roles & permission matrix |
| `src/router/index.ts` | Tambah route `/admin/users` |
| `src/layouts/AdminLayout.vue` | Tambah menu "Pengguna & Hak Akses" di grup config |
| `src/pages/AdminDashboard.vue` | Tambah tile "Pengguna & Hak Akses" di seksi keamanan |

---

## Role Default di Sistem

| Role | Deskripsi | Permission Utama |
|------|-----------|-----------------|
| SUPERADMIN | Akses penuh ke seluruh sistem | Semua permission |
| STORE_MANAGER | Manajer toko level tinggi | Semua kecuali system config |
| SUPERVISOR | Otorisasi void, diskon, shift | pos:*, shift:*, inventory:read, report:view |
| CASHIER | Kasir operasional | pos:checkout, shift:open/close |
| WAREHOUSE | Staf gudang | inventory:*, purchasing:* |

---

## Keamanan

- Password & PIN selalu di-hash menggunakan **Argon2id** sebelum disimpan ke database
- Soft-delete: pengguna tidak pernah dihapus permanen (data audit tetap utuh)
- Role SUPERADMIN: permission matrix tidak bisa diubah (proteksi UI + logika)
- Role yang masih digunakan pengguna aktif tidak dapat dihapus

---

## Catatan Build

- ✅ Frontend: Vite build berhasil (0 error)  
- ✅ Backend: cargo check berhasil (0 error, 1 warning minor sudah diperbaiki)
- ✅ Seluruh 22 icon yang digunakan sudah divalidasi tersedia di @iconify-json/fluent-color
