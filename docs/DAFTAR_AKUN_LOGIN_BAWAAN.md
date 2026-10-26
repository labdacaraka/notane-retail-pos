# 🔐 Daftar Akun Pengguna Bawaan (Default Credentials)

Dokumen ini berisi informasi akun pengguna bawaan (*default seed users*) untuk **Notane Retail POS**. 

> ⚠️ **PENTING UNTUK KEAMANAN:**
> - Demi menjaga keamanan data operasional toko, **tombol login cepat (shortcut) pada layar login telah dihilangkan**.
> - Setelah aplikasi pertama kali dipasang dan digunakan di toko yang sebenarnya, **segera ubah kata sandi dan PIN** seluruh akun bawaan ini melalui menu **Pengguna & Hak Akses** atau melalui ikon profil di pojok kanan atas.

---

## 1. Tabel Akun Bawaan Notane Retail (Server Host & POS Client)

| Nama Staf | Nama Pengguna (*Username*) | Kata Sandi (*Password*) | PIN Otorisasi (6 Digit) | Peran (*Role*) | Hak Akses Utama |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Administrator Utama** | `admin` | `admin123` | `123456` | **Administrator (Superadmin)** | Seluruh menu: master data, pengaturan sistem, kelola pengguna, akuntansi, dan rilis. |
| **Supervisor Toko** | `spv` | `spv123` | `123456` | **Supervisor** | Otorisasi void/pembatalan nota, diskon khusus, opname stok, dan laporan toko. |
| **Siti Kasir Utama** | `kasir1` | `kasir123` | `111111` | **Kasir (Cashier)** | Buka/tutup shift laci kasir, transaksi penjualan POS, cetak struk, dan setor kas laci. |

---

## 2. Tabel Akun Bawaan Notane Manager (License & Provider Studio)

Aplikasi **Notane Manager** digunakan untuk mengelola lisensi toko/brand, rilis update komponen, dan aktivasi lisensi.

| Nama Akun | Nama Pengguna (*Username*) | Kata Sandi (*Password*) | Quick PIN (6 Digit) | Peran (*Role*) | Hak Akses Utama |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Provider Superadmin** | `admin` | `admin` | `123456` | **Superadmin** | Kelola tenant & brand toko, generate token aktivasi offline/online, penandatanganan rilis update (*cryptographic signing*), monitoring HWID & audit lisensi. |

---

## 3. Cara Mengubah Kata Sandi & PIN Mandiri

### A. Pada Notane Server & POS Client
1. Masuk (*login*) menggunakan akun Anda.
2. Pada baris menu atas (di sebelah kanan nama Anda), klik nama atau avatar profil Anda.
3. Jendela **"Keamanan Akun & Ganti PIN"** akan muncul.
4. Masukkan kata sandi lama, kemudian buat kata sandi baru atau 6 digit PIN baru yang mudah Anda ingat.
5. Klik **"Simpan Perubahan"**.

### B. Pada Notane Manager
1. Masuk (*login*) ke dashboard Notane Manager.
2. Buka menu **Pengaturan** di sidebar navigasi.
3. Pada tab **Akun & Keamanan**, ubah kata sandi dan PIN cepat (*Quick PIN*) sesuai kebutuhan Anda.
4. Klik **"Simpan Perubahan Akun"**.
