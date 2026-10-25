# Progress Log 12: Dropdown Searchable (Select2 Style) Terintegrasi API & Redesain Ribbon Menu Desktop

**Tanggal**: 20 September 2026  
**Status**: Selesai (Completed & Verified)  
**Fitur**: 
1. Komponen Universal Dropdown Searchable (Select2 Style) dengan integrasi pemanggilan REST API real-time.
2. Redesain Command Ribbon Menu Desktop Back-Office dengan ikon besar di tengah dan label teks di bawah.

---

## 1. Latar Belakang & Permintaan Pengguna

1. **Setiap Dropdown Wajib Memanggil API & Searchable (Select2 Style)**:
   - Dropdown bawaan HTML `<select>` sulit digunakan pada data master yang banyak (ratusan kategori, puluhan satuan, ratusan supplier, atau ribuan produk).
   - Pengguna membutuhkan dropdown interaktif mirip library **Select2**: memiliki kotak pencarian di dalam popover dropdown, otomatis memanggil API server untuk mengambil daftar data terbaru, mendukung navigasi keyboard (Panah Atas/Bawah, Enter, Escape), serta tombol reset / hapus pilihan.
2. **Desain Tombol Menu Desktop Klasik (Ikon Besar di Tengah & Label di Bawah)**:
   - Pada aplikasi desktop POS / ERP profesional, tombol ribbon menu aksi biasanya didesain bertumpuk secara vertikal (vertical stack): ikon besar berada di posisi tengah atas dan teks keterangan menu berada tepat di bawahnya.
   - Tata letak ini memaksimalkan kemudahan klik (*target area* luas), ramah layar sentuh (*touch-friendly*), serta mempertahankan estetika desktop retro yang rapi dan tegas.

---

## 2. Arsitektur & Implementasi Solusi

### A. Komponen Reusable `SearchableSelect.vue`
Dibuat komponen khusus di `src/components/common/SearchableSelect.vue` dengan kapabilitas lengkap:
- **Integrasi REST API**:
  - Menerima prop `apiEndpoint` (misal: `/categories`, `/units`, `/suppliers`, `/customer-groups`, `/products`).
  - Mengambil data otomatis menggunakan client Axios bawaan saat komponen dimuat atau popover dibuka.
  - Opsi juga dapat disuplai manual via prop `options` jika menggunakan data lokal/statis.
- **Fitur Interaksi & UX (Select2 Style)**:
  - **Live Filter Search Input**: Input teks di bagian atas popover dengan auto-focus instan saat dropdown dibuka.
  - **Pencarian Real-Time**: Menyaring opsi berdasarkan teks label secara instan tanpa jeda.
  - **Badge Total & Status Kosong**: Menampilkan jumlah opsi yang cocok (misal: `3 opsi`), indikator loading saat fetch data, serta pesan informatif jika tidak ada data yang cocok.
  - **Dukungan Navigasi Keyboard**:
    - `ArrowDown` / `ArrowUp`: Memindahkan sorotan item daftar opsi.
    - `Enter`: Memilih item yang sedang disorot.
    - `Escape`: Menutup dropdown secara instan.
  - **Clear Button**: Tombol `(x)` untuk menghapus pilihan yang sudah dipilih kembali ke state awal (*placeholder*).
  - **Deteksi Klik Luar (Click-Outside)**: Otomatis menutup popover dropdown jika pengguna mengklik area luar modal.
  - **Desain Retro Desktop**: Border solid slate tebal (`border-slate-400`), font monospace pada badge, ring fokus warna amber hangat (`ring-amber-400`), dan kontras teks tinggi.

### B. Integrasi pada Halaman-Halaman Utama
Komponen `SearchableSelect` telah diintegrasikan pada form-form kritis:
1. **`ProductsPage.vue`**:
   - Field **Kategori**: Mengambil data dinamis dari `GET /api/v1/categories`.
   - Field **Satuan Dasar**: Mengambil data dinamis dari `GET /api/v1/units`.
2. **`InventoryPage.vue`**:
   - Field **Supplier Penerimaan Barang (GRN)**: Mengambil data dari `GET /api/v1/suppliers`.
   - Field **Pencarian Produk Kartu Stok (Stock Ledger)**: Mengambil data dari `GET /api/v1/products`.
3. **`CustomersPage.vue`**:
   - Field **Grup Pelanggan / Diskon Member**: Mengambil data dari `GET /api/v1/customer-groups`.

### C. Redesain Command Ribbon Menu Desktop (`AdminLayout.vue`)
Tombol-tombol menu navigasi di header back-office dirombak dari format horizontal tipis menjadi format tombol desktop klasik:
- **Struktur Vertikal**: `flex flex-col items-center justify-center`.
- **Dimensi**: Ukuran tetap dengan min-width ergonomis (`min-w-[88px] h-[64px]`).
- **Ikon Besar di Tengah**: Ikon berukuran `w-6 h-6` (24x24px) dengan stroke tegas di posisi tengah atas.
- **Label Teks di Bawah**: Label menu berada tepat di bawah ikon dengan ukuran `text-[10.5px] font-bold uppercase tracking-tight text-center leading-tight`.
- **Styling Retro Desktop**:
  - State Normal: Background `bg-slate-100`, border `border-slate-300`, shadow tipis `shadow-sm`.
  - State Hover: `hover:bg-amber-100 hover:border-amber-400 hover:text-amber-900`.
  - State Aktif: `bg-amber-200 border-amber-600 text-amber-950 shadow-inner font-extrabold`.

---

## 3. Hasil Pengujian & Verifikasi Visual

1. **Pengujian Visual Ribbon Menu Desktop**:
   - Seluruh tombol menu utama (*Dashboard*, *Produk & Stok*, *Data Pelanggan*, *Kontrol Host*, dll.) sukses tampil dengan ikon besar di tengah dan teks berada di bawah.
   - Sesuai dengan estetika aplikasi desktop klasik / retro POS.
2. **Pengujian Searchable Dropdown Terintegrasi API**:
   - Membuka form modal *Tambah Produk*.
   - Dropdown Kategori dan Satuan langsung memanggil API backend (`/categories` dan `/units`) dan menampilkan opsi yang ada di database (contoh: *Makanan & Minuman*, *Pieces / Satuan*).
   - Memasukkan kata kunci pencarian di kotak input pencarian dropdown -> daftar opsi tersaring secara langsung dan responsif.
   - Memilih opsi menggunakan klik mouse maupun tombol keyboard (`ArrowDown` + `Enter`) berjalan sempurna dan nilai terikat secara dua arah (*v-model binding*).
3. **Verifikasi Build Frontend**:
   - Perintah `npm run build` dijalankan dan sukses 100% tanpa error kompilasi TypeScript atau Vue SFC.
