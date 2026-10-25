# Panduan Deployment Docker: Notane Server (Multi-Instance) & Notane Manager

Dokumen ini menjelaskan cara menjalankan **Notane Server** dan **Notane Manager** di server / VPS menggunakan Docker.

---

## 1. Notane Manager di Docker (Licensing & OTA Provider)

Notane Manager dapat dijalankan sebagai service pusat lisensi di VPS dengan web dashboard bawaan.

### A. File Konfigurasi
Gunakan file `docker-compose.manager.yml` yang sudah tersedia di root proyek.

### B. Menjalankan Notane Manager
```bash
# Build dan jalankan di background:
docker compose -f docker-compose.manager.yml up -d --build
```

### C. Port & Data Persisten
* **Port Web Dashboard & API:** `19090` (dapat diubah via `MANAGER_PORT=...`)
* **Akses Browser:** Buka `http://IP-VPS-ANDA:19090` untuk masuk ke Studio Manajemen Lisensi & Rilis OTA.
* **Volume Data:**
  * `manager_data` -> Menyimpan database SQLite lisensi (`/data/notane_provider.db`) secara aman dan persisten.
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
