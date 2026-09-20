# Dokumen Perencanaan & Spesifikasi Proses Bisnis Sistem POS Retail Enterprise

## 1. Pendahuluan
Dokumen ini merupakan panduan komprehensif bagi developer dan programmer dalam memahami arsitektur, aliran proses bisnis, dan spesifikasi teknis dari Sistem Point of Sales (POS) Retail berskala enterprise (setara minimarket modern seperti Indomaret/Alfamart). Sistem ini dirancang untuk menangani beban transaksi tinggi dengan kapabilitas *offline-first* di level gerai (Jaringan LAN) dengan pencatatan akuntansi dan inventory yang sangat ketat.

## 2. Arsitektur Sistem & Topologi Jaringan
Sistem ini menggunakan pendekatan **Single-Binary Dual-Mode** yang dikembangkan menggunakan **Tauri v2 (Rust) + Vue 3**. Distribusi aplikasi dikemas dalam satu file eksekusi tunggal (.exe/.app) yang perannya ditentukan saat instalasi.

*   **Mode Host (Server Lokal / Back-Office):** Bertindak sebagai *server* gerai utama. Komponen ini menjalankan mesin **PostgreSQL (Embedded)** secara otomatis di latar belakang untuk menjamin reliabilitas data dan **REST/WebSocket API (menggunakan Rust Axum)** untuk melayani permintaan dari mesin-mesin klien.
*   **Mode Client (Thin POS / Front-Office):** Berjalan di mesin-mesin kasir (misalnya Kasir 1, Kasir 2). Beroperasi **tanpa** database lokal. Semua proses baca-tulis dikirim langsung ke API Host via jaringan LAN. Status interaktif (seperti isi keranjang belanja sementara) dikelola dalam memori volatil klien menggunakan *Pinia Store*.

**Spesifikasi Teknologi:**
*   **Frontend UI:** Vue 3, Pinia (State Management), Tailwind CSS (dioptimasi untuk antarmuka padat, interaksi cepat, dan dukungan penuh *keyboard shortcut*).
*   **Backend Logic:** Rust (dengan framework Axum dan sqlx).
*   **Database:** PostgreSQL (Mode Embedded pada mesin Host).

### 2.1. Struktur Direktori Proyek (Monorepo Tauri)
Untuk memudahkan *development* dan pemisahan *concerns* antara antarmuka Vue 3 (Frontend) dan logika inti Rust Axum (Backend/Host), proyek ini dianjurkan menggunakan struktur standar Tauri v2:

```text
notane-retail-pos/
├── src/                          # FRONTEND: Kode sumber Vue 3 & Pinia
│   ├── assets/                   # Gambar, font, styling CSS/Tailwind global
│   ├── components/               # Komponen UI Vue (Buttons, Modals, Data Tables)
│   ├── layouts/                  # Layout wrapper (POS Layout, Back-Office Layout)
│   ├── pages/                    # Halaman utama (Login, Cashier POS, Master Data)
│   ├── stores/                   # Pinia State Management (authStore, cartStore)
│   ├── services/                 # Pembungkus panggilan API Axios / Tauri IPC
│   ├── App.vue                   # Root komponen Vue
│   └── main.ts                   # Entry point aplikasi Vue 3
│
├── src-tauri/                    # BACKEND: Logika inti Rust, Axum, & Tauri v2
│   ├── src/
│   │   ├── api/                  # Rute-rute REST API & WebSocket Axum (Controllers)
│   │   ├── db/                   # Konfigurasi koneksi sqlx dan PG Embedded
│   │   ├── models/               # Struct data Rust (Representasi tabel Database)
│   │   ├── services/             # Logika bisnis inti (POS Checkout, Jurnal Double-Entry)
│   │   ├── tauri_cmds/           # Fungsi IPC (Inter-Process) spesifik sistem operasi
│   │   ├── utils/                # Fungsi utilitas (Hashing Argon2id, Formatter)
│   │   └── main.rs               # Entry point Rust (Bootstrapping Tauri & Axum Server)
│   ├── migrations/               # Berkas SQL DDL untuk migrasi skema database (sqlx)
│   ├── tauri.conf.json           # Konfigurasi build Tauri, ID aplikasi, window settings
│   └── Cargo.toml                # Manajemen dependensi Rust (axum, sqlx, tokio)
│
├── package.json                  # Manajemen dependensi Frontend (NPM/Yarn/PNPM)
├── vite.config.ts                # Konfigurasi bundler Vite
└── tailwind.config.js            # Konfigurasi tema *styling* Tailwind CSS
```

---

## 3. Detail Proses Bisnis (Business Process Flow)

Sistem ini menganut prinsip ERP terintegrasi, di mana aliran barang dari Gudang hingga dibeli oleh Konsumen tercatat dengan akurat dan langsung terhubung dengan laporan keuangan.

### A. Proses Manajemen Master Data
1.  **Struktur Hierarki Barang:** Pengelompokan barang berdasarkan Departemen -> Kategori -> Sub-Kategori.
2.  **Manajemen SKU & Multi-Barcode:** 
    *   Satu entitas produk memiliki satu SKU (Stock Keeping Unit).
    *   Mendukung pencatatan **Multi-Barcode** untuk satu SKU (sering terjadi jika pabrikan mengubah desain kemasan namun isi sama).
    *   Mendukung **Weight-Embedded Barcode** (Barcode dinamis hasil cetakan mesin timbangan digital otomatis untuk sayur/buah/daging yang memuat kombinasi kode barang, harga, dan berat).
3.  **Manajemen Harga & Multi-Satuan:**
    *   Mendukung konversi satuan bertingkat (Contoh: 1 Dus = 24 Pcs). Penjualan dalam Pcs akan otomatis mendemosi stok Dus, begitu juga sebaliknya.
    *   Mendukung skema harga grosir bertingkat / *Tier Pricing* berdasarkan kuantitas beli.
    *   **Harga Khusus Pelanggan (Customer Tier Pricing):** Mendukung pembedaan harga produk berdasarkan Grup Pelanggan (Misal: Harga Member, Harga Grosir, Harga Reguler).
    *   **Metode Penilaian Persediaan (Costing Method):** Mendukung fleksibilitas pengaturan pengeluaran stok untuk HPP (COGS). Tiap produk dapat dikonfigurasi menggunakan metode **FIFO** (First-In First-Out), **LIFO** (Last-In First-Out), atau **Weighted Moving Average** (Rata-Rata Tertimbang).

### B. Proses Pengadaan & Penerimaan Barang (Procurement to Pay)
1.  **Purchase Order (PO):** Pembuatan pesanan ke Supplier yang dapat dipicu secara manual atau otomatis berdasarkan indikator batas stok minimum (`min_stock`).
2.  **Good Receipt Note (GRN) / Penerimaan Faktur Barang:**
    *   Barang fisik tiba dan dicocokkan dengan PO.
    *   Sistem melakukan penerimaan barang masuk ke gudang. Proses ini **menambah stok riil** (`stock_mutations`) dan **memperbarui harga HPP (Rata-rata)** jika terjadi fluktuasi harga beli dari supplier.
    *   Faktur penerimaan ini otomatis memicu hutang usaha ke supplier jika berstatus kredit.
3.  **Retur Pembelian (Return to Vendor - RTV):** Pengembalian barang cacat atau kedaluwarsa ke pihak distributor. Transaksi ini otomatis mengurangi stok gudang dan menyesuaikan saldo hutang ke supplier.

### C. Proses Operasional Kasir (Order to Cash / Front-Office)
1.  **Buka Shift (Open Shift):**
    *   Kasir login dengan kredensial masing-masing, memilih terminal kasir yang sedang digunakan.
    *   Kasir menginput modal awal kasir (*Floating Cash / Uang Kembalian Dasar*). Sistem akan mencatat `opened_at` dan ID sesi shift tersebut ke server.
2.  **Transaksi Penjualan (Checkout):**
    *   Kasir memindai barcode menggunakan scanner, klien akan mengambil profil harga dari API Host dengan sangat cepat.
    *   **Fitur Hold / Recall (Tahan/Panggil Transaksi):** Sangat krusial. Jika konsumen tertinggal dompet atau perlu mengambil barang lain, kasir dapat "menahan" (Hold) transaksi tersebut ke memori, lalu melayani antrean konsumen berikutnya (Recall transaksi baru), tanpa membatalkan barang konsumen pertama.
    *   **Otorisasi Supervisor Langsung (In-Place PIN):** Tindakan sensitif seperti pembatalan barang yang terlanjur dipindai (*Void Item*), pembatalan seluruh nota (*Cancel Bill*), pemberian diskon manual, atau pengubahan harga hanya dapat dilakukan dengan verifikasi PIN Supervisor. Modal/Popup otorisasi akan muncul seketika di layar kasir, tanpa memaksa kasir logout.
3.  **Penggunaan Voucher & Poin (Promosi & Loyalitas):**
    *   **Voucher Diskon:** Kasir dapat memindai atau menginput kode voucher (potongan nominal/persentase). Sistem memvalidasi limit maksimum & batas minimum belanja.
    *   **Buku Besar Poin (Point Ledger):** Mendukung *reward points* pelanggan. Transaksi perolehan atau penukaran poin dicatat ke dalam *ledger* khusus untuk menjamin akurasi mutasi poin jika terjadi transaksi simultan.
4.  **Proses Pembayaran (Payment):**
    *   Mendukung **Split Payment** (pembayaran campuran, misalnya sebagian Tunai, sisanya via QRIS atau Poin Pelanggan).
    *   Sistem langsung menghitung kembalian uang tunai.
5.  **Integrasi Real-Time Pembukuan:** Setelah struk tercetak, transaksi **otomatis membentuk jurnal akuntansi lengkap** (Penjualan, HPP, Persediaan, Pajak, Kas).
6.  **Tutup Shift (Blind Settlement):**
    *   Di akhir hari kerja, kasir melakukan serah terima. Kasir diwajibkan menghitung uang fisik yang ada di laci dan menginput nominal total tersebut ke sistem **(Blind Count)**.
    *   Kasir tidak diperlihatkan berapa nominal "seharusnya" menurut sistem.
    *   Sistem Host kemudian mengkalkulasi komparasi antara hitungan sistem vs hitungan fisik kasir. Jika ada beda, akan dicatat sebagai *Over* (Lebih Kas) atau *Short* (Kurang Kas) yang kemudian dapat ditindaklanjuti sebagai beban potong gaji kasir atau keuntungan tak terduga.
    *   Mesin akan mencetak Laporan Ringkasan Shift (X/Z Report).

### D. Proses Manajemen Persediaan (Inventory Control)
1.  **Stock Opname (Penyesuaian Fisik Berkala):**
    *   Proses pendataan barang aktual di rak (misal dilakukan setiap akhir bulan).
    *   Sistem membandingkan nilai Stok Sistem vs Stok Aktual.
    *   Nilai selisih (*Discrepancy*) akan tercatat dalam jurnal sebagai Beban/Keuntungan Selisih Persediaan pada laporan keuangan.
2.  **Kartu Stok Ketat (Stock Ledger):** Seluruh aktivitas pergerakan stok mutlak didasarkan pada dokumen referensi. Tidak diperbolehkan mengedit tabel stok secara manual. (Dokumen Ref: Penjualan, GRN Pembelian, Retur, Opname, Transfer Antar Gudang).

### E. Integrasi Akuntansi (General Ledger)
1.  **Otomatisasi Double-Entry:** Poin terkuat sistem ini. Pengguna operasional (Kasir, Gudang) tidak dituntut untuk paham ilmu akuntansi debet-kredit. Sistem yang akan menerjemahkan setiap operasi mereka ke dalam entri jurnal.
2.  **Contoh Penjurnalan POS secara latar belakang:**
    *   *(Debet)* Kas Tunai / Piutang QRIS Bank
    *   *(Debet)* Beban Pokok Penjualan (HPP / COGS)
    *   *(Kredit)* Persediaan Barang Dagang
    *   *(Kredit)* Pendapatan Penjualan Retail
    *   *(Kredit)* Hutang Pajak PPN (Jika ada)
3.  Dengan ini, Eksekutif atau Manajer Toko bisa menarik Laporan Laba/Rugi (*Income Statement*) kapan saja dengan data seakurat detik itu juga.

---

## 4. Standar Keamanan & Jejak Audit (Security & Auditing)
1.  **Role-Based Access Control (RBAC):** Hak akses dikendalikan berdasarkan matriks peran pengguna. Contoh: Supervisor punya hak `pos:void`, kasir biasa tidak. Staf Gudang memiliki hak `inventory:write`, namun tidak punya akses ke UI transaksi POS.
2.  **Jejak Audit 360 Derajat (Comprehensive Audit Trail):**
    *   Setiap baris data di hampir seluruh tabel mencatat secara persis waktu modifikasi dan UUID pelaku (`created_at`, `updated_at`, `created_by`, `updated_by`).
    *   Aktivitas krusial direkam ke dalam tabel terpisah `audit_logs` yang menyimpan payload data lama vs data baru. Aksi krusial mencakup: Pembukaan Cash Drawer tanpa transaksi, Otorisasi PIN, Pembatalan Nota, Penghapusan Retur.
3.  **Keamanan Sandi & PIN:** Kata sandi (*password*) pengguna dan PIN otrisasi Supervisor disimpan dalam bentuk *hash* satu arah berstandar industri modern menggunakan algoritma **Argon2id**.

---

## 5. Standar Desain Antarmuka (UI/UX) Frontend
1.  **Produktivitas Maksimal Kasir:** Desain UI layar kasir bukan untuk tampilan "cantik" konsumer, melainkan untuk **efisiensi tingkat milidetik**. Tampilan cenderung padat (tabular data). Interaksi *mouse* dikurangi hingga batas minimum.
2.  **Keyboard-Driven Interface:** Semua fungsi utama harus dapat dipicu melalui *hotkeys/shortcuts*. (Contoh: `Enter` untuk fokus input barcode, `F1` Cari Barang, `F4` Buka Laci, `F12` Bayar, `Esc` Batal).
3.  **Layar Hadap Pelanggan (Customer Display):** Sistem harus mendukung proyeksikan rincian belanja real-time (Nama Barang, Diskon, Subtotal) ke monitor sekunder atau perangkat *pole display* yang menghadap konsumen, menjamin transparansi transaksi.

---

## 6. Struktur Database (Skema Lengkap PostgreSQL)
Berikut adalah rincian lengkap Data Definition Language (DDL) untuk database sistem POS ini. Struktur ini mengadopsi standar *double-entry accounting* dan *audit trail* komprehensif, mendukung seluruh alur bisnis dari pengadaan hingga penjualan.

```sql
-- EKSTENSI UNTUK GENERASI UUID
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- FUNGSI OTOMATISASI UPDATED_AT
CREATE OR REPLACE FUNCTION update_timestamp_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- MODUL 1: MANAJEMEN PENGGUNA, PERAN, & AUDIT KEAMANAN
-- ============================================================================

CREATE TABLE roles (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) UNIQUE NOT NULL,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID,
    updated_by UUID
);

CREATE TABLE permissions (
    id SERIAL PRIMARY KEY,
    code VARCHAR(50) UNIQUE NOT NULL, -- e.g., pos:void, report:view
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE role_permissions (
    role_id INT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id INT NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (role_id, permission_id)
);

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    role_id INT NOT NULL REFERENCES roles(id),
    username VARCHAR(50) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    pin_hash VARCHAR(255) NOT NULL, -- Untuk approval popup supervisor
    full_name VARCHAR(100) NOT NULL,
    email VARCHAR(100),
    phone VARCHAR(30),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- Foreign key self-reference untuk tabel roles
ALTER TABLE roles ADD CONSTRAINT fk_roles_created_by FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE roles ADD CONSTRAINT fk_roles_updated_by FOREIGN KEY (updated_by) REFERENCES users(id);

CREATE TABLE audit_logs (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID REFERENCES users(id),
    authorized_by UUID REFERENCES users(id), -- Supervisor yang approve aksi sensitif
    terminal_id VARCHAR(50) NOT NULL,
    action VARCHAR(50) NOT NULL,             -- LOGIN, VOID, PRICE_OVERRIDE, CASH_DRAWER_OPEN
    entity_name VARCHAR(50) NOT NULL,        -- sales, inventory, cash_shift
    entity_id VARCHAR(100),
    old_values JSONB,
    new_values JSONB,
    ip_address VARCHAR(45),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- ============================================================================
-- MODUL 2: MASTER DATA REFERENSI, OUTLET, PRODUK, & HARGA
-- ============================================================================

CREATE TABLE brands (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(30) UNIQUE NOT NULL,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE payment_methods (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(30) UNIQUE NOT NULL,
    name VARCHAR(100) NOT NULL,
    category VARCHAR(50) NOT NULL, -- CASH, EDC, QRIS, TRANSFER, VOUCHER
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE reason_codes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    type VARCHAR(30) NOT NULL, -- VOID, ADJUSTMENT, RETURN
    code VARCHAR(30) UNIQUE NOT NULL,
    description TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE outlets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(20) UNIQUE NOT NULL,
    name VARCHAR(100) NOT NULL,
    address TEXT,
    phone VARCHAR(30),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_id UUID REFERENCES categories(id),
    code VARCHAR(30) UNIQUE NOT NULL,
    name VARCHAR(100) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE units (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(20) UNIQUE NOT NULL, -- PCS, BOX, KG, LTR, DUS
    name VARCHAR(50) NOT NULL,
    is_fractional BOOLEAN NOT NULL DEFAULT FALSE, -- Boleh desimal (timbangan sayur/buah)
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE products (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category_id UUID NOT NULL REFERENCES categories(id),
    base_unit_id UUID NOT NULL REFERENCES units(id),
    sku VARCHAR(50) UNIQUE NOT NULL,
    barcode VARCHAR(64) UNIQUE NOT NULL,
    name VARCHAR(200) NOT NULL,
    brand_id UUID REFERENCES brands(id),
    costing_method VARCHAR(20) NOT NULL DEFAULT 'WMA', -- FIFO, LIFO, WMA
    cost_price NUMERIC(18, 4) NOT NULL DEFAULT 0.0000, -- HPP saat ini (Current Cost)
    base_selling_price NUMERIC(18, 4) NOT NULL,
    min_stock NUMERIC(14, 4) NOT NULL DEFAULT 0.0000,
    max_stock NUMERIC(14, 4) NOT NULL DEFAULT 0.0000,
    tax_percent NUMERIC(5, 2) NOT NULL DEFAULT 11.00, -- PPN
    is_weight_embedded BOOLEAN NOT NULL DEFAULT FALSE, -- Barcode dari timbangan elektrik
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- Multi-Satuan Konversi (contoh: 1 BOX = 24 PCS)
CREATE TABLE product_units (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    unit_id UUID NOT NULL REFERENCES units(id),
    conversion_factor NUMERIC(14, 4) NOT NULL, -- Misal 24 terhadap base unit
    selling_price NUMERIC(18, 4) NOT NULL,
    barcode VARCHAR(64) UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id),
    CONSTRAINT uk_product_unit UNIQUE (product_id, unit_id)
);

-- Multi-Barcode untuk satu produk (pabrikan ganti kemasan/UPC alternatif)
CREATE TABLE product_barcodes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    barcode VARCHAR(64) UNIQUE NOT NULL,
    description VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- Skema Harga Bertingkat / Grosir (Tier Pricing)
CREATE TABLE product_price_tiers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    min_quantity NUMERIC(14, 4) NOT NULL,
    tier_price NUMERIC(18, 4) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- ============================================================================
-- MODUL 3: MANAJEMEN GUDANG & INVENTORI
-- ============================================================================

CREATE TABLE warehouses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    outlet_id UUID NOT NULL REFERENCES outlets(id),
    code VARCHAR(20) UNIQUE NOT NULL,
    name VARCHAR(100) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE warehouse_stocks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    warehouse_id UUID NOT NULL REFERENCES warehouses(id),
    product_id UUID NOT NULL REFERENCES products(id),
    quantity NUMERIC(14, 4) NOT NULL DEFAULT 0.0000,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id),
    CONSTRAINT uk_warehouse_product UNIQUE (warehouse_id, product_id)
);

-- Tabel Batch Stok untuk pencatatan FIFO / LIFO
CREATE TABLE stock_batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    warehouse_id UUID NOT NULL REFERENCES warehouses(id),
    product_id UUID NOT NULL REFERENCES products(id),
    batch_number VARCHAR(50) NOT NULL,
    received_date DATE NOT NULL,
    expiry_date DATE,
    original_qty NUMERIC(14, 4) NOT NULL,
    remaining_qty NUMERIC(14, 4) NOT NULL,
    unit_cost NUMERIC(18, 4) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE', -- ACTIVE, DEPLETED, EXPIRED
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- Kartu Stok Real-Time (Semua mutasi barang wajib tercatat di sini)
CREATE TABLE stock_mutations (
    id BIGSERIAL PRIMARY KEY,
    warehouse_id UUID NOT NULL REFERENCES warehouses(id),
    product_id UUID NOT NULL REFERENCES products(id),
    reference_type VARCHAR(50) NOT NULL, -- SALES, PURCHASE_GRN, RETURN_IN, RETURN_OUT, OPNAME, TRANSFER
    reference_id UUID NOT NULL,
    qty_in NUMERIC(14, 4) NOT NULL DEFAULT 0.0000,
    qty_out NUMERIC(14, 4) NOT NULL DEFAULT 0.0000,
    balance NUMERIC(14, 4) NOT NULL,
    cost_price NUMERIC(18, 4) NOT NULL,  -- Nilai HPP per unit saat mutasi
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- Stock Opname / Penyesuaian Fisik Toko
CREATE TABLE stock_adjustments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    adjustment_number VARCHAR(50) UNIQUE NOT NULL,
    warehouse_id UUID NOT NULL REFERENCES warehouses(id),
    adjustment_date DATE NOT NULL,
    reason_id UUID NOT NULL REFERENCES reason_codes(id),
    status VARCHAR(20) NOT NULL DEFAULT 'DRAFT', -- DRAFT, APPROVED, CANCELLED
    approved_by UUID REFERENCES users(id),
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE stock_adjustment_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    adjustment_id UUID NOT NULL REFERENCES stock_adjustments(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id),
    system_qty NUMERIC(14, 4) NOT NULL,
    actual_qty NUMERIC(14, 4) NOT NULL,
    discrepancy_qty NUMERIC(14, 4) NOT NULL, -- actual - system
    cost_price NUMERIC(18, 4) NOT NULL,
    total_cost_discrepancy NUMERIC(18, 4) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- ============================================================================
-- MODUL 4: PENGADAAN & HUTANG USAHA (SUPPLIER & PURCHASING)
-- ============================================================================

CREATE TABLE suppliers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(30) UNIQUE NOT NULL,
    name VARCHAR(150) NOT NULL,
    contact_person VARCHAR(100),
    phone VARCHAR(30),
    address TEXT,
    term_of_payment_days INT NOT NULL DEFAULT 0, -- 0 = Cash On Delivery
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE purchase_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    po_number VARCHAR(50) UNIQUE NOT NULL,
    supplier_id UUID NOT NULL REFERENCES suppliers(id),
    warehouse_id UUID NOT NULL REFERENCES warehouses(id),
    order_date DATE NOT NULL,
    expected_delivery DATE,
    total_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    status VARCHAR(30) NOT NULL DEFAULT 'PENDING', -- PENDING, PARTIAL, COMPLETED, CANCELLED
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE purchase_order_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    po_id UUID NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id),
    ordered_qty NUMERIC(14, 4) NOT NULL,
    unit_price NUMERIC(18, 4) NOT NULL,
    subtotal NUMERIC(18, 4) NOT NULL,
    received_qty NUMERIC(14, 4) NOT NULL DEFAULT 0.0000,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- Good Receipt Note (GRN) / Faktur Pembelian Masuk
CREATE TABLE purchase_invoices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_number VARCHAR(50) UNIQUE NOT NULL,
    supplier_invoice_number VARCHAR(100) NOT NULL,
    po_id UUID REFERENCES purchase_orders(id),
    supplier_id UUID NOT NULL REFERENCES suppliers(id),
    warehouse_id UUID NOT NULL REFERENCES warehouses(id),
    invoice_date DATE NOT NULL,
    due_date DATE NOT NULL,
    subtotal NUMERIC(18, 4) NOT NULL,
    tax_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    discount_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    total_amount NUMERIC(18, 4) NOT NULL,
    paid_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    payment_status VARCHAR(20) NOT NULL DEFAULT 'UNPAID', -- UNPAID, PARTIAL, PAID
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE purchase_invoice_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_id UUID NOT NULL REFERENCES purchase_invoices(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id),
    received_qty NUMERIC(14, 4) NOT NULL,
    unit_price NUMERIC(18, 4) NOT NULL,
    discount_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    subtotal NUMERIC(18, 4) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- ============================================================================
-- MODUL 5: TRANSAKSI KASIR (POS), SHIFT, PELANGGAN, & PROMOSI
-- ============================================================================

CREATE TABLE customer_groups (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) UNIQUE NOT NULL, -- REGULAR, MEMBER, WHOLESALE
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE customer_group_prices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    customer_group_id INT NOT NULL REFERENCES customer_groups(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    special_price NUMERIC(18, 4) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id),
    CONSTRAINT uk_group_product_price UNIQUE (customer_group_id, product_id)
);

CREATE TABLE customers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    customer_group_id INT REFERENCES customer_groups(id),
    code VARCHAR(30) UNIQUE NOT NULL,
    name VARCHAR(100) NOT NULL,
    phone VARCHAR(30) UNIQUE,
    email VARCHAR(100),
    credit_limit NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    reward_points INT NOT NULL DEFAULT 0, -- Cached balance
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE customer_point_ledgers (
    id BIGSERIAL PRIMARY KEY,
    customer_id UUID NOT NULL REFERENCES customers(id),
    transaction_type VARCHAR(30) NOT NULL, -- EARNED, REDEEMED, EXPIRED, CORRECTION
    reference_id UUID, -- Bisa mengarah ke sales(id)
    points_in INT NOT NULL DEFAULT 0,
    points_out INT NOT NULL DEFAULT 0,
    balance INT NOT NULL,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id)
);

CREATE TABLE vouchers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(50) UNIQUE NOT NULL,
    description TEXT,
    discount_type VARCHAR(20) NOT NULL, -- PERCENTAGE, FIXED_AMOUNT
    discount_value NUMERIC(18, 4) NOT NULL,
    min_purchase_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    max_discount_amount NUMERIC(18, 4), -- Batas maksimal untuk PERCENTAGE
    valid_from TIMESTAMPTZ NOT NULL,
    valid_until TIMESTAMPTZ NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE', -- ACTIVE, USED, EXPIRED
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE cashier_shifts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    terminal_id VARCHAR(50) NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    opened_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    closed_at TIMESTAMPTZ,
    opening_cash NUMERIC(18, 4) NOT NULL,
    expected_cash NUMERIC(18, 4), -- Dihitung otomatis oleh sistem
    actual_cash NUMERIC(18, 4),   -- Blind count oleh kasir
    discrepancy NUMERIC(18, 4),   -- actual - expected
    status VARCHAR(20) NOT NULL DEFAULT 'OPEN', -- OPEN, CLOSED
    supervisor_approval_id UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE sales (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_number VARCHAR(50) UNIQUE NOT NULL,
    shift_id UUID NOT NULL REFERENCES cashier_shifts(id),
    customer_id UUID REFERENCES customers(id),
    warehouse_id UUID NOT NULL REFERENCES warehouses(id),
    voucher_id UUID REFERENCES vouchers(id), -- Voucher yang digunakan
    subtotal NUMERIC(18, 4) NOT NULL,
    discount_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    tax_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    total_amount NUMERIC(18, 4) NOT NULL,
    paid_amount NUMERIC(18, 4) NOT NULL,
    change_amount NUMERIC(18, 4) NOT NULL,
    total_cost_amount NUMERIC(18, 4) NOT NULL, -- Total HPP untuk COGS real-time
    status VARCHAR(20) NOT NULL DEFAULT 'COMPLETED', -- COMPLETED, VOID, RETURNED
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE sale_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sale_id UUID NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id),
    unit_id UUID NOT NULL REFERENCES units(id),
    quantity NUMERIC(14, 4) NOT NULL,
    unit_cost_price NUMERIC(18, 4) NOT NULL, -- Nilai HPP satuan saat penjualan
    unit_selling_price NUMERIC(18, 4) NOT NULL,
    discount_percent NUMERIC(5, 2) NOT NULL DEFAULT 0.00,
    discount_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    tax_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    subtotal NUMERIC(18, 4) NOT NULL,
    is_voided BOOLEAN NOT NULL DEFAULT FALSE,
    void_reason_id UUID REFERENCES reason_codes(id),
    void_authorized_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE sale_payments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sale_id UUID NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
    payment_method_id UUID NOT NULL REFERENCES payment_methods(id),
    amount NUMERIC(18, 4) NOT NULL,
    card_number_masked VARCHAR(20),
    reference_number VARCHAR(100),
    payment_proof_url TEXT, -- URL / Path ke file bukti transfer/pembayaran
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- ============================================================================
-- MODUL 6: PEMBUKUAN AKUNTANSI DOUBLE-ENTRY (GENERAL LEDGER)
-- ============================================================================

CREATE TABLE account_categories (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) UNIQUE NOT NULL, -- ASSET, LIABILITY, EQUITY, REVENUE, EXPENSE
    normal_balance VARCHAR(10) NOT NULL, -- DEBIT, CREDIT
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE chart_of_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_category_id INT NOT NULL REFERENCES account_categories(id),
    parent_id UUID REFERENCES chart_of_accounts(id),
    code VARCHAR(30) UNIQUE NOT NULL, -- Misal: 1101 (Kas Toko), 5101 (HPP)
    name VARCHAR(150) NOT NULL,
    description TEXT,
    is_header BOOLEAN NOT NULL DEFAULT FALSE, -- Akun induk atau operasional
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- Jurnal Header (Double Entry)
CREATE TABLE journal_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    journal_number VARCHAR(50) UNIQUE NOT NULL,
    journal_date DATE NOT NULL,
    reference_type VARCHAR(50) NOT NULL, -- SALES, PURCHASE, INVENTORY_DISCREPANCY, PAYMENT, EXPENSE
    reference_id UUID,
    description TEXT NOT NULL,
    total_debit NUMERIC(18, 4) NOT NULL,
    total_credit NUMERIC(18, 4) NOT NULL,
    is_balanced BOOLEAN GENERATED ALWAYS AS (total_debit = total_credit) STORED,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id),
    CONSTRAINT chk_journal_balanced CHECK (total_debit = total_credit)
);

-- Jurnal Detail (Debit / Kredit Baris)
CREATE TABLE journal_lines (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    journal_id UUID NOT NULL REFERENCES journal_entries(id) ON DELETE CASCADE,
    account_id UUID NOT NULL REFERENCES chart_of_accounts(id),
    debit NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    credit NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    memo TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id),
    CONSTRAINT chk_debit_credit CHECK (
        (debit > 0 AND credit = 0) OR 
        (debit = 0 AND credit > 0)
    )
);

-- ============================================================================
-- ATTACH TIMESTAMP TRIGGERS PADA SEMUA TABEL
-- ============================================================================

DO $$
DECLARE
    t text;
BEGIN
    FOR t IN 
        SELECT table_name 
        FROM information_schema.tables 
        WHERE table_schema = 'public' 
          AND table_type = 'BASE TABLE'
    LOOP
        EXECUTE format('
            CREATE TRIGGER trg_update_timestamp_%I
            BEFORE UPDATE ON %I
            FOR EACH ROW
            EXECUTE FUNCTION update_timestamp_column();', t, t);
    END LOOP;
END;
$$;
```

---

## 6. Spesifikasi Endpoint API (Rust Axum & WebSockets)

Untuk menjembatani antarmuka klien Vue 3 di mesin kasir dengan database PostgreSQL di Host, *backend* (Rust Axum) harus mengekspos endpoint REST dan WebSocket berikut. Semua *request* wajib menyertakan `Authorization: Bearer <JWT>` kecuali `POST /login`.

### A. Modul Otentikasi & Keamanan (Auth)
| Method | Endpoint | Kegunaan | Payload / Keterangan |
| :--- | :--- | :--- | :--- |
| `POST` | `/api/v1/auth/login` | Login kasir/admin | `{ username, password, terminal_id }` -> Kembalikan Token JWT. |
| `POST` | `/api/v1/auth/supervisor-approve` | Otorisasi aksi sensitif (*In-Place PIN*) | `{ pin }`. Mengembalikan *short-lived token* atau status validasi yang harus dilampirkan ke *request* selanjutnya. |
| `GET` | `/api/v1/audit-logs` | Tarik histori aksi kritikal | Penarikan data dengan *pagination* & *filtering* (Tanggal, Aksi). |

### B. Modul Transaksi & POS (Front-Office)
| Method | Endpoint | Kegunaan | Payload / Keterangan |
| :--- | :--- | :--- | :--- |
| `POST` | `/api/v1/shifts/open` | Buka shift kasir harian | `{ terminal_id, opening_cash }`. Mencatat ID Kasir. |
| `POST` | `/api/v1/shifts/close` | Tutup shift (*Blind Settlement*) | `{ actual_cash }`. Sistem akan mengkalkulasi selisih (*discrepancy*). |
| `GET` | `/api/v1/shifts` | Riwayat Shift Kasir | Mendapatkan daftar shift (Filter: Tanggal, Kasir, Status). |
| `POST` | `/api/v1/sales/checkout` | Proses pembayaran belanja | **[VITAL]** `{ shift_id, customer_id, voucher_id, items: [...], payments: [...] }`. API harus membungkus proses ini dalam satu **DB Transaction**: (1) Buat `sales`, (2) Kurangi stok di `warehouse_stocks`, (3) Catat `stock_mutations`, (4) Hitung Poin Loyalitas, (5) Bentuk `journal_entries` ganda. |
| `GET` | `/api/v1/sales` | Daftar Riwayat Transaksi POS | Mendapatkan daftar struk/penjualan dengan *pagination*. |
| `GET` | `/api/v1/sales/:id` | Detail Struk/Nota | Menampilkan `sale_items` dan `sale_payments`. |
| `POST` | `/api/v1/sales/:id/void` | Pembatalan nota yang sudah bayar | Membutuhkan otorisasi supervisor. Akan membentuk mutasi stok kembali (*In*) dan jurnal *Reversal*. |
| `POST` | `/api/v1/sales/hold` | Menahan struk ke DB sementara | Untuk fitur antrean. Status struk disetel ke `ON_HOLD`. |

### C. Modul Master Data (Back-Office)
| Method | Endpoint | Kegunaan | Payload / Keterangan |
| :--- | :--- | :--- | :--- |
| `GET` | `/api/v1/products/barcode/:code` | Pencarian produk via *Scanner* | Sangat butuh latensi rendah (<50ms). Jika *weight-embedded*, langsung hitung harga dari berat *barcode*. |
| `GET/POST/PUT` | `/api/v1/products` | Master Data Produk | Endpoint CRUD (`GET` mengembalikan daftar *paginated*, `POST` tambah, `PUT/:id` perbarui). |
| `GET/POST` | `/api/v1/brands` | Master Data Merek | Menampilkan dan mengatur tabel `brands`. |
| `GET/POST` | `/api/v1/categories` | Master Kategori Barang | Menampilkan hierarki tabel `categories`. |
| `GET/POST` | `/api/v1/units` | Master Satuan | Menampilkan konfigurasi UoM / tabel `units`. |
| `GET/POST/PUT` | `/api/v1/customers` | Master Pelanggan | `GET` mengembalikan daftar pelanggan. Tersedia sub-route `GET /api/v1/customers/:id/points` untuk histori. |
| `GET/POST` | `/api/v1/customer-groups` | Master Grup Pelanggan | Menampilkan *tier* grup pelanggan di `customer_groups`. |
| `GET/POST/PUT` | `/api/v1/vouchers` | Master Voucher Promo | `GET` menampilkan daftar *voucher* di dasbor *marketing*, `POST` membuat kode promosi baru. |
| `GET` | `/api/v1/vouchers/validate/:code` | Validasi Penggunaan Voucher | Dipakai oleh Kasir POS untuk validasi *minimum belanja* & *expiry*. |
| `GET/POST` | `/api/v1/payment-methods` | Master Metode Pembayaran | Mengambil opsi bayar aktif (`CASH`, `QRIS`, `EDC`). |
| `GET/POST` | `/api/v1/reason-codes` | Master Kode Alasan | Mengambil opsi alasan *Void* dan *Stock Adjustment*. |
| `GET/POST` | `/api/v1/users` | Manajemen Karyawan | Daftar *users*, *roles*, dan *permissions*. |

### D. Modul Inventori & Pengadaan
| Method | Endpoint | Kegunaan | Payload / Keterangan |
| :--- | :--- | :--- | :--- |
| `GET/POST` | `/api/v1/purchase-orders` | Manajemen *Purchase Order* | Pembuatan dan daftar PO ke Supplier. |
| `GET` | `/api/v1/purchase-invoices` | Riwayat Faktur Pembelian (GRN) | Menampilkan daftar barang masuk beserta tagihan supplier. |
| `POST` | `/api/v1/inventory/grn` | Terima faktur barang masuk | Memicu mutasi penambahan stok (*Qty In*), membentuk `stock_batches` untuk alur FIFO, dan kalkulasi HPP Rata-rata. |
| `GET/POST` | `/api/v1/inventory/adjustments` | Manajemen Koreksi/Opname Stok | `GET` daftar opname, `POST` dengan `{ reason_id, actual_qty }` untuk mencatat perbedaan ke jurnal *shrinkage/loss*. |
| `GET` | `/api/v1/inventory/stock` | Cek sisa stok aktual | Menerima kueri `?warehouse_id=X&product_id=Y`. |

### E. Modul Pelaporan & Akuntansi (Reporting)
| Method | Endpoint | Kegunaan | Payload / Keterangan |
| :--- | :--- | :--- | :--- |
| `GET` | `/api/v1/reports/sales-summary` | Laporan Penjualan Harian/Bulanan | Data agregasi penjualan kotor, diskon, PPN, dan penjualan bersih. |
| `GET` | `/api/v1/reports/shift-z-report` | Laporan *Z-Report* per Shift | Ringkasan penerimaan kas, QRIS, dan EDC berdasarkan ID Shift. |
| `GET` | `/api/v1/reports/stock-ledger` | Laporan Kartu Stok | *Tracing* historis mutasi barang (Saldo Awal, Masuk, Keluar, Saldo Akhir). |
| `GET` | `/api/v1/reports/income-statement` | Laporan Laba Rugi | Ditarik langsung dari `journal_entries` dan `chart_of_accounts`. |

### F. Integrasi Real-Time (WebSockets)
| Endpoint | Arah | Kegunaan | Format Pesan (*Message*) |
| :--- | :--- | :--- | :--- |
| `WS /ws/v1/events` | 2-Arah | Menjamin konsistensi data antar kasir secara instan. | `{"event": "STOCK_UPDATED", "product_id": "...", "new_qty": 5}`<br>`{"event": "PRICE_CHANGED", "product_id": "..."}` |

> [!IMPORTANT]  
> **Isolasi Beban Jaringan**: Semua beban kalkulasi akuntansi dan jurnal berganda (Double-Entry) dieksekusi di *Host* (API Server). Aplikasi Klien Vue 3 di mesin kasir hanya bertugas mengirim payload JSON yang tervalidasi dan menerima konfirmasi *Success* untuk mengurangi lag pergesekan.
