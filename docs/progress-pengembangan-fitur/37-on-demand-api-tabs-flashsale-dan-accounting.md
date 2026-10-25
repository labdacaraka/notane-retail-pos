# Progress Fitur 37: On-Demand Lazy Loading API Tabs pada Menu Flash Sale & Pembukuan Akuntansi

## 1. Latar Belakang Masalah
1. **Menu Flash Sale (`FlashSalesPage.vue`)**:
   - Tab filter status program (`Semua Program`, `⚡ Sedang Aktif`, `⏳ Akan Datang`, `✓ Selesai`, `⊘ Nonaktif`) sebelumnya hanya memfilter di frontend via JavaScript `computed`, tanpa memanggil API ke server.
   - Panggilan API `/flash-sales` hanya dilakukan satu kali di awal tanpa query parameter `status`, `search`, `page`, dan `page_size`. Hal ini membuat tab terkesan tidak responsif atau tidak sinkron dengan filter database server.
2. **Menu Jurnal & Laba (`AccountingPage.vue`)**:
   - Saat halaman dibuka, sistem menjalankan `Promise.all` serentak yang memuat seluruh 5 endpoint akuntansi sekaligus:
     1. Laba Rugi (`/reports/income-statement`)
     2. Posisi Aset / Neraca (`/reports/balance-sheet`)
     3. Jurnal Umum (`/accounting/journal-entries`)
     4. Bagan Akun (`/accounting/chart-of-accounts`)
     5. Riwayat Tutup Buku (`/accounting/closings`)
   - Tombol tab sebelumnya hanya mengganti variabel `activeTab` lokal tanpa memanggil API terkait.
   - **Risiko Performa**: Ketika basis data transaksi membesar menjadi puluhan ribu hingga jutaan baris, meload seluruh modul sekaligus sangat boros memori, memperlambat database, dan dapat memicu timeout/out-of-memory.

---

## 2. Solusi & Perubahan yang Diterapkan

### A. Menu Flash Sale (`FlashSalesPage.vue`)
1. **Pemanggilan API Berbasis Permintaan (*On-Demand Server-Side Fetching*)**:
   - Fungsi `fetchFlashSales()` dirombak untuk mengirim query parameter langsung ke backend Axum:
     - `status`: Mengirim status yang dipilih (`ACTIVE`, `UPCOMING`, `EXPIRED`, `INACTIVE`) atau kosong jika `ALL`.
     - `search`: Debounce pencarian 350ms langsung ke backend (`ILIKE %query%`).
     - `page` & `page_size`: Pagination server-side terintegrasi.
2. **Tab Status Responsif & Berikon**:
   - Tombol tab kini dilengkapi icon Fluent yang jelas dan memicu `handleTabChange(tab.id)`, yang mereset halaman ke 1 dan langsung memanggil API dengan filter status terkait.
   - Menggunakan `res.data.total_items` dari respons server untuk komponen `PaginationBar`.
3. **Indikator Loading Server**:
   - Dilengkapi state spinner loading saat mengambil data dari server.

### B. Menu Jurnal & Laba (`AccountingPage.vue`)
1. **Sistem Lazy Loading & Caching Per Tab**:
   - Mengganti `fetchAllData()` serentak menjadi fungsi modular `fetchDataForTab(tab, force)`:
     - Tab `income`: HANYA memanggil `/reports/income-statement` (dengan filter outlet & rentang tanggal).
     - Tab `balance_sheet`: HANYA memanggil `/reports/balance-sheet` (dengan filter outlet & tanggal cut-off `as_of_date`).
     - Tab `journal`: HANYA memanggil `/accounting/journal-entries` (dengan filter outlet & rentang tanggal).
     - Tab `closing`: HANYA memanggil `/accounting/closings` (dengan filter outlet).
     - Tab `coa`: HANYA memanggil `/accounting/chart-of-accounts`.
   - Menggunakan flag `loadedTabs` agar data yang telah dimuat tidak di-request ulang saat user berpindah-pindah tab, kecuali user menekan tombol Refresh atau mengubah filter.
2. **Pemuatan Awal Ringan (*Lightweight Initial Load*)**:
   - Saat halaman dibuka (`onMounted`), hanya memuat daftar outlet dan data untuk tab pertama (`income`). Keempat endpoint lainnya tidak dipanggil sampai tab bersangkutan diklik oleh user.
3. **Penyegaran Sinkron (*Smart Invalidation on Filter Change*)**:
   - Ketika filter Cabang/Outlet atau Rentang Tanggal diubah:
     - Status `loadedTabs` direset.
     - Sistem langsung memuat ulang data mutakhir **HANYA untuk tab yang sedang aktif saat itu**.
4. **Indikator Loading Elegan**:
   - Konten tab dibungkus dengan indikator loading modern yang menginformasikan kepada pengguna bahwa server sedang mengambil data spesifik tab tersebut.

---

## 3. Hasil Pengujian
1. **Pengujian API Flash Sale**:
   - `GET /api/v1/flash-sales?status=ACTIVE&page=1&page_size=5` teruji mengembalikan data yang difilter di level query SQL dengan field `total_items`, `page`, dan `total_pages`.
2. **Pengujian Frontend Build**:
   - `npm run build` selesai tanpa error (lulus type-check TypeScript dan Vite bundling).
