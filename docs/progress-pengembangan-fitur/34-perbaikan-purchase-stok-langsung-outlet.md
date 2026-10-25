# Progress 34: Perbaikan Alur Pembelian & Penerimaan Barang (Purchase GRN) Masuk Langsung ke Outlet Pemesan

## Latar Belakang & Masalah
Sebelumnya, pada formulir pembuatan Pembelian / Penerimaan Barang Baru (*Purchase GRN*), terdapat input ganda:
- **Outlet Pemesan**
- **Gudang Penerima**

Ketika user memilih Outlet Pemesan (misal Cabang Toko A) namun sistem secara default mengarahkan ke Gudang Utama, stok barang masuk dialokasikan ke Gudang Utama alih-alih masuk ke stok Cabang Toko A yang memesan barang tersebut. Selain membingungkan pengguna, input `Gudang Penerima` menjadi redundan karena pada arsitektur sistem Notane (sejak unifikasi outlet & gudang), setiap outlet sudah bertindak sebagai lokasi stok mandiri.

---

## Perubahan yang Dilakukan

### 1. Form Penerimaan Barang Masuk (`PurchasesPage.vue`)
- **Penghapusan Selector Gudang Penerima**: Input form `Gudang Penerima` (`warehouse_id`) dihilangkan dari antarmuka modal pembuatan Purchase GRN.
- **Outlet Pemesan / Penerima Stok Mandiri**:
  - Diberi label jelas: `Outlet Pemesan / Penerima Stok *`.
  - Default otomatis terisi dengan outlet aktif yang sedang dibuka pengguna (`locationStore.currentOutletId`), atau cabang pertama.
  - Form disederhanakan menjadi 6 input field dalam grid 3 kolom yang proporsional dan bersih:
    1. Pemasok / Supplier
    2. Outlet Pemesan / Penerima Stok
    3. No. Surat Jalan / Faktur Supplier
    4. Tanggal Faktur
    5. Tanggal Jatuh Tempo
    6. Catatan Pengiriman
- **Pembersihan Tampilan Tabel & Detail Modal**:
  - Kolom Outlet pada tabel daftar faktur menampilkan nama outlet pemesan secara bersih tanpa label gudang yang tumpang tindih.
  - Modal detail faktur menampilkan informasi `Outlet Pemesan: [Nama Outlet]`.

### 2. Backend Inventory Service (`inventory_service.rs`)
- **Alokasi Stok Langsung ke Outlet Pemesan**:
  - Variabel target penyimpanan stok (`target_wh_id`) langsung diarahkan ke `outlet_id` (Outlet Pemesan).
  - Tidak lagi melakukan pencarian fallback ke `default_warehouse_id` atau gudang induk.
- **Pencatatan Stok & Mutasi**:
  - Record stok di tabel `warehouse_stocks` diupdate untuk `warehouse_id = outlet_id`.
  - Riwayat `stock_mutations` mencatat mutasi masuk dengan keterangan: `Penerimaan Barang Masuk No [Nomor Faktur] ke [Nama Outlet]`.
- **Query Daftar & Detail Faktur**:
  - Relasi join diselaraskan dengan tabel `outlets`, sehingga nama outlet pemesan selalu tersaji akurat baik untuk data baru maupun arsip lama.

---

## Hasil Pengujian & Verifikasi
1. Dilakukan pengujian pembuatan GRN baru untuk outlet **Notane Cabang Kelapa Gading**:
   - Stok awal produk `Green Tea Latte 250ml Botol`: **97 BTL**
   - Penerimaan GRN: **20 BTL**
   - Stok akhir Cabang Kelapa Gading: **117 BTL** $(\text{Bertambah tepat } +20 \text{ BTL})$
   - Stok Gudang Utama Toko tidak terganggu / tidak bertambah salah sasaran.
2. Pembayaran faktur ditandai lunas (`PAID`) via endpoint `/api/v1/inventory/purchases/:id/pay`, status dan saldo lunas tersinkronisasi sempurna.
