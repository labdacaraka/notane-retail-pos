# Progress 67: Implementasi Server-Side Pagination, Sorting, Search pada Buku Jurnal Umum (Ledger) dan On-Demand Switching Tab Akuntansi

## Latar Belakang & Kebutuhan
1. **Tab Buku Jurnal Umum (Ledger)** pada halaman Akuntansi (`AccountingPage.vue` / `/admin/accounting`):
   - Sebelumnya membatasi pengambilan query secara hardcoded (`limit: 200`) dan melakukan paginasi di client-side (`paginatedJournalEntries = computed(...)`), sehingga data jurnal di atas 200 transaksi tidak dapat diakses dan tidak ada sorting interaktif.
   - Belum mendukung server-side pagination, sorting dinamis (berdasarkan tanggal, no jurnal, referensi, debet, kredit), dan pencarian teks cepat.
2. **Navigasi Antar-Tab Akuntansi**:
   - Fungsi `switchTab` sebelumnya memanfaatkan flag cache client (`loadedTabs[tab]`) dengan kondisi `if (loadedTabs.value[tab] && !force) return`, yang menyebabkan perpindahan tab tidak lagi memanggil server untuk memuat data terkini secara segar.

## Langkah Perbaikan yang Diterapkan

### 1. Backend (`notane-server`)
- **Struktur Query & Respons Model ([`models/accounting.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/models/accounting.rs)):**
  - Menambahkan `JournalQueryFilter` yang menerima parameter:
    - `page: Option<i64>`, `page_size: Option<i64>`
    - `sort_by: Option<String>`, `sort_dir: Option<String>`
    - `search: Option<String>` (pencarian teks pada nomor jurnal, deskripsi, jenis referensi, nama outlet)
    - `outlet_id: Option<Uuid>`, `start_date: Option<NaiveDate>`, `end_date: Option<NaiveDate>`
  - Menambahkan `PaginatedJournalEntries` untuk mengemas hasil paginasi: `data`, `total_items`, `total_pages`, `page`, dan `page_size`.
- **Query Dinamis ([`services/accounting_service.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/services/accounting_service.rs)):**
  - Menghitung `total_items` melalui `SELECT COUNT(*)::bigint` dengan kriteria filter lengkap.
  - Menerapkan dynamic sorting kolom (`journal_date`, `journal_number`, `reference_type`, `description`, `total_debit`, `total_credit`, `created_at`) dengan arah ASC/DESC.
  - Menerapkan `LIMIT page_size OFFSET (page - 1) * page_size` secara efisien di PostgreSQL.
- **API Handler ([`api/reports.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/api/reports.rs)):**
  - Mengupdate `journal_entries_handler` untuk merespons dengan format standar paginasi server (`data`, `total_items`, `total_pages`, `page`, `page_size`).

### 2. Frontend (`notane-client`)
- **State & Method Server-Side ([`AccountingPage.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-client/src/pages/AccountingPage.vue)):**
  - Mengganti computed slicing client dengan state server-side:
    - `jrnPage`, `jrnPageSize`, `jrnTotalItems`
    - `jrnSortBy`, `jrnSortDir`, `jrnSearch`
  - Membuat method `fetchJournalEntries()` yang mengirim parameter pagination, filter tanggal, outlet, sorting, dan kata kunci pencarian ke API backend.
  - Membuat `handleJrnSort(col)` untuk mengubah arah sorting atau kolom yang diurutkan saat header tabel diklik.
  - Membuat `handleJrnSearch()` untuk melakukan pencarian server-side.
- **Tampilan UI Tab Jurnal:**
  - Menambahkan toolbar pencarian cepat dengan icon search, clear button, dan ringkasan total transaksi jurnal.
  - Mengubah header tabel menjadi interaktif yang dapat diklik untuk sorting naik/turun dengan indikator panah (`↑`, `↓`).
  - Menghubungkan komponen `<PaginationBar>` langsung dengan state `jrnPage`, `jrnPageSize`, `jrnTotalItems`, serta event listener `@update:currentPage` dan `@update:pageSize` yang memicu pemanggilan server-side.
- **Navigasi Tab Selalu Server-Side:**
  - Mengubah `switchTab(tab)` agar selalu memanggil `fetchDataForTab(tab, true)`, sehingga setiap kali pengguna berpindah tab (Laba Rugi, Neraca, Jurnal, Tutup Buku, Setoran Modal, Bank, COA), data segar selalu di-fetch langsung dari backend.

## Hasil Pengujian & Verifikasi
- Pengujian cURL paginasi & sorting:
  ```bash
  curl -s "http://127.0.0.1:18080/api/v1/accounting/journal-entries?page=1&page_size=2&sort_by=journal_date&sort_dir=desc"
  ```
  Menghasilkan respons valid dengan total 86 entri jurnal, 43 halaman, dan 2 baris data per halaman.
- Pengujian cURL pencarian:
  ```bash
  curl -s "http://127.0.0.1:18080/api/v1/accounting/journal-entries?search=Pelunasan&page=1&page_size=2"
  ```
  Menghasilkan 6 transaksi jurnal terfilter secara server-side.
- Kompilasi backend `cargo check` dan `cargo build` berhasil sukses.
- Build frontend `npm run build` (`vue-tsc && vite build`) berhasil sukses tanpa error (`code 0`).
- Daemon server host berjalan aktif di port 18080.
