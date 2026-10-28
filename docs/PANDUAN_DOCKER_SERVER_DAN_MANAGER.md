# Panduan Deployment Docker: Notane Server (Multi-Instance) & Notane Manager

Dokumen ini menjelaskan cara menjalankan **Notane Server** dan **Notane Manager** di server / VPS menggunakan Docker.

---

## 1. Notane Manager di Docker (Licensing & OTA Provider)

Notane Manager dapat dijalankan sebagai service pusat lisensi di VPS dengan web dashboard bawaan.
Mendukung **Dual-Database Engine**:
* **Lokal / Desktop Development:** Menggunakan **SQLite** bawaan tanpa perlu install server database terpisah.
* **VPS / Production Container:** Menggunakan **PostgreSQL** otomatis via container Docker pada port non-default **19432** (mencegah konflik dengan PostgreSQL default `5432` atau Notane Server `15432`).

### A. File Konfigurasi
Gunakan file `docker-compose.manager.yml` dan file environment `.env.manager` (atau template `.env.manager.example`) yang sudah tersedia di root proyek.
Cukup sesuaikan domain dan port di `.env.manager`:
```env
# .env.manager
MANAGER_PORT=19090
PROVIDER_URL=https://license.domainanda.com

# PostgreSQL Host Port (Custom non-default 19432)
MANAGER_POSTGRES_PORT=19432
MANAGER_POSTGRES_USER=notane_manager
MANAGER_POSTGRES_PASSWORD=manager_secret_2026
MANAGER_POSTGRES_DB=notane_manager_db
```

### B. Menjalankan Notane Manager
```bash
# Jalankan di background via NPM shortcut:
npm run manager:up

# Atau perintah Docker Compose standar:
docker compose -f docker-compose.manager.yml --env-file .env.manager up -d --build
```

### C. Port & Data Persisten
* **Port Web Dashboard & API:** `19090` (dapat diubah via `MANAGER_PORT=...`)
* **Port Database PostgreSQL:** `19432` (dapat diubah via `MANAGER_POSTGRES_PORT=...`)
* **Domain / Host Publik (Opsional):** Setel `PROVIDER_URL=https://license.domainanda.com` atau `HOST_IP=license.domainanda.com` di environment agar alamat API di sidebar & dashboard sesuai nama domain publik Anda.
* **Akses Browser:** Buka `http://IP-VPS-ANDA:19090` atau `https://license.domainanda.com` untuk masuk ke Studio Manajemen Lisensi & Rilis OTA.
* **Volume Data:**
  * `manager_pg_data` -> Menyimpan database PostgreSQL secara aman dan persisten.
  * `manager_data` -> Menyimpan cadangan data/SQLite jika mode SQLite diaktifkan.
  * `manager_releases` -> Menyimpan arsip rilis OTA (`/app/releases`).

---

## 2. Notane Server di 1 VPS dengan Multi-Port (Multi-Instance / Multi-Tenant)

Anda dapat menjalankan **banyak toko sekaligus dalam 1 VPS** tanpa bentrok port maupun database.

### Cara Menjalankan Beberapa Toko di 1 VPS:

#### Contoh: Menjalankan Toko 1 (Cabang A)
Buat file environment `.env.toko1`:
```env
COMPOSE_PROJECT_NAME=toko1
SERVER_PORT=18080
CONTROL_PORT=18081
POSTGRES_PORT=15432
POSTGRES_DB=pos_toko1
PAIRING_CODE=NOTANE-CABANG-A
BRAND_NAME=Notane Cabang A
```
Jalankan Toko 1:
```bash
docker compose -p toko1 --env-file .env.toko1 up -d
```

---

#### Contoh: Menjalankan Toko 2 (Cabang B)
Buat file environment `.env.toko2`:
```env
COMPOSE_PROJECT_NAME=toko2
SERVER_PORT=18090
CONTROL_PORT=18091
POSTGRES_PORT=15433
POSTGRES_DB=pos_toko2
PAIRING_CODE=NOTANE-CABANG-B
BRAND_NAME=Notane Cabang B
```
Jalankan Toko 2:
```bash
docker compose -p toko2 --env-file .env.toko2 up -d
```

---

### Mengapa Setup Ini 100% Aman & Terisolasi?
1. **Container Name & Network Terpisah:**
   Flag `-p <project_name>` memastikan setiap toko memiliki jaringan internal Docker terpisah.
2. **Port Host Tidak Bentrok:**
   * Toko 1: POS API `18080`, Control Host `18081`, Postgres `15432`.
   * Toko 2: POS API `18090`, Control Host `18091`, Postgres `15433`.
3. **Database Terisolasi:**
   Masing-masing container PostgreSQL memiliki volume data persisten sendiri (`toko1_postgres_data` dan `toko2_postgres_data`).
4. **Hardware ID Unik:**
   Setiap container memiliki identitas unik sehingga lisensi masing-masing toko tidak saling bercampur.

---

## 3. Perintah Cepat Manajemen Container

| Tindakan | Perintah |
| :--- | :--- |
| **Cek Status Semua Toko** | `docker ps` |
| **Melihat Log Notane Manager** | `docker compose -f docker-compose.manager.yml logs -f` |
| **Melihat Log Toko Tertentu** | `docker compose -p toko1 logs -f server` |
| **Restart Toko Tertentu** | `docker compose -p toko1 restart` |
| **Hentikan Toko Tertentu** | `docker compose -p toko1 down` |
| **Update / Rebuild Image** | `docker compose -p toko1 up -d --build` |

---

## 4. Alur Pembaruan & Pengecekan Versi di Docker (Catatan Penting)

Untuk memastikan container Docker menjalankan versi terbaru yang sesuai dengan rilis Desktop dan Git Tag:

### A. Alur Update Versi di Server VPS (Standar & Otomatis)
Setiap kali ada rilis versi baru (misalnya rilis `v1.4.1`):
1. **Tarik perubahan kode terbaru dari Git:**
   ```bash
   git pull origin main
   # atau jika menggunakan tag rilis tertentu:
   # git fetch --tags && git checkout v1.4.1
   ```
2. **Rebuild container dengan kode terbaru:**
   ```bash
   # Untuk Notane Server (Toko 1):
   docker compose -p toko1 up -d --build

   # Untuk Notane Manager:
   docker compose -f docker-compose.manager.yml --env-file .env.manager up -d --build
   ```
> [!NOTE]
> Container otomatis membaca nomor versi terbaru dari `Cargo.toml` dan `package.json` hasil `git pull`. Anda **tidak perlu mengedit file `.env` secara manual**.

### B. Override Versi Runtime via `.env` (Opsional / Tanpa Rebuild)
Jika Anda ingin mengubah versi yang ditampilkan sistem secara instan tanpa melakukan kompilasi ulang image Docker:
* Cukup tambahkan atau ubah baris berikut di file `.env` toko Anda:
  ```env
  APP_VERSION=1.4.1
  ```
* Kemudian restart service server:
  ```bash
  docker compose -p toko1 restart server
  ```
* Jika variabel `APP_VERSION` dikosongkan atau dihapus dari `.env`, sistem akan otomatis fallback menggunakan nomor versi bawaan kompilasi kode (`Cargo.toml` & `package.json`).

