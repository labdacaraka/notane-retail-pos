# 29 — Kolom Foto Produk & Sistem Import Canggih

**Tanggal**: 2026-09-26  
**Status**: ✅ SELESAI (Full-stack verified via API test)

---

## Ringkasan Fitur

Menambahkan kolom `image_url` pada produk dan membangun sistem import massal produk canggih yang mencakup:
- Upload & tampil foto produk di seluruh UI (tabel, modal buat/edit, kasir search)
- Template Excel/CSV dengan semua outlet & kelompok pelanggan aktif
- Deteksi foto otomatis dari nama file, SKU, Barcode, atau Nama Produk
- Ekstraksi foto dari arsip ZIP
- Manajemen harga multi-outlet dan harga khusus member (customer group)
- Auto-provisioning kategori & satuan baru jika belum ada
- Pratinjau live sebelum eksekusi import

---

## Perubahan Database

### Migration 013_product_images_and_advanced_import.sql
```sql
ALTER TABLE products ADD COLUMN IF NOT EXISTS image_url TEXT;
```
- Terdaftar di `src-tauri/src/db/mod.rs` sebagai embedded migration
- Dieksekusi otomatis saat startup (Batch 3, setelah `012_license_system`)

### Migration 014_fix_product_image_storage.sql
```sql
DROP INDEX IF EXISTS idx_products_image_url;
UPDATE products SET image_url = NULL WHERE image_url LIKE 'data:image/%';
```
- **Masalah**: PostgreSQL B-tree index membatasi ukuran baris index maksimal 8191 bytes. Ketika foto disimpan sebagai string base64 (>400KB), PostgreSQL mengembalikan error `index row requires 463944 bytes, maximum size is 8191`.
- **Solusi**: Drop index `idx_products_image_url` dan ubah arsitektur penyimpanan foto: file gambar disimpan ke filesystem lokal (`~/.notane/product_images/`), database hanya menyimpan relative URL path pendek (misal: `/uploads/product_images/kopi-8c37035f.png`), dan backend melayani file statis via `tower-http::services::ServeDir`.
- Dieksekusi otomatis saat startup (Batch 4).

---

## Arsitektur Penyimpanan & Penyajian Foto Produk

1. **Upload Handler & File System**:
   - Foto dikirim sebagai data URL / base64 ke endpoint `POST /api/v1/products/upload-photo`
   - Backend mendecode base64 menjadi raw binary bytes dan menyimpannya ke direktori `~/.notane/product_images/<slug>-<uuid8>.<ext>`
   - Endpoint mengembalikan relative URL path `/uploads/product_images/<filename>` yang hemat ruang di DB (< 60 karakter)
2. **Auto-Convert pada Import Produk**:
   - Endpoint `POST /api/v1/products/import` secara otomatis mendeteksi apakah `image_url` berformat `data:image/` (base64 dari ZIP / Excel)
   - Jika iya, file otomatis diekstrak dan disimpan ke filesystem, dan DB hanya menyimpan relative path-nya
3. **Penyajian Statis (Static File Serving)**:
   - Backend Axum me-mount `ServeDir` pada route `/uploads/product_images`
   - `vite.config.ts` ditambahkan proxy rule `/uploads` yang mengarah ke `http://127.0.0.1:18080`
   - Frontend `ProductsPage.vue` menggunakan helper `resolveImageUrl()` untuk memastikan path gambar dapat dimuat di dev server maupun standalone client

---

## Perubahan Backend (Rust)

### src-tauri/Cargo.toml
- Tambahkan feature `"fs"` pada crate `tower-http`

### src-tauri/src/api/mod.rs
- Mount `ServeDir::new(product_images_dir)` pada `/uploads/product_images`

### src-tauri/src/api/master.rs
- Helper `save_product_image_from_base64`:
  - Mendecode base64 string
  - Deteksi ekstensi gambar (PNG, WEBP, GIF, JPG)
  - Simpan ke `~/.notane/product_images/`
- `upload_product_photo_handler`: Upload foto mandiri
- `import_products_handler`: Auto-convert foto base64 menjadi file fisik saat import massal

---

## Perubahan Frontend (Vue 3 / TypeScript)

### src/pages/ProductsPage.vue
- Kolom **Foto** (40×40px thumbnail) dengan fallback icon `fluent-color:image-24` dan label "No Image"
- Helper `resolveImageUrl()` untuk resolving relative path backend
- Indikator loading (spinner + disabled state) pada tombol upload foto saat file sedang diunggah
- Modal preview foto resolusi penuh saat thumbnail diklik
- Preview foto responsif pada Modal Tambah dan Modal Edit Produk

---

## Status Pengujian & Verifikasi

- **Upload Foto via API**: Berhasil (HTTP 200, file PNG tersimpan 70 bytes di `~/.notane/product_images/`)
- **Static File Serving**:
  - Direct via Backend (`http://127.0.0.1:18080/uploads/product_images/...`): HTTP 200 OK
  - Via Vite Proxy (`http://localhost:5175/uploads/product_images/...`): HTTP 200 OK
- **Update Master Produk dengan Foto**: Berhasil disimpan tanpa error limit database
- **Import Massal dengan Foto Base64**: Berhasil diekstraksi ke filesystem dan path tersimpan di DB
- **Frontend Build**: `npm run build` sukses (exit code 0)
- **Backend Build**: `cargo build` sukses (exit code 0)

---

## Port Terisolasi (tidak berubah)

| Service | Port |
|---------|------|
| PostgreSQL embedded | 15432 |
| Backend POS API | 18080 |
| Host Control Channel | 18081 |
| Vite Dev Server | 5175 |

