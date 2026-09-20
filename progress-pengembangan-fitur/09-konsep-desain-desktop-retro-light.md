# Milestone 09: Transformasi Total ke Desain Desktop Retro Light Mode (Tanpa Dark Mode)

## 1. Ringkasan Eksekutif & Permintaan Pengguna
Sesuai arahan dan preferensi pengguna:
> *"AKu tidak suka dark mode. Jangan darkmode. Biasanya konsep desain aplikasi desktop retro"*

Seluruh antarmuka **Notane Retail POS** (Front-Office Kasir, Customer Display, Back-Office ERP, Layar Login, serta seluruh Komponen Modal Dialog) telah ditransformasikan secara menyeluruh dari tema gelap (*dark mode*) menjadi **Light Retro Desktop Aesthetic**. 

Desain ini mereplikasi kejelasan, ketajaman kontras, dan nuansa taktil khas aplikasi desktop kasir/ERP profesional (kombinasi kanvas neutral gray/slate, panel kartu putih bersih dengan garis batas *border* solid, tabel spreadsheet data grid dengan header abu-abu klasik, serta tombol aksi bertingkat taktil), dipadukan dengan aksen warna logo resmi Notane (gradasi hijau lime ke toska/teal).

---

## 2. Rincian Perubahan Desain Sistem (Design System Tokens)

| Elemen UI | Nilai Desain Lama (Dark Mode) | Desain Baru (Light Retro Desktop) |
|---|---|---|
| **Root Canvas / Workspace** | `bg-slate-950 text-slate-100` | `bg-slate-100 text-slate-900` |
| **Panel / Kartu Konten** | `bg-slate-900 border-slate-800` | `bg-white border border-slate-300 rounded-xl shadow-sm` |
| **Header Ribbon / Top Bar** | `bg-slate-900 border-slate-800` | `bg-white border-b border-slate-300 shadow-xs` |
| **Navigasi Tab Modul** | `bg-slate-900/950 border-slate-800` | `bg-slate-200/80 border border-slate-300` |
| **Active Tab Pill** | `bg-brand-600 text-white` | `bg-white text-emerald-800 font-bold shadow-xs border border-slate-300/80` |
| **Header Tabel Data** | `bg-slate-950 text-slate-400 font-mono` | `bg-slate-100 border-b border-slate-300 text-slate-700 font-mono font-bold text-[11px] uppercase tracking-wider` |
| **Baris Tabel Data** | `divide-slate-800 hover:bg-slate-800/40` | `divide-y divide-slate-200 hover:bg-emerald-50/40` (selang-seling `bg-white` & `bg-slate-50/60`) |
| **Input & Dropdown Select** | `bg-slate-950 border-slate-700 text-white` | `bg-slate-50 border border-slate-300 rounded-lg text-slate-900 focus:bg-white focus:border-emerald-600 focus:outline-none` |
| **Tombol Utama (Primary CTA)** | `bg-brand-600 hover:bg-brand-500` | `bg-emerald-600 hover:bg-emerald-700 text-white font-bold border border-emerald-700 shadow-sm active:scale-95` |
| **Tombol Sekunder (Secondary)** | `bg-slate-800 text-slate-300` | `bg-slate-100 hover:bg-slate-200 text-slate-700 border border-slate-300 font-bold shadow-xs` |
| **Lencana Status (Badges)** | Latar gelap neon berbayang | Latar pastel kontras tinggi (`bg-emerald-50 text-emerald-800 border-emerald-300`, `bg-amber-50 text-amber-800 border-amber-300`, dsb.) |
| **Bilah Paginasi** | `bg-slate-900 border-slate-800` | `bg-slate-50 border-t border-slate-300 text-slate-600` dengan tombol kotak putih |
| **Modal / Dialog Popup** | `bg-slate-900 border-slate-800 text-white` | `bg-white border border-slate-300 shadow-2xl rounded-xl text-slate-900` dengan backdrop `bg-slate-900/50 backdrop-blur-xs` |
| **Scrollbars** | Scrollbar gelap | Scrollbar klasik retro desktop (`#e2e8f0` track, `#94a3b8` thumb) |

---

## 3. Berkas yang Diperbarui ke Light Retro Desktop

1. **Konfigurasi Global:**
   - `index.html`: Menghapus atribut `class="dark"` pada `<html>`, mengganti latar body ke `bg-slate-100 text-slate-900`.
   - `src/App.vue`: Mengubah wrapper global dari `bg-slate-950 text-slate-100` ke `bg-slate-100 text-slate-900 font-sans`.
   - `src/assets/main.css`: Menata variabel CSS `:root` warna cerah netral, styling scrollbar retro, dan utility `.bg-notane-gradient`.

2. **Komponen Bersama:**
   - `src/components/common/PaginationBar.vue`: Bilah navigasi halaman dengan kontras tinggi, tombol aktif hijau emerald dengan border solid, dan penunjuk halaman taktil.

3. **Layar Login:**
   - `src/pages/Login.vue`: Kartu login putih dengan logo resmi Notane, kolom isian abu-abu terang yang fokus ke putih, tombol demo akun uji coba dengan kontras tinggi.

4. **Back-Office ERP Layout & Pages:**
   - `src/layouts/AdminLayout.vue`: Title bar atas putih bersih, pita navigasi ribbon tombol abu-abu retro desktop, dan kanvas konten netral `bg-slate-100`.
   - `src/pages/AdminDashboard.vue`: Hero panel selamat datang putih elegan dan grid kartu launcher modular ("kotak-kotak di tengah") bergaris strip warna cerah di bagian atas.
   - `src/pages/ProductsPage.vue`: Master data produk & penetapan harga dengan tabel spreadsheet putih, strip baris bergradasi saat di-hover, serta modal CRUD produk & harga bertingkat.
   - `src/pages/CustomersPage.vue`: Master data member pelanggan & grup pelanggan dengan tab putih, badge poin belanja, dan form modal cerah.
   - `src/pages/VouchersPage.vue`: Kartu KPI putih bersih, tabel kupon voucher diskon, dan dialog penambahan kupon promosi.
   - `src/pages/ParametersPage.vue`: 4 master parameter retail (Satuan UoM, Kategori Barang, Merek/Prinsipal, Distributor Pemasok) berbalut tabel putih bersih dan modal form terintegrasi.
   - `src/pages/InventoryPage.vue`: Tab Stock Opname fisik, Penerimaan Barang Faktur Supplier (GRN), dan Kartu Stok (Ledger) dengan input selisih fisik yang kontras dan jelas.
   - `src/pages/AccountingPage.vue`: Kartu ringkasan Laba Rugi, rincian akun pendapatan/beban, Buku Jurnal Umum (General Ledger), dan Bagan Akun (COA).
   - `src/pages/AuditLogPage.vue`: Rekaman log jejak audit keamanan kasir & supervisor dengan badge aksi VOID merah pastel dan hijau pastel.

5. **Front-Office Layar Kasir POS & Komponen:**
   - `src/layouts/PosLayout.vue`: Header kasir putih berkelas dengan indikator shift aktif, status LAN host, jam digital monospasi, dan footer shortcut keyboard (`F1` - `F12`, `Esc`).
   - `src/pages/PosRegister.vue`: Grid kasir dengan banner member cerah, panel grand total tagihan raksasa berwarna emerald di atas kanvas abu-abu terang, rincian subtotal & PPN, serta modal member & kupon diskon.
   - `src/components/pos/BarcodeBar.vue`: Kolom input barcode berbingkai hijau emerald dengan tombol cepat `Cari (F1)`, `Member`, `Voucher`, `Laci (F4)`, `Tahan (F6)`, `Panggil (F7)`, dan `Bayar (F12)`.
   - `src/components/pos/CartTable.vue`: Tabel keranjang belanja kasir bergaya grid spreadsheet klasik, stepper kuantitas taktil, dan badge produk timbangan.
   - `src/components/pos/QuickPayModal.vue`: Dialog pembayaran kasir putih dengan opsi Tunai, QRIS dinamis, EDC, dan Poin Member, lengkap dengan tombol pecahan uang pas / cepat (`+10.000`, `+50.000`, `+100.000`), serta struk thermal kasir.
   - `src/components/pos/ShiftModal.vue`: Modal buka shift (modal awal floating cash), tutup shift (blind settlement kasir), dan pratinjau cetak Laporan Z-Report shift.
   - `src/components/pos/HoldRecallModal.vue`: Modal antrean transaksi tertunda dengan daftar belanjaan pelanggan yang siap dipanggil kembali.
   - `src/components/pos/ProductSearchModal.vue`: Modal pencarian produk manual dengan kotak pencarian instan dan kartu produk cerah.
   - `src/components/pos/SupervisorPinModal.vue`: Dialog otorisasi PIN supervisor toko dengan keypad numerik taktil.
   - `src/pages/CustomerDisplay.vue`: Layar hadap pelanggan (Customer Facing Display) di monitor kedua berlatar cerah dengan daftar 5 barang belanjaan terakhir dan total pembayaran raksasa.

---

## 4. Hasil Verifikasi & Kompilasi
- **Kompilasi TypeScript & Vue SFC:** `npm run build` (`vue-tsc && vite build`) berhasil dengan **Exit Code 0** tanpa ada satupun *syntax error* atau *type mismatch*.
- **Pemeriksaan Browser Real-Time:** Dilakukan via Browser Subagent pada port `http://localhost:5175`, mendokumentasikan tampilan halaman Login, Admin Dashboard, dan Master Produk yang kini sepenuhnya mengadopsi Light Retro Desktop Aesthetic.
- **Tangkapan Layar Verifikasi:**
  - Login Page: `login_page_1789862502923.png`
  - Admin Dashboard Hub: `admin_dashboard_page_1789862559369.png`
  - Master Produk Table: `admin_products_page_1789862566543.png`
