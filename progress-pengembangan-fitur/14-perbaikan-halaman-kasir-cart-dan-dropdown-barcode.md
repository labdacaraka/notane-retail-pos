# Progress Log 14: Penyesuaian Halaman Kasir POS (Cart Item Removal Tanpa PIN, Dropdown Barcode Autocomplete, & Akumulasi Quantity)

**Tanggal**: 20 September 2026  
**Status**: Selesai (Completed & Verified)  
**Fitur**:
1. **Hapus Item Cart Tanpa PIN Supervisor**: Penghapusan item barang dari keranjang belanja aktif (draft) dapat dilakukan langsung oleh kasir tanpa meminta otorisasi PIN supervisor. Otorisasi PIN supervisor dipertahankan khusus untuk pembatalan transaksi utuh / nota tersimpan (Void Transaksi) atau buka laci kasir manual.
2. **Dropdown Barang Interaktif pada Input Barcode (`BarcodeBar.vue`)**: Kolom input barcode kini dilengkapi dengan dropdown popover autocomplete yang memanggil API server `GET /products`, mendukung pencarian instan nama/SKU/barcode, navigasi panah keyboard (`ArrowDown`, `ArrowUp`, `Enter`, `Escape`), lencana stok & kategori, serta format rupiah.
3. **Akumulasi Kuantitas Otomatis pada Cart (`cartStore.ts`)**: Ketika kasir menambahkan produk yang sudah ada di dalam keranjang belanja (baik via scan barcode, pemilihan dari dropdown, maupun modal pencarian produk `F2`), kuantitas produk tersebut otomatis bertambah (`quantity += n`) pada baris yang sama, alih-alih membuat baris baru terduplikasi, dan tier harga dihitung ulang secara dinamis.

---

## 1. Latar Belakang & Permintaan Pengguna

Pengguna meminta penyesuaian fungsionalitas operasional kasir retail pada 3 aspek penting:
> *"Ketika aku hapus barang di cart yang udah aku tambahkan harusnya tidak perlu pin supervisor kecuali transaksi yang sudah dibuat, dibagian pindai barcode harusnya muncul dropdown barang, ketika tambah barang yang sudah ada di cart harusnya ditambahkan quantitynya"*

### Analisis Kebutuhan Bisnis Retail:
1. **Kecepatan Layanan Kasir (Checkout Throughput)**:
   - Jika kasir salah memindai barang atau pelanggan membatalkan 1 item belanjaan sebelum pembayaran selesai, meminta PIN supervisor di setiap baris akan memperlambat antrean toko secara signifikan.
   - PIN supervisor hanya wajib diminta untuk tindakan audit kritis, seperti pembatalan nota tersimpan (*Void Sale*) atau pembukaan laci kas (*No-Sale Cash Drawer Kick*).
2. **Pencarian Cepat & Pengenalan Produk (Fast-Key / Typeahead)**:
   - Kasir sering kali perlu mencari produk tanpa barcode fisik (misal produk curah atau label barcode rusak) langsung dari kolom input barcode utama tanpa harus menekan tombol modal `F2`.
   - Adanya dropdown responsif tepat di bawah kolom input memudahkan kasir memilih produk secara visual dengan informasi stok dan harga terkini.
3. **Integritas Keranjang Belanja**:
   - Pemindaian produk yang sama berulang kali harus meningkatkan kuantitas (`qty + 1`) pada baris item tersebut, menjaga struk tetap ringkas dan rapi serta mengaktifkan harga grosir bertingkat (*wholesale tier price*) jika syarat kuantitas terpenuhi.

---

## 2. Rincian Perubahan Implementasi

### A. Penghapusan Supervisor PIN pada Hapus Item Draft Cart
- **File Dimodifikasi**: `src/components/pos/CartTable.vue` & `src/pages/PosRegister.vue`
- **Perubahan**:
  - Tombol ikon hapus (trash) pada setiap baris item keranjang kini langsung mengeksekusi `cartStore.removeItem(index)`.
  - Menghilangkan emit `void-item` dan state `pendingVoidItemIndex` dari alur keranjang kasir.
  - Otorisasi PIN supervisor tetap aktif untuk fungsi pembatalan nota transaksi dan pembukaan laci kasir manual.

### B. Dropdown Barang Autocomplete pada Input Barcode
- **File Dimodifikasi**: `src/components/pos/BarcodeBar.vue`
- **Fitur Komponen**:
  - **Live Search Debounced**: Pencarian ke API server `GET /products?search=...&limit=8` dengan debounce 120ms.
  - **Pratinjau Awal saat Fokus**: Saat kasir mengklik atau fokus pada kolom input barcode, sistem otomatis menampilkan daftar produk teratas jika kolom belum diisi.
  - **Navigasi Keyboard Penuh**:
    - `ArrowDown` / `ArrowUp` untuk memilih item dalam dropdown.
    - `Enter` untuk langsung memasukkan produk terpilih ke keranjang belanja.
    - `Escape` untuk menutup dropdown.
  - **Desain Ergonomis & Informatif**:
    - Setiap item menampilkan nama produk, kode SKU, barcode utama, kategori, lencana stok (`badge-stock`), dan harga jual dalam format rupiah standar Notane.
  - **Click-Outside Handler**: Dropdown otomatis tertutup saat pengguna mengklik area luar layar kasir.

### C. Akumulasi Kuantitas Otomatis & Penyesuaian Store
- **File Dimodifikasi**: `src/stores/cartStore.ts` & `src/components/pos/ProductSearchModal.vue`
- **Perbaikan Logika**:
  - `addProductToCart` diperbaiki agar melakukan penjumlahan kuantitas secara aritmatika eksplisit: `Number(currentQty) + Number(qtyToAdd)`.
  - Normalisasi pencocokan unit produk berbasis `unit_id` atau unit dasar default jika belum terpilih.
  - Menambahkan dan mengekspor fungsi pembantu `addProduct(product, quantity)` yang secara otomatis memetakan model produk API ke item keranjang serta memperbarui perhitungan diskon dan tier harga grosir.
  - Sinkronisasi pemilihan produk dari modal pencarian produk `F2` (`ProductSearchModal.vue`) agar menggunakan `cartStore.addProduct`.

### D. Perbaikan Akumulasi Kuantitas Barcode Timbangan (`200001000000`) & Stepper Cart
- **File Dimodifikasi**: `src/stores/cartStore.ts` & `src/components/pos/CartTable.vue`
- **Penyebab Masalah**:
  - Sebelumnya terdapat kondisi pembatas `&& !detail.product.is_weight_embedded` dan `&& !prod.is_weight_embedded` yang mengecualikan produk timbangan dari logika penambahan kuantitas saat sudah ada di keranjang.
  - Parsing bobot pada barcode master `200001000000` menghasilkan 0 gram (`00000`), sehingga kuantitas terhitung `0` alih-alih default `1`.
  - Tombol stepper `+` dan `-` pada `CartTable.vue` sebelumnya memiliki atribut `:disabled="item.product.is_weight_embedded"`.
- **Solusi**:
  - Menghapus pembatasan `is_weight_embedded` pada `existingIndex >= 0`, sehingga pemindaian berulang produk timbangan otomatis menambah kuantitas (`qty += 1`).
  - Barcode timbangan dengan nilai gram `0` (seperti barcode master `200001000000`) kini default bernilai `1` KG.
  - Mengaktifkan kembali tombol stepper `+` dan `-` di tabel keranjang kasir untuk semua jenis item termasuk barang timbangan dengan pembulatan presisi desimal 3 angka (`toFixed(3)`).

---

## 3. Hasil Pengujian & Verifikasi

### A. TypeScript & Vite Build
```bash
npm run build
# vue-tsc -b && vite build
# Exit code: 0 (Passed, 4.81s)
```

### B. Pengujian End-to-End dengan Browser Subagent
- **Skenario 1: Dropdown Autocomplete Barcode**:
  - Kasir mengetik kata *"Indo"* pada kolom input barcode.
  - Dropdown popover muncul tepat di bawah kolom input dengan hasil relevan (*Indomie Goreng Original*, SKU `IND-001`, Barcode `8992388101015`, Stok `150 PCS`, Rp 3.885).
- **Skenario 2: Akumulasi Kuantitas Otomatis**:
  - Produk *Indomie Goreng Original* dimasukkan ke keranjang (Qty: 1, Total: Rp 3.885).
  - Produk yang sama dipilih kembali.
  - Keranjang belanja tetap memiliki **1 baris**, dengan kolom kuantitas otomatis terakumulasi menjadi **2**, dan total harga menjadi **Rp 7.770**.
- **Skenario 3: Penghapusan Item Tanpa PIN Supervisor**:
  - Kasir menekan tombol ikon tong sampah (hapus) pada baris item tersebut.
  - Item langsung terhapus seketika tanpa muncul dialog PIN supervisor.
  - Status keranjang kembali ke *"Keranjang Belanja Masih Kosong"*.
- **Skenario 4: Barcode Timbangan `200001000000` (Apel Fuji)**:
  - Pemindaian pertama `200001000000` menambahkan Apel Fuji ke keranjang dengan Qty: 1 (Total: Rp 45.000).
  - Pemindaian kedua `200001000000` mempertahankan 1 baris keranjang dan mengakumulasikan kuantitas menjadi **Qty: 2** (Total: Rp 90.000).
  - Tombol stepper `+` diklik, kuantitas bertambah menjadi **Qty: 3** (Total: Rp 135.000).

