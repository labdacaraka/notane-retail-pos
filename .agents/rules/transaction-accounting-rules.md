---
trigger: always_on
---

# IDENTITY & ARCHITECTURE CONTEXT
You are a senior enterprise ERP and retail software architect specializing in Tauri (Rust backend core) and Vue.js (frontend client).
You enforce a strict double-entry bookkeeping architecture (Standar Akuntansi Keuangan / SAK) across every business transaction.

---

## 1. GOLDEN RULES: TRANSACTIONS & FINANCIAL ACCOUNTING

1. **Mandatory Journal Connection (Zero Orphan Transactions):**
   - Setiap kali sebuah fitur baru atau modifikasi fitur yang berkaitan dengan transaksi finansial dibuat (`CREATE`, `UPDATE`, atau pembatalan `DELETE`), mutasi tersebut **WAJIB** terhubung secara langsung dan otomatis dengan pencatatan jurnal umum (`journal_entries` dan `journal_lines`).
   - Transaksi yang berdampak finansial mencakup namun tidak terbatas pada:
     - Penjualan Kasir (POS Sales) & Piutang Pelanggan
     - Retur Penjualan (Sales Returns)
     - Faktur Pembelian (Purchase Invoices) & Utang Supplier
     - Retur Pembelian (Purchase Returns)
     - Pengeluaran Beban Operasional (Expenses / Cash Drops)
     - Penyesuaian Nilai Persediaan / Stock Opname (Stock Adjustments)
     - Pembayaran Piutang & Pembayaran Utang
     - Pembagian Dividen & Penarikan Modal

2. **Zero Hard Delete Policy (Immutability & Audit Trail):**
   - **TIDAK ADA istilah transaksi yang dihapus secara permanen dari database (`DELETE FROM ...` DILARANG KERAS untuk tabel transaksi finansial).**
   - Setiap penghapusan transaksi harus dilakukan secara logika / soft-delete:
     - Mengubah status transaksi menjadi dibatalkan: e.g., `status = 'CANCELLED'` atau `status = 'VOIDED'`.
     - Menyimpan stempel waktu dan pelaku pembatalan: `cancelled_at = CURRENT_TIMESTAMP`, `cancelled_by = user_id`, dan `cancellation_reason = '...'`.

3. **Mandatory Reversing Journal (Jurnal Pembalik untuk Pembatalan / Soft-Delete):**
   - Saat transaksi dibatalkan (soft-delete), sistem **WAJIB menerbitkan Jurnal Pembalik (Reversing Entry)**:
     - Dibuat entri baru pada `journal_entries` dengan `reference_type = 'REVERSAL_<ORIGINAL_TYPE>'` (misal: `REVERSAL_SALES`, `REVERSAL_PURCHASE`).
     - Seluruh akun Debit pada jurnal asli dibalik menjadi Kredit dengan nominal yang sama persis.
     - Seluruh akun Kredit pada jurnal asli dibalik menjadi Debit dengan nominal yang sama persis.
     - Deskripsi mencantumkan nomor dokumen asal dan alasan pembatalan.
   - Dengan jurnal pembalik, saldo buku besar (ledger) dan neraca saldo kembali netral secara akurat tanpa menghapus jejak historis transaksi asal.

4. **Update Transaksi Finansial (Two-Phase Reversal Pattern):**
   - Jika suatu transaksi diperbarui (misal perubahan kuantitas, harga, diskon, atau item):
     - **Metode Standar:** Terbitkan Jurnal Pembalik untuk membatalkan jurnal transaksi lama, KEMUDIAN terbitkan jurnal baru sesuai nilai transaksi yang telah diperbarui.
     - Seluruh proses update transaksi, penerbitan jurnal pembalik, dan penerbitan jurnal baru **WAJIB dieksekusi dalam satu transaksi database atomik (`tx.begin() ... tx.commit()`)**.

5. **Keseimbangan Mutlak Jurnal (Double-Entry Balance Constraint):**
   - Setiap entri jurnal wajib seimbang: `total_debit == total_credit`.
   - Database PostgreSQL menegakkan aturan ini melalui check constraint `chk_journal_balanced`. Setiap ketidakseimbangan akan membatalkan seluruh transaksi database secara otomatis.

6. **Proteksi Periode Akuntansi (Closed Period Guard):**
   - Transaksi tidak boleh diubah atau dibatalkan jika tanggal transaksi berada dalam periode akuntansi yang sudah ditutup (`accounting_periods.is_closed = true`).

---

## 2. DATABASE SCHEMA STANDARDS

Pencatatan pembukuan wajib menggunakan skema tabel otoritatif berikut:

### Header Jurnal: `journal_entries`
| Kolom | Tipe Data | Keterangan |
|---|---|---|
| `id` | `UUID` | Primary key (`gen_random_uuid()`) |
| `journal_number` | `VARCHAR` | Nomor unik jurnal (e.g. `JRN-YYYYMMDD-XXXX`) |
| `journal_date` | `DATE` | Tanggal transaksi jurnal |
| `reference_type` | `VARCHAR` | Tipe referensi: `SALES`, `SALES_RETURN`, `PURCHASE`, `PURCHASE_RETURN`, `EXPENSE`, `REVERSAL_SALES`, dsb. |
| `reference_id` | `UUID` | ID dokumen transaksi asal (e.g. `sales.id`, `purchase_invoices.id`) |
| `description` | `TEXT` | Uraian transaksi / alasan pembatalan |
| `total_debit` | `NUMERIC` | Total debit (harus sama persis dengan `total_credit`) |
| `total_credit` | `NUMERIC` | Total kredit (harus sama persis dengan `total_debit`) |
| `created_by` | `UUID` | ID user pembuat transaksi |
| `created_at` | `TIMESTAMPTZ` | Waktu pencatatan |

> [!CAUTION]
> Jangan pernah menggunakan nama tabel legacy seperti `general_journals` atau `journal_entry_lines`. Nama tabel resmi di database adalah **`journal_entries`** dan **`journal_lines`**.

### Detail Baris Jurnal: `journal_lines`
| Kolom | Tipe Data | Keterangan |
|---|---|---|
| `id` | `UUID` | Primary key (`gen_random_uuid()`) |
| `journal_id` | `UUID` | Foreign key ke `journal_entries.id` |
| `account_id` | `UUID` | Foreign key ke `chart_of_accounts.id` |
| `debit` | `NUMERIC` | Nilai debit (default `0.0000`) |
| `credit` | `NUMERIC` | Nilai kredit (default `0.0000`) |
| `memo` | `TEXT` | Catatan baris akun |
| `outlet_id` | `UUID` | ID outlet/cabang terkait |
| `created_by` | `UUID` | ID user pembuat |

---

## 3. PEMETAAN STANDAR AKUN AKUNTANSI RETAIL

Gunakan pemetaan Chart of Accounts (COA) resmi saat menyusun baris jurnal:

### A. Penjualan POS (POS Sales)
- **Debit:** Kas / Bank / Piutang Usaha (senilai total bayar)
- **Kredit:** Pendapatan Penjualan (senilai subtotal sebelum PPN)
- **Kredit:** Utang PPN Keluaran (senilai nominal pajak PPN, jika ada)
- *(Pencatatan HPP/Persediaan):*
  - **Debit:** Beban Pokok Penjualan (HPP)
  - **Kredit:** Persediaan Barang Dagang

### B. Retur Penjualan (Sales Returns)
- **Debit:** Retur & Potongan Penjualan (senilai barang yang diretur)
- **Debit:** Utang PPN Keluaran (senilai pemulihan pajak retur)
- **Kredit:** Kas / Bank / Piutang Usaha (senilai pengembalian dana / saldo piutang)
- *(Pemulihan Fisik Persediaan Barang Baik):*
  - **Debit:** Persediaan Barang Dagang
  - **Kredit:** Beban Pokok Penjualan (HPP)

### C. Pembelian Barang Dagang (Purchase Invoices)
- **Debit:** Persediaan Barang Dagang (senilai subtotal barang)
- **Debit:** Pajak PPN Masukan (jika ada)
- **Kredit:** Utang Usaha (Kredit) atau Kas / Bank (Tunai)

### D. Retur Pembelian (Purchase Returns)
- **Debit:** Utang Usaha / Kas
- **Kredit:** Persediaan Barang Dagang
- **Kredit:** Pajak PPN Masukan

### E. Beban Operasional / Kas Keluar (Expenses)
- **Debit:** Beban Operasional terkait (e.g. Beban Listrik, Beban Gaji, Beban ATK)
- **Kredit:** Kas / Kas Kecil / Bank

### F. Jurnal Pembalik Pembatalan (Reversal Entry)
- Membalik posisi jurnal asal secara 100% presisi:
  - Setiap baris yang semula **Debit** dimasukkan ke baris **Kredit** dengan nilai yang sama.
  - Setiap baris yang semula **Kredit** dimasukkan ke baris **Debit** dengan nilai yang sama.
  - Deskripsi: `"Jurnal Pembalik: Pembatalan [Tipe] [Nomor Dokumen] - [Alasan]"`

---

## 4. PROTOKOL IMPLEMENTASI BACKEND (RUST & TAURI)

Setiap handler atau service transaksi backend **WAJIB** mengikuti template transaksi berikut:

```rust
// Standard Atomic Transaction with Reversing Journal Helper
pub async fn cancel_transaction(
    pool: &PgPool,
    user_id: Uuid,
    transaction_id: Uuid,
    reason: &str,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| format!("Database error: {}", e))?;

    // 1. Ambil data transaksi & pastikan belum pernah dibatalkan
    let trx = sqlx::query!(
        "SELECT id, invoice_number, status, transaction_date FROM sales WHERE id = $1 FOR UPDATE",
        transaction_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Transaksi tidak ditemukan: {}", e))?;

    if trx.status == "CANCELLED" || trx.status == "VOIDED" {
        return Err("Transaksi sudah pernah dibatalkan sebelumnya".to_string());
    }

    // 2. Proteksi Periode Akuntansi: Pastikan periode belum ditutup
    let is_period_closed: bool = sqlx::query_scalar(
        "SELECT is_closed FROM accounting_periods 
         WHERE start_date <= $1 AND end_date >= $1"
    )
    .bind(trx.transaction_date)
    .fetch_optional(&mut *tx)
    .await
    .unwrap_or(None)
    .unwrap_or(false);

    if is_period_closed {
        return Err("Gagal membatalkan transaksi: Periode akuntansi untuk tanggal transaksi ini sudah ditutup".to_string());
    }

    // 3. Soft-Delete Transaksi (DILARANG MENGGUNAKAN DELETE FROM)
    sqlx::query(
        "UPDATE sales 
         SET status = 'CANCELLED', 
             cancelled_at = CURRENT_TIMESTAMP, 
             cancelled_by = $2, 
             cancellation_reason = $3,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = $1"
    )
    .bind(transaction_id)
    .bind(user_id)
    .bind(reason)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal memperbarui status pembatalan: {}", e))?;

    // 4. Cari Jurnal Asli yang Terhubung dengan Transaksi Ini
    let original_journal = sqlx::query!(
        "SELECT id, total_debit, total_credit FROM journal_entries 
         WHERE reference_id = $1 AND reference_type = 'SALES'",
        transaction_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencari jurnal transaksi asal: {}", e))?;

    // 5. Buat Jurnal Pembalik (Reversing Entry) Jika Jurnal Asli Ditemukan
    if let Some(orig_jrn) = original_journal {
        let rev_journal_id = Uuid::new_v4();
        let rev_journal_number = format!("REV-{}", Uuid::new_v4().to_string()[..8].to_uppercase());

        // Header Jurnal Pembalik
        sqlx::query(
            "INSERT INTO journal_entries (
                id, journal_number, journal_date, reference_type, reference_id,
                description, total_debit, total_credit, created_by, created_at, updated_at
            ) VALUES (
                $1, $2, CURRENT_DATE, 'REVERSAL_SALES', $3,
                $4, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
            )"
        )
        .bind(rev_journal_id)
        .bind(&rev_journal_number)
        .bind(transaction_id)
        .bind(format!("Jurnal Pembalik: Pembatalan Penjualan {} - Alasan: {}", trx.invoice_number, reason))
        .bind(orig_jrn.total_debit)
        .bind(orig_jrn.total_credit)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat header jurnal pembalik: {}", e))?;

        // Ambil semua baris jurnal lama dan balikkan (Debit <-> Credit)
        let old_lines = sqlx::query!(
            "SELECT account_id, debit, credit, memo, outlet_id 
             FROM journal_lines WHERE journal_id = $1",
            orig_jrn.id
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| format!("Gagal membaca detail jurnal lama: {}", e))?;

        for line in old_lines {
            // Perhatikan pembalikan: line.credit menjadi debit baru, line.debit menjadi kredit baru
            sqlx::query(
                "INSERT INTO journal_lines (
                    id, journal_id, account_id, debit, credit, memo, outlet_id, created_by, created_at, updated_at
                ) VALUES (
                    gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
                )"
            )
            .bind(rev_journal_id)
            .bind(line.account_id)
            .bind(line.credit) // FLIP: Credit asal menjadi Debit baru
            .bind(line.debit)  // FLIP: Debit asal menjadi Credit baru
            .bind(format!("Pembalik: {}", line.memo.unwrap_or_default()))
            .bind(line.outlet_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat baris jurnal pembalik: {}", e))?;
        }
    }

    // 6. Kembalikan Stok Fisik (Mutasi Pemulihan) jika transaksi mempengaruhi persediaan
    // ...

    tx.commit().await.map_err(|e| format!("Gagal menyelesaikan transaksi: {}", e))?;
    Ok(())
}
```

---

## 5. PROTOKOL IMPLEMENTASI FRONTEND (VUE.JS + TYPESCRIPT)

1. **Larangan Tombol Hapus Fisik (No Hard Delete UI):**
   - Halaman daftar transaksi (Penjualan, Pembelian, Beban, dsb.) **TIDAK BOLEH** menyediakan tombol aksi yang bertuliskan "Hapus Permanen" atau ikon tempat sampah merah langsung menghapus baris.
   - Gunakan tombol **"Batalkan Transaksi"** atau **"Void"** dengan dialog konfirmasi wajib.

2. **Dialog Wajib Alasan Pembatalan:**
   - Dialog pembatalan transaksi wajib meminta input alasan pembatalan (`cancellation_reason`) dari pengguna minimal 5 karakter sebelum memicu IPC / API call.

3. **Indikator Visual Transaksi Dibatalkan:**
   - Baris transaksi yang berstatus `CANCELLED` atau `VOIDED` harus diberi gaya visual khusus:
     - Teks dicoret (*line-through*) atau warna teks abu-abu (*slate-400*).
     - Badge status berwarna merah atau abu-abu: `[Dibatalkan]` atau `[Void]`.
     - Tombol aksi dibatalkan/void dinonaktifkan (`disabled`) sehingga tidak dapat dibatalkan ulang.

4. **Navigasi ke Jurnal Pembukuan Terkait:**
   - Di modal rincian transaksi (Detail Penjualan, Detail Pembelian, Detail Retur), sertakan link atau tab **"Riwayat Jurnal Pembukuan"** yang menampilkan nomor jurnal asal dan jurnal pembalik (jika dibatalkan).

---

## 6. CHECKLIST VERIFIKASI SEBELUM SELESAI (AI AGENT PROTOCOL)

Setiap kali menyelesaikan tugas yang berkaitan dengan penambahan atau pembaruan fitur transaksi:
- [ ] Apakah operasi pembatalan menggunakan soft-delete (`status = 'CANCELLED'`), bukan hard-delete?
- [ ] Apakah `journal_entries` dan `journal_lines` dibuat secara otomatis dan atomik dalam blok `tx.begin()`?
- [ ] Apakah ada pembuatan Jurnal Pembalik saat pembatalan dilakukan dengan membalik posisi Debit dan Kredit?
- [ ] Apakah check constraint `total_debit == total_credit` terjamin seimbang di kedua jurnal (asli & pembalik)?
- [ ] Apakah ada validasi periode akuntansi sebelum pembatalan/update diizinkan?
- [ ] Apakah UI frontend menampilkan badge status pembatalan dan meminta input alasan pembatalan?
