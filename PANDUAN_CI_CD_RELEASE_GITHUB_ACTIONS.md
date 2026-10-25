# Panduan Rilis Otomatis Multi-Platform (GitHub Actions OTA Builder)

Dokumen ini menjelaskan cara menggunakan **GitHub Actions** untuk mem-build dan menghasilkan bundle rilis **Notane Client** dan **Notane Server** secara otomatis untuk seluruh sistem operasi (**Windows, Linux, macOS Intel, & macOS Apple Silicon**) tanpa harus berganti-ganti komputer.

---

## 1. Arsitektur File yang Dihasilkan Otomatis

Setiap kali Anda membuat rilis, GitHub Actions akan menghasilkan **2 kategori paket sekaligus**:

### A. 📦 Installer Desktop untuk Pengguna Baru (First-Time Install)
File installer resmi (*wizard "Next > Next > Install > Finish"*) untuk dibagikan kepada pengguna/komputer kasir yang baru pertama kali memasang aplikasi:

| Sistem Operasi | Ekstensi / Format | Nama File Installer |
| :--- | :--- | :--- |
| **Windows (64-bit)** | `.exe` (Setup) & `.msi` | `Notane-Retail-POS_x64-setup.exe` / `.msi` |
| **Windows (32-bit / x86)** | `.exe` (Setup) & `.msi` | `Notane-Retail-POS_x86-setup.exe` / `.msi` (Untuk POS Jadul / Atom / Celeron) |
| **macOS** | `.dmg` (Disk Image) | `Notane-Retail-POS.dmg` (Universal / Apple Silicon & Intel) |
| **Linux (Debian/Ubuntu)** | `.deb` | `notane-retail-pos_amd64.deb` |
| **Linux (Universal)** | `.AppImage` | `notane-retail-pos_amd64.AppImage` |

---

### B. 🔄 Paket Pembaruan OTA (Over-The-Air) untuk Di-upload ke Notane Manager
Paket pembaruan yang digunakan oleh toko yang sudah berjalan (kasir tinggal klik tombol *"Update"* dari dalam aplikasi):

| Nama File Rilis | Komponen | Platform Target | Deskripsi |
| :--- | :--- | :--- | :--- |
| `notane-client-ui-vX.X.X.zip` | **Client** | **Universal (Semua OS)** | Bundle aset UI Frontend Kasir & Back-Office (HTML, JS, CSS). Berlaku langsung untuk Windows, Linux, & macOS. |
| `notane-server-vX.X.X-windows-x64.zip` | **Server** | **Windows (x64)** | Biner backend `notane-retail-server.exe` 64-bit, Host UI, dan migrasi skema SQL. |
| `notane-server-vX.X.X-windows-x86.zip` | **Server** | **Windows (32-bit)** | Biner backend `notane-retail-server.exe` 32-bit, Host UI, dan migrasi skema SQL. |
| `notane-server-vX.X.X-linux-x64.zip` | **Server** | **Linux (x64)** | Biner backend Linux ELF, Host UI, dan migrasi skema SQL. |
| `notane-server-vX.X.X-macos-arm64.zip` | **Server** | **macOS (M1/M2/M3/M4)** | Biner backend macOS Apple Silicon ARM64, Host UI, dan migrasi skema SQL. |
| `notane-server-vX.X.X-macos-x64.zip` | **Server** | **macOS (Intel x64)** | Biner backend macOS Intel x86_64, Host UI, dan migrasi skema SQL. |
| `CHECKSUMS.txt` | **Checksum** | **Semua** | Daftar hash SHA-256 dari seluruh file yang siap disalin ke Notane Manager. |

---

## 2. Cara Memulai Rilis di GitHub

Terdapat **2 cara** mudah untuk menjalankan rilis otomatis:

### Cara 1: Menggunakan Git Tag (Sangat Direkomendasikan)
Cukup buat tag versi di git lokal Anda dan push ke GitHub:

```bash
# 1. Pastikan seluruh kodingan telah di-commit
git add .
git commit -m "feat: rilis pembaruan versi 1.2.0"

# 2. Buat tag semver (misal: v1.2.0)
git tag v1.2.0

# 3. Push commit & tag ke GitHub
git push origin main
git push origin v1.2.0
```

GitHub Actions akan langsung otomatis mendeteksi tag `v1.2.0` dan mem-build seluruh platform secara paralel!

---

### Cara 2: Menjalankan Manual dari Antarmuka Web GitHub (One-Click)
Jika Anda belum ingin membuat tag git lokal:
1. Buka repositori proyek Anda di GitHub.
2. Klik tab **"Actions"** di bar menu atas.
3. Di panel sebelah kiri, pilih workflow **"Multi-Platform OTA Release Builder"**.
4. Klik tombol **"Run workflow"** di sebelah kanan.
5. Masukkan rincian:
   - **Nomor Versi Rilis:** contoh `1.2.0`
   - **Catatan Perubahan (Changelog):** rangkuman fitur baru / perbaikan bug
   - Centang apakah ingin mem-build Client, Server, atau keduanya.
6. Klik tombol hijau **"Run workflow"**.

---

## 3. Mengambil Hasil Build & Memasukkan ke Notane Manager

Setelah GitHub Actions selesai berjalan (sekitar 3–5 menit):

1. Masuk ke halaman **"Releases"** di repositori GitHub Anda (`https://github.com/username/repository/releases`).
2. Di rilis terbaru tersebut, Anda akan melihat:
   - File-file `.zip` untuk setiap OS yang siap diunduh langsung.
   - Tabel ringkasan **Checksum SHA-256** dari setiap file.
3. **Pendaftaran ke Notane Manager:**
   - Buka aplikasi **Notane Manager** -> menu **Rilis OTA**.
   - Klik **"Rilis Client (UI)"** atau **"Rilis Server"**.
   - Salin URL file `.zip` (bisa menggunakan URL rilis GitHub langsung, atau upload ke hosting Anda).
   - Salin nilai **SHA-256 Checksum** yang tertera pada tabel GitHub Release.
   - Klik **"Terbitkan & Tanda Tangani Ed25519"**.

Selesai! Seluruh terminal kasir dan host server Notane Retail akan langsung mendeteksi ketersediaan pembaruan resmi yang aman dan terverifikasi secara kriptografis.
