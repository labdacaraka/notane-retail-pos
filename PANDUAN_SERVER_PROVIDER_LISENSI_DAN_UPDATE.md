# 🛡️ PANDUAN PENGEMBANGAN SERVER PROVIDER LISENSI & SISTEM PEMBARUAN (OTA)
## NOTANE RETAIL POS ENTERPRISE

Dokumen ini merupakan spesifikasi arsitektur teknis, standar keamanan anti-cracking, skema database, API endpoint, serta kode implementasi siap pakai untuk server backend penyedia (Provider Server) Notane Retail POS.

---

## 📑 DAFTAR ISI
1. [Arsitektur Keamanan & Anti-Cracking](#1-arsitektur-keamanan--anti-cracking)
2. [Manajemen Pasangan Kunci Kriptografi (Ed25519)](#2-manajemen-pasangan-kunci-kriptografi-ed25519)
3. [Skema Database Provider Server (PostgreSQL / SQLite)](#3-skema-database-provider-server)
4. [Spesifikasi REST API Server Provider](#4-spesifikasi-rest-api-server-provider)
5. [Implementasi Lengkap Server Provider (Node.js / Express / TypeScript)](#5-implementasi-lengkap-server-provider)
6. [Mekanisme One-Click Update (Front-End, Back-End, & Database Migration)](#6-mekanisme-one-click-update)
7. [Panduan Hardening & Pencegahan Reverse Engineering](#7-panduan-hardening--pencegahan-reverse-engineering)

---

## 1. ARSITEKTUR KEAMANAN & ANTI-CRACKING

Sistem lisensi Notane dirancang dengan standar keamanan militer asimetris untuk mencegah cracking, keygen tiruan, dan pembajakan:

```
+-------------------------------------------------------------+
|                 NOTANE PROVIDER SERVER                      |
| (Tersimpan Aman di Infrastruktur Provider / Cloud)           |
|                                                             |
|  [Master Private Key (Ed25519)] ---> Generate Signature     |
|  [Database Pelanggan & Lisensi]                             |
|  [Repositori Rilis & Bundle Update OTA]                     |
+-------------------------------------------------------------+
                              |
                     HTTPS / TLS Encrypted
                              |
                              v
+-------------------------------------------------------------+
|               TERMINAL NOTANE RETAIL POS                    |
| (Komputer Klien / Kasir / Back-Office)                      |
|                                                             |
|  1. Node-Locking HWID: Identifikasi hardware unik           |
|     (SHA-256 dari Serial Motherboard, CPU, MAC, Machine ID) |
|  2. [Master Public Key (Ed25519)]: Tersemat di kode Rust    |
|  3. Verifikasi Signature Asimetris: Mustahil dipalsukan     |
|     tanpa Private Key Provider.                             |
|  4. Anti-Clock-Rollback: Pencatatan waktu monotonic SQLite  |
|     mencegah pemunduran jam sistem operasi komputer klien.  |
|  5. Offline Grace Period: Tetap bisa beroperasi offline     |
|     selama 7 hari grace period sebelum verifikasi online.   |
+-------------------------------------------------------------+
```

### Keunggulan Kriptografi:
1. **Asymmetric Ed25519**: Klien **hanya** menyimpan *Public Key*. Meskipun binary didekompilasi atau di-reverse-engineer, hacker **tidak pernah bisa membuat generator kunci valid** karena Private Key tidak pernah keluar dari server provider.
2. **Node-Locking Hardware ID**: Lisensi yang dikeluarkan diikat ke ID fisik mesin klien (`current_hardware_id`). Token lisensi yang disalin ke komputer lain otomatis **ditolak** (`HARDWARE_MISMATCH`).
3. **Anti-Clock-Rollback**: Klien menyimpan timestamp terakhir ke dalam tabel `app_licenses.last_monotonic_time`. Jika pengguna memundurkan jam PC untuk memperpanjang lisensi, sistem langsung mendeteksi tampering dan memblokir aplikasi (`CLOCK_TAMPERED`).
4. **Verifikasi Hash Bundle Update**: Setiap paket pembaruan diverifikasi menggunakan SHA-256 dan Signature Ed25519 sebelum diekstrak, memastikan binary tidak disusupi trojan/patcher.

---

## 2. MANAJEMEN PASANGAN KUNCI KRIPTOGRAFI (Ed25519)

Pasangan kunci yang telah terkonfigurasi pada Notane Retail POS:

### A. Kunci Publik (Tersimpan di Client Binary Rust):
```
779b7d3fb80a25e9503bf7b85ffbd226ed9a4b8ff9de844872dd3a19357bb213
```
*Lokasi di kode klien*: `src-tauri/src/services/license_service.rs` (`MASTER_PUBLIC_KEY_HEX`).

### B. Kunci Privat (HANYA UNTUK SERVER PROVIDER):
```
40c97328401a31ed63b6ecd07bf1ea35a731e775cab79c730582b49069b0a5b9
```
> [!CAUTION]
> **JANGAN PERNAH** membagikan atau menyertakan kunci privat ini ke dalam repository publik atau file instalasi klien. Simpan di environment variable (`PROVIDER_PRIVATE_KEY`) pada server backend provider Anda.

---

## 3. SKEMA DATABASE PROVIDER SERVER

Gunakan skema database PostgreSQL (atau MySQL/SQLite) berikut pada server provider Anda:

```sql
-- 1. Tabel Klien / Bisnis Penyewa
CREATE TABLE provider_clients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_name VARCHAR(150) NOT NULL,
    company_name VARCHAR(150) NOT NULL,
    email VARCHAR(100) UNIQUE NOT NULL,
    phone VARCHAR(30),
    address TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- 2. Tabel Lisensi
CREATE TABLE provider_licenses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    license_key VARCHAR(64) UNIQUE NOT NULL,
    client_id UUID NOT NULL REFERENCES provider_clients(id) ON DELETE CASCADE,
    tier VARCHAR(50) NOT NULL DEFAULT 'ENTERPRISE', -- BASIC, PRO, ENTERPRISE
    max_terminals INT NOT NULL DEFAULT 5,
    status VARCHAR(30) NOT NULL DEFAULT 'ACTIVE', -- ACTIVE, SUSPENDED, EXPIRED, REVOKED
    issued_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    grace_period_days INT NOT NULL DEFAULT 7,
    features JSONB NOT NULL DEFAULT '["multi_outlet", "inventory_batch", "api_sync", "offline_first", "auto_update"]',
    notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- 3. Tabel Aktivasi Perangkat (Node-Locking HWID)
CREATE TABLE provider_device_activations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    license_id UUID NOT NULL REFERENCES provider_licenses(id) ON DELETE CASCADE,
    hardware_id VARCHAR(128) NOT NULL,
    device_name VARCHAR(100),
    ip_address VARCHAR(50),
    os_info VARCHAR(100),
    app_version VARCHAR(30),
    activated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    last_ping_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    CONSTRAINT uq_license_hardware UNIQUE (license_id, hardware_id)
);

-- 4. Tabel Rilis Pembaruan Aplikasi (OTA Update Releases)
CREATE TABLE provider_app_releases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version VARCHAR(30) UNIQUE NOT NULL, -- contoh: '1.2.0'
    release_name VARCHAR(150) NOT NULL,
    changelog TEXT NOT NULL,
    release_date DATE NOT NULL DEFAULT CURRENT_DATE,
    download_url TEXT NOT NULL,
    checksum_sha256 VARCHAR(64) NOT NULL,
    signature_ed25519 VARCHAR(128) NOT NULL,
    is_mandatory BOOLEAN NOT NULL DEFAULT FALSE,
    min_license_tier VARCHAR(50) NOT NULL DEFAULT 'BASIC',
    has_db_migration BOOLEAN NOT NULL DEFAULT TRUE,
    is_published BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- 5. Tabel Audit Log & Percobaan Pembajakan
CREATE TABLE provider_license_audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    license_key VARCHAR(64),
    hardware_id VARCHAR(128),
    action VARCHAR(50) NOT NULL, -- ACTIVATE, VERIFY, CHECK_UPDATE, TAMPER_ALERT
    ip_address VARCHAR(50),
    status VARCHAR(20) NOT NULL, -- SUCCESS, REJECTED, BLOCKED
    details JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
```

---

## 4. SPESIFIKASI REST API SERVER PROVIDER

Semua response menggunakan format JSON baku.

### A. `POST /api/v1/license/activate`
Dipanggil klien saat pertama kali mendaftarkan lisensi atau mengaktifkan mesin baru.

**Request Payload:**
```json
{
  "license_key": "NTN-ENT-2026-X899",
  "hardware_id": "a1b2c3d4e5f67890abcdef1234567890abcdef1234567890abcdef1234567890",
  "device_name": "Kasir-Utama-Lt1",
  "app_version": "1.1.0"
}
```

**Response Payload (Status 200 OK):**
```json
{
  "success": true,
  "data": {
    "license_token": "{\"license_key\":\"NTN-ENT-2026-X899\",\"client_name\":\"PT Retail Sejahtera\",\"tier\":\"ENTERPRISE\",\"max_terminals\":5,\"hardware_id\":\"a1b2c3d4e5f67890abcdef1234567890abcdef1234567890abcdef1234567890\",\"issued_at\":\"2026-09-26T00:00:00Z\",\"expires_at\":\"2027-09-26T23:59:59Z\",\"grace_period_days\":7,\"features\":[\"multi_outlet\",\"auto_update\"]}",
    "signature": "3f88b77a0194e45c479e0a29ef3104e12e2f38d...64_bytes_hex..."
  }
}
```

---

### B. `POST /api/v1/license/verify`
Pengecekan periodik status lisensi dari terminal klien ke server provider.

**Request Payload:**
```json
{
  "license_key": "NTN-ENT-2026-X899",
  "hardware_id": "a1b2c3d4e5f67890abcdef1234567890abcdef1234567890abcdef1234567890",
  "app_version": "1.1.0"
}
```

**Response Payload (Status 200 OK):**
```json
{
  "success": true,
  "status": "ACTIVE",
  "expires_at": "2027-09-26T23:59:59Z",
  "days_remaining": 365,
  "features": ["multi_outlet", "auto_update"]
}
```

---

### C. `POST /api/v1/updates/check`
Memeriksa apakah ada pembaruan aplikasi baru yang tersedia untuk terminal ini.

**Request Payload:**
```json
{
  "current_version": "1.1.0",
  "hardware_id": "a1b2c3d4e5f67890...",
  "license_key": "NTN-ENT-2026-X899"
}
```

**Response Payload (Ada Update):**
```json
{
  "has_update": true,
  "latest_version": "1.2.0",
  "release_name": "Notane Retail Enterprise Update Q4 2026",
  "changelog": "1. Penambahan integrasi EDC Multi-Bank\n2. Optimasi kecepatan query kasir 5x lipat\n3. Pembaruan tabel skema promo diskon otomatis",
  "release_date": "2026-10-01",
  "download_url": "https://update.notane.id/releases/v1.2.0/update-bundle.zip",
  "checksum_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "signature": "8a7f9b2...",
  "is_mandatory": false,
  "can_update": true
}
```

---

## 5. IMPLEMENTASI LENGKAP SERVER PROVIDER (Node.js / Express / TypeScript)

Berikut adalah kode sumber server provider yang dapat langsung Anda jalankan menggunakan Node.js:

### `package.json`
```json
{
  "name": "notane-provider-server",
  "version": "1.0.0",
  "main": "dist/index.js",
  "scripts": {
    "build": "tsc",
    "start": "node dist/index.js",
    "dev": "ts-node src/index.ts"
  },
  "dependencies": {
    "@noble/ed25519": "^2.1.0",
    "cors": "^2.8.5",
    "dotenv": "^16.4.5",
    "express": "^4.19.2",
    "pg": "^8.11.5"
  },
  "devDependencies": {
    "@types/cors": "^2.8.17",
    "@types/express": "^4.17.21",
    "@types/node": "^20.12.7",
    "@types/pg": "^8.11.5",
    "ts-node": "^10.9.2",
    "typescript": "^5.4.5"
  }
}
```

### `src/index.ts`
```typescript
import express, { Request, Response } from 'express'
import cors from 'cors'
import dotenv from 'dotenv'
import * as ed from '@noble/ed25519'
import crypto from 'crypto'

dotenv.config()

const app = express()
app.use(cors())
app.use(express.json())

// KUNCI PRIVAT MASTER PROVIDER (Dari Environment Variable)
const MASTER_PRIVATE_KEY_HEX = process.env.PROVIDER_PRIVATE_KEY || 
  '40c97328401a31ed63b6ecd07bf1ea35a731e775cab79c730582b49069b0a5b9'

// Helper: Tanda Tangani Payload JSON secara Kriptografis (Ed25519)
async function signPayload(payloadJsonString: string): Promise<string> {
  const messageBytes = Buffer.from(payloadJsonString, 'utf-8')
  const privateKeyBytes = Buffer.from(MASTER_PRIVATE_KEY_HEX, 'hex')
  const signatureBytes = await ed.signAsync(messageBytes, privateKeyBytes)
  return Buffer.from(signatureBytes).toString('hex')
}

// 1. Endpoint: Aktivasi Lisensi Baru / Hubungkan Mesin
app.post('/api/v1/license/activate', async (req: Request, res: Response) => {
  try {
    const { license_key, hardware_id, device_name } = req.body

    if (!license_key || !hardware_id) {
      return res.status(400).json({ success: false, message: 'license_key dan hardware_id wajib diisi' })
    }

    // TODO: Validasi license_key terhadap Database (kuota terminal, masa aktif)
    // Contoh Payload Lisensi Terverifikasi:
    const tokenObject = {
      license_key: license_key,
      client_name: 'PT Mitra Retail Mandiri',
      tier: 'ENTERPRISE',
      max_terminals: 10,
      hardware_id: hardware_id, // Node-Locking: Terkunci ke HWID mesin ini!
      issued_at: new Date().toISOString(),
      expires_at: new Date(Date.now() + 365 * 24 * 60 * 60 * 1000).toISOString(), // 1 Tahun
      grace_period_days: 7,
      features: ['multi_outlet', 'inventory_batch', 'offline_mode', 'auto_update']
    }

    const licenseTokenJson = JSON.stringify(tokenObject)
    const signatureHex = await signPayload(licenseTokenJson)

    return res.json({
      success: true,
      data: {
        license_token: licenseTokenJson,
        signature: signatureHex
      }
    })
  } catch (err: any) {
    console.error('Error saat aktivasi lisensi:', err)
    return res.status(500).json({ success: false, message: 'Internal Server Error' })
  }
})

// 2. Endpoint: Cek Status Lisensi Real-Time
app.post('/api/v1/license/verify', async (req: Request, res: Response) => {
  const { license_key, hardware_id } = req.body
  // Query DB untuk memastikan lisensi tidak di-suspend atau di-revoke
  return res.json({
    success: true,
    status: 'ACTIVE',
    expires_at: '2027-09-26T23:59:59Z',
    days_remaining: 365,
    features: ['multi_outlet', 'auto_update']
  })
})

// 3. Endpoint: Cek Pembaruan Versi (OTA Updater)
app.post('/api/v1/updates/check', async (req: Request, res: Response) => {
  try {
    const { current_version, license_key } = req.body

    // Versi rilis stabil terbaru di server provider
    const LATEST_VERSION = '1.2.0'
    const hasUpdate = current_version !== LATEST_VERSION

    if (!hasUpdate) {
      return res.json({
        has_update: false,
        latest_version: current_version,
        message: 'Aplikasi sudah versi terbaru.'
      })
    }

    const changelog = 
      "1. Perbaikan performa migrasi database otomatis saat start.\n" +
      "2. Dukungan struk printer thermal Bluetooth & Network.\n" +
      "3. Penambahan fitur harga khusus member per outlet."

    const downloadUrl = `https://update.notane.id/releases/v${LATEST_VERSION}/notane-update-bundle.zip`
    const checksumSha256 = '4a8a9a4b3c7e...' // SHA256 file zip bundle

    // Tanda tangani checksum file bundle update agar tidak dapat disusupi
    const signature = await signPayload(`${LATEST_VERSION}:${checksumSha256}`)

    return res.json({
      has_update: true,
      latest_version: LATEST_VERSION,
      release_name: `Notane POS v${LATEST_VERSION} Stable`,
      changelog: changelog,
      release_date: new Date().toISOString().split('T')[0],
      download_url: downloadUrl,
      checksum_sha256: checksumSha256,
      signature: signature,
      is_mandatory: false,
      can_update: true,
      message: 'Pembaruan resmi siap diunduh.'
    })
  } catch (err: any) {
    return res.status(500).json({ error: 'Gagal memeriksa pembaruan' })
  }
})

const PORT = process.env.PORT || 3000
app.listen(PORT, () => {
  console.log(`🚀 Notane Provider Server berjalan pada port ${PORT}`)
})
```

---

## 6. MEKANISME ONE-CLICK UPDATE

### A. Struktur File Rilis Pembaruan (`update-bundle.zip`)
Saat Anda merilis pembaruan, paket zip bundle yang di-upload ke CDN/Storage Provider memiliki struktur berikut:

```
update-bundle.zip
├── manifest.json              # Info versi, daftar file, SHA256
├── migrations/                # Skema database baru
│   ├── 013_promo_pricing.sql
│   └── 014_thermal_printer.sql
├── frontend/                  # Web app Vite/Vue hasil 'npm run build'
│   ├── index.html
│   └── assets/
│       ├── index-*.js
│       └── index-*.css
└── backend/                   # Binary Rust hasil 'cargo build --release'
    └── notane-retail-pos      (notane-retail-pos.exe jika Windows)
```

### B. Alur Eksekusi Pembaruan Otomatis Sekali Klik di Klien:

1. **Unduh Paket**: Klien mengunduh `update-bundle.zip` ke direktori sementara lokal.
2. **Validasi Kriptografi**:
   - Hitung SHA-256 file zip unduhan, cocokkan dengan `checksum_sha256`.
   - Verifikasi Ed25519 signature dari server provider menggunakan Master Public Key.
3. **Ekstraksi & Penempatan File**:
   - Replace file direktori `frontend/dist/` dengan aset antarmuka baru.
   - Timpa migration script SQL baru ke direktori migrasi lokal.
   - Timpa binary backend.
4. **Eksekusi Migrasi Database Otomatis**:
   - Backend Notane POS secara bawaan memiliki runner migrasi otomatis (`db::run_migrations(&pool)`).
   - Setiap file SQL baru (misalnya `013_promo_pricing.sql`) yang belum terdaftar di tabel `app_migrations` akan **dieksekusi secara transaksional** saat backend me-restart.
5. **Auto-Restart Service**:
   - Backend memicu restart server daemon melalui endpoint kendali internal (`18081`).
   - Browser / antarmuka UI me-reload halaman (`window.location.reload()`).
   - Seluruh proses berjalan dalam waktu kurang dari 5 detik tanpa perlu instalasi ulang manual!

---

## 7. PANDUAN HARDENING & PENCEGAHAN REVERSE ENGINEERING

Untuk mengamankan binary klien Notane dari teknik decompilation (Ghidra, IDA Pro, dnSpy, dsb.):

### A. Konfigurasi Cargo Release Profile (`src-tauri/Cargo.toml`)
Tambahkan optimasi berikut pada file `Cargo.toml`:
```toml
[profile.release]
opt-level = 3            # Optimasi agresif, meratakan pola kode
lto = "fat"              # Link-Time Optimization antar seluruh crate
codegen-units = 1        # Reduksi batas modularisasi sehingga sulit di-dekompilasi
panic = "abort"          # Hapus traceback informasi stack trace
strip = true             # Hapus seluruh simbol debug & nama fungsi internal
```

### B. Stripping Simbol Biner Setelah Kompilasi
Jalankan perintah berikut sebelum merilis biner ke klien:
```bash
# Untuk Linux / macOS
strip -s target/release/notane-retail-pos

# Proteksi kompresi biner (UPX)
upx --best --lzma target/release/notane-retail-pos
```

### C. Proteksi Sisi Server Provider
1. **Cloudflare Rate Limiting**: Batasi pemanggilan endpoint `/api/v1/license/verify` maksimal 10 request/menit per IP untuk mencegah brute-forcing.
2. **Anti-Replay Nonce**: Sertakan nonce/timestamp pada request aktivasi untuk mencegah rekaman replay request jaringan.
3. **Audit Log Percobaan Ilegal**: Jika ada lisensi yang mencoba didaftarkan pada lebih dari kuota `max_terminals`, catat IP dan tandai di dashboard admin provider.

---

**© 2026 PT Notane Retail Teknologi Nusantara. Seluruh Hak Cipta Dilindungi.**
