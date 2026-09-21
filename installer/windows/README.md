# 📦 Panduan Membuat Installer Distribusi Satu Klik (Opsi A)

Folder ini berisi konfigurasi **Inno Setup** untuk memaketkan **Notane Retail POS Enterprise** beserta **Database PostgreSQL** ke dalam **1 file installer Windows tunggal** (`Notane-Retail-POS-Installer-x64.exe`).

Dengan installer ini, pengguna awam (kasir/pemilik toko) cukup menjalankan 1 file `.exe`:
- Aplikasi akan terinstal di `C:\Program Files\Notane Retail POS`.
- PostgreSQL 16 akan otomatis terinstal dan berjalan sebagai background service di Windows secara *silent* (tanpa memunculkan wizard rumit ke pengguna).
- Database `notane_pos` otomatis dibuat.
- Shortcut aplikasi otomatis muncul di Desktop.

---

## 🛠️ Langkah-Langkah Build Paket Distribusi

### 1. Build Binary Aplikasi (Tauri)
Pastikan Anda sudah mengompilasi binary release dari aplikasi:
```powershell
npm run tauri build
```
Binary release akan tercipta di:
`src-tauri/target/release/notane-retail-pos.exe`

### 2. Siapkan File Installer PostgreSQL Silent
Unduh installer PostgreSQL 16 untuk Windows (`postgresql-16.x-windows-x64.exe`), lalu simpan dan ubah namanya menjadi:
`installer/windows/prerequisites/postgresql-installer.exe`

*(Jika Anda sudah mengunduhnya sebelumnya, Anda cukup menyalin file dari folder cache/temp)*.

### 3. Kompilasi Installer dengan Inno Setup
1. Instal [Inno Setup 6.x](https://jrsoftware.org/isdl.php) di komputer Anda (atau via winget: `winget install JRSoftware.InnoSetup`).
2. Klik kanan file `setup.iss` lalu pilih **"Compile"**, atau gunakan perintah CLI:
   ```powershell
   & "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" setup.iss
   ```
3. File installer akhir akan tercipta di folder `Output/`:
   `Output/Notane-Retail-POS-Installer-x64.exe`

File installer tersebut siap dibagikan kepada pengguna akhir (*end-user*). Pengguna awam cukup melakukan *"Next > Install > Finish"*.
