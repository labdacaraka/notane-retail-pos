# Progress 33: Jurnal Pembukuan Otomatis Potongan Penjualan (Flash Sale, Diskon Member, Voucher, Kasir)

## Ringkasan Fitur
Mengimplementasikan pencatatan akuntansi double-entry (*auto-journaling*) otomatis untuk setiap jenis potongan harga penjualan di sistem POS, mencakup:
1. **Diskon Flash Sale** (Akun `4102`)
2. **Diskon Member** (Akun `4103`)
3. **Diskon Voucher** (Akun `4104`)
4. **Diskon Kasir / Item Manual** (Akun `4105`)

Semua akun diskon dimasukkan ke dalam kategori `REVENUE` (kategori 4) sebagai akun kontra pendapatan (*contra-revenue account*) dengan saldo normal DEBIT, sesuai standar akuntansi ritel modern (PSAK).

---

## 1. Skema Akun Baru (Migrasi 018)
File: `src-tauri/migrations/018_sales_discount_accounts.sql`

| Kode Akun | Nama Akun | Kategori | Saldo Normal | Fungsi |
|---|---|---|---|---|
| `4101` | Pendapatan Penjualan Retail POS | REVENUE | CREDIT | Omzet penjualan kotor (*Gross Sales*) |
| `4102` | Potongan Penjualan - Flash Sale | REVENUE | DEBIT (Contra) | Menampung potongan dari promo Flash Sale |
| `4103` | Potongan Penjualan - Diskon Member | REVENUE | DEBIT (Contra) | Menampung potongan dari level/grup pelanggan member |
| `4104` | Potongan Penjualan - Diskon Voucher | REVENUE | DEBIT (Contra) | Menampung potongan dari penukaran voucher belanja |
| `4105` | Potongan Penjualan - Diskon Kasir / Item | REVENUE | DEBIT (Contra) | Menampung diskon manual yang diinput kasir |

---

## 2. Struktur Double-Entry POS Checkout
Di backend `src-tauri/src/services/pos_service.rs`:

### Sisi DEBIT:
- **Kas Operasional Kasir (1101)**: Jumlah uang tunai bersih yang masuk ke laci kasir
- **Bank / Piutang QRIS / EDC (1102)**: Jumlah non-tunai yang diterima
- **Potongan Penjualan - Flash Sale (4102)**: `total_flash_sale_discount`
- **Potongan Penjualan - Diskon Member (4103)**: `total_member_discount`
- **Potongan Penjualan - Diskon Voucher (4104)**: `voucher_discount`
- **Potongan Penjualan - Diskon Kasir / Item (4105)**: `manual_item_discount`
- **Beban Pokok Penjualan / HPP (5101)**: `total_cost`

### Sisi KREDIT:
- **Pendapatan Penjualan Retail POS (4101)**: `gross_sales_revenue` (Penjualan kotor sebelum diskon flash sale & member)
- **Hutang Pajak PPN Keluaran (2102)**: `total_tax`
- **Persediaan Barang Dagang Retail (1201)**: `total_cost`

$$\text{Total Debit} = \text{Total Kredit}$$

---

## 3. Integrasi Laporan Laba Rugi & Jurnal Umum
1. **Laba Rugi (`AccountingPage.vue` & `accounting_service.rs`)**:
   - Akun kontra pendapatan dihitung dengan rumus: $\sum(\text{credit} - \text{debit})$, menghasilkan nilai negatif yang otomatis mengurangi Gross Sales menjadi Net Sales secara proporsional.
   - Frontend menampilkan akun potongan secara elegan dengan warna pembeda (*rose-600*) sebagai pengurang omzet.
2. **Buku Jurnal Umum**:
   - Setiap transaksi checkout kasir mencatat baris debit potongan penjualan lengkap dengan keterangan/memo (termasuk kode voucher yang dipakai jika ada).
   - Pada pembatalan nota (*void sale*), sistem otomatis membalik seluruh baris jurnal debet/kredit secara akurat (*reversal journal*).
