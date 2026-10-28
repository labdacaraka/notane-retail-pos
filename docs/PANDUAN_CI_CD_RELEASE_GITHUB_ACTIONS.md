# Panduan Rilis Otomatis Multi-Platform (GitHub Actions OTA Builder)

Dokumen ini menjelaskan cara menggunakan **GitHub Actions** untuk mem-build dan menghasilkan bundle rilis **Notane Client** dan **Notane Server** secara otomatis untuk seluruh sistem operasi (**Windows, Linux, macOS Intel, & macOS Apple Silicon**) tanpa harus berganti-ganti komputer.

---

## 1. Arsitektur File yang Dihasilkan Otomatis

Setiap kali Anda membuat rilis, GitHub Actions akan menghasilkan **3 kategori paket sekaligus**:

### A. 🖥️ Installer Desktop Notane Server (Komputer Induk / Host Toko)
File installer resmi (*wizard GUI Desktop Host Manager*) untuk dipasang pada komputer server / pusat data toko:

| Sistem Operasi | Ekstensi / Format | Nama File Installer Server | Keterangan |
| :--- | :--- | :--- | :--- |
| **Windows (64-bit)** | `.exe` (Setup) & `.msi` | `Notane-Retail-Server_x64-setup.exe` / `.msi` | Komputer server Windows modern 64-bit |
| **Windows (32-bit / x86)** | `.exe` (Setup) & `.msi` | `Notane-Retail-Server_x86-setup.exe` / `.msi` | Komputer kasir/server POS Windows 32-bit |
| **macOS** | `.dmg` (Disk Image) | `Notane-Retail-Server.dmg` | macOS Apple Silicon (M1-M4) & Intel x64 |
| **Linux (Debian/Ubuntu)** | `.deb` | `notane-retail-server_amd64.deb` | Ubuntu / Debian / Linux Mint |
| **Linux (Universal)** | `.AppImage` | `notane-retail-server_amd64.AppImage` | Portable binary tanpa perlu instalasi rumit |

---

### B. 🛒 Installer Desktop Notane Client (Komputer Terminal Kasir POS)
File installer resmi untuk dibagikan kepada pengguna / komputer kasir terminal baru:

| Sistem Operasi | Ekstensi / Format | Nama File Installer Kasir | Keterangan |
| :--- | :--- | :--- | :--- |
| **Windows (64-bit)** | `.exe` (Setup) & `.msi` | `Notane-Retail-POS_x64-setup.exe` / `.msi` | Terminal POS Windows 64-bit |
| **Windows (32-bit / x86)** | `.exe` (Setup) & `.msi` | `Notane-Retail-POS_x86-setup.exe` / `.msi` | Terminal POS hemat daya / Atom / Celeron 32-bit |
| **macOS** | `.dmg` (Disk Image) | `Notane-Retail-POS.dmg` | Terminal POS macOS Apple Silicon & Intel |
| **Linux (Debian/Ubuntu)** | `.deb` | `notane-retail-pos_amd64.deb` | Terminal POS Ubuntu / Debian |
| **Linux (Universal)** | `.AppImage` | `notane-retail-pos_amd64.AppImage` | Terminal POS portable AppImage |

---

### C. 🔄 Paket Pembaruan OTA (Over-The-Air) untuk Di-upload ke Notane Manager
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
Cukup sinkronkan versi, buat tag semver di git lokal Anda, lalu push ke GitHub:

```bash
# 1. Sinkronkan nomor versi ke seluruh manifes proyek (package.json, tauri.conf.json, Cargo.toml)
node scripts/sync-version.js 1.4.1

# 2. Commit perubahan versi dan buat tag semver (misal: v1.4.1)
git add .
git commit -m "chore(release): bump version to 1.4.1"
git tag v1.4.1

# 3. Push commit & tag ke GitHub
git push origin main --tags
# atau:
# git push origin main && git push origin v1.4.1
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

---

## 4. Keamanan Lisensi Terkunci (Anti-Cloning & Zero-Trust)

Untuk mencegah pengguna/pihak luar memanipulasi aplikasi atau membuat situs lisensi tiruan (*cloning / fake license server*):

1. **URL Provider Terkunci di Level Biner (Immutable Compile-Time Baked):**
   - Di mode rilis produksi, Notane Server **mengabaikan seluruh file `.env` dan variabel lingkungan sistem operasi**.
   - URL server lisensi resmi dibakar langsung ke dalam biner berekstensi `.exe`, `.dmg`, `.deb`, dan `.AppImage` saat proses kompilasi di GitHub Actions. Pengguna tidak bisa mengubahnya di level toko.

2. **Cara Mengganti URL Provider Resmi Melalui GitHub Actions:**
   Jika sewaktu-waktu Anda memindahkan Notane Manager ke domain sendiri:
   - **Opsi A (Otomatis untuk semua rilis tag):**
     Masuk ke repositori GitHub Anda: **Settings** > **Secrets and variables** > **Actions** > **Variables** (atau **Secrets**).
     Tambahkan nama: `NOTANE_PROVIDER_URL` dengan nilai: `https://lisensi-anda.domain.com`.
   - **Opsi B (Manual via UI GitHub Actions):**
     Saat klik **"Run workflow"** di tab Actions, masukkan URL server lisensi baru pada kolom input **"URL Server Lisensi Notane Provider"**.
   - GitHub Actions akan otomatis membakar URL tersebut secara permanen ke dalam file installer rilis baru. Pengguna toko tetap tidak dapat mengutak-atiknya!

---

## 5. Proteksi Anti-Reverse Engineering & Kunci Enkripsi (Enterprise Hardening)

Untuk melindungi hak kekayaan intelektual (IP), logika bisnis, dan mekanisme validasi lisensi dari decompilation modern berbasis AI (Ghidra, IDA Pro, Deobfuscator), proses build menerapkan **Defense-in-Depth (4 Lapis Proteksi)**:

### A. Lapis Proteksi yang Aktif Otomatis Saat Build
1. **Frontend JavaScript / Vue Obfuscator (AST Obfuscation):**
   * Mengacak variabel menjadi string heksadesimal tidak terbaca.
   * Meratakan alur kontrol (*control flow flattening*) sehingga struktur percabangan menjadi rumit.
   * Meng-encode string teks menjadi Base64 / array dinamis.
   * Mengaktifkan *Self-Defending* (mencegah attacker memformat ulang / *pretty-print* kode via DevTools).
   * Menonaktifkan output console (*suppress console.log*) untuk mencegah *sniffing* data sensitif.
2. **Rust Binary Hardening & Stripping:**
   * `strip = true`: Seluruh simbol debug, metadata file, nomor baris, dan nama fungsi dihilangkan 100%.
   * `lto = true`: Link-Time Optimization melebur seluruh fungsi lintas crate menjadi satu kesatuan mesin biner yang rumit.
   * `panic = "abort"`: Membuang *unwind stack* yang membocorkan path direktori komputer pengembang.
3. **Compile-Time String Masking (XOR Obfuscation):**
   * Kunci publik lisensi (*Master Public Key*) dienkripsi di level byte compile-time dan hanya didekripsi di RAM saat validasi tanda tangan berlangsung. Perintah `strings` pada biner tidak dapat menemukan teks kunci publik.

### B. Konfigurasi Kunci Rahasia Enkripsi (`NOTANE_BUILD_SECRET`)
* **Penggunaan Default (Otomatis):**
  Jika Anda tidak menyetel apa pun, sistem menggunakan kunci enkripsi bawaan aman `NotanePOS-Default-Internal-Key-2026`.
* **Mengatur Kunci Kustom di GitHub Repository (Direkomendasikan):**
  1. Masuk ke GitHub Repository Anda -> **Settings** > **Secrets and variables** > **Actions**.
  2. Klik **New repository secret** (atau **Repository variable**).
  3. Masukkan nama: `NOTANE_BUILD_SECRET`.
  4. Masukkan string rahasia kuat pilihan Anda (misal: kombinasi 32-64 karakter acak).
  5. Setiap rilis via tag Git otomatis menggunakan kunci rahasia ini.
* **Mengubah Kunci Kustom Secara Manual (Saat Run Workflow):**
  Saat menjalankan rilis secara manual dari menu **Run workflow**, Anda dapat mengisi kolom input **"Kunci Rahasia Enkripsi & Hardening"** dengan kunci yang Anda inginkan.

