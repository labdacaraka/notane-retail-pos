# 43. Granular Action Permissions & Backend Semantic Validations

## Ringkasan Perubahan
Menindaklanjuti permintaan penambahan otorisasi spesifik untuk seluruh aksi operasional, termasuk:
1. **Aksi Pembatalan / Void Pembelian (`purchasing:void`)**:
   - Pembatalan Purchase Order (PO): `POST /inventory/purchase-orders/{id}/cancel`
   - Pembatalan Penerimaan Barang Fisik (GRN): `POST /inventory/goods-receipts/{id}/cancel`
   - Pembatalan Faktur Pembelian (Purchase Invoice): `POST /inventory/purchase-invoices/{id}/cancel`
   - Retur Pembelian: `POST /inventory/purchase-returns`
2. **Aksi Pembayaran Hutang Faktur Pembelian (`purchasing:pay`)**:
   - Pelunasan Faktur Pembelian: `POST /inventory/purchases/{id}/pay`
3. **Aksi Harga Khusus & Multi-Barcode Produk**:
   - Harga Khusus Kelompok Pelanggan / Member VIP (`products:customer_group_prices`):
     - `POST /products/{id}/customer-group-prices`
     - `DELETE /products/customer-group-prices/{id}`
     - `POST /products/{id}/outlet-customer-group-prices`
     - `DELETE /products/outlet-customer-group-prices/{id}`
   - Harga Jual Khusus Outlet Gerai (`products:outlet_prices`):
     - `POST /products/{id}/outlet-prices`
     - `DELETE /products/outlet-prices/{id}`
   - Multi-Barcode & Barcode Alias (`products:barcodes`):
     - `POST /products/{id}/barcodes`
     - `DELETE /products/{id}/barcodes/{barcode_id}`
     - `PUT /products/{id}/barcodes/{barcode_id}/set-primary`
   - Harga Grosir Bertingkat (`products:price_tiers`):
     - `POST /products/{id}/price-tiers`
     - `DELETE /products/price-tiers/{id}`
   - Import Katalog & Upload Foto (`products:import`):
     - `POST /products/import`
     - `POST /products/upload-photo`
4. **Aksi Fitur Lainnya**:
   - Pelanggan & Grup (`customers:manage`): `POST/PUT/DELETE /customers`, `/customer-groups`
   - Voucher Diskon (`vouchers:manage`): `POST/PUT/DELETE /vouchers`
   - Mutasi Transfer Stok (`inventory:transfer`): `POST /inventory/transfers`, `POST /inventory/transfers/{id}/receive`
   - Pembatalan Transfer Stok (`inventory:transfer_cancel`): `POST /inventory/transfers/{id}/cancel`
   - Tutup Buku Akuntansi (`accounting:close_period`): `POST /accounting/closings`
   - Buka Kembali Buku Akuntansi (`accounting:reopen_period`): `POST /accounting/closings/{id}/reopen`

---

## 1. Migrasi Database (029_action_feature_permissions.sql)
- Seluruh 13 permission aksi baru dimasukkan ke tabel `permissions` dan dipetakan ke role:
  - `SUPERADMIN`: seluruh permission aksi.
  - `STORE_MANAGER`: seluruh permission aksi kecuali yang dibatasi.
  - `SUPERVISOR`: permission void, bayar hutang, harga grup, harga outlet, barcode, transfer stok, voucher, customer.
  - `WAREHOUSE`: barcode dan transfer stok.
- Migrasi dieksekusi terintegrasi pada batch 19.

---

## 2. Validasi Semantik Backend (Rust)
- `save_product_group_price_handler`: Validasi `special_price > 0` dan `customer_group_id > 0`.
- `save_outlet_cgp_handler`: Validasi `special_price > 0` dan `customer_group_id > 0`.
- `save_product_outlet_price_handler`: Validasi `base_selling_price > 0` dan `cost_price >= 0`.
- `add_product_barcode_handler`: Validasi panjang barcode `2..=100` karakter.
- `add_product_tier_handler`: Validasi `min_quantity > 1` dan `tier_price > 0`.
- `cancel_purchase_order`: Validasi PO yang sudah berstatus `CANCELLED` tidak dapat dibatalkan ulang.
- `pay_purchase_invoice`: Validasi faktur yang berstatus `CANCELLED` tidak dapat dilunasi.

---

## 3. UI Guard Frontend (Vue.js)
1. **PurchasesPage.vue**:
   - Tombol dan handler `handleCancelPo`, `handleCancelGrn`, `handleCancelInvoice`, dan `handleSaveReturn` dijaga oleh permission `['purchasing:void', 'purchasing:write']`.
   - Tombol dan handler `handlePayInvoice` dijaga oleh permission `['purchasing:pay', 'purchasing:write']`.
   - Handler pembuatan PO, GRN, dan faktur dijaga oleh `purchasing:write`.
   - Handler master supplier dijaga oleh `['purchasing:write', 'master:write']`.
2. **ProductsPage.vue & ProductBarcodesModal.vue**:
   - Tombol dan modal barcode dijaga oleh `['products:barcodes', 'master:write']`.
   - Tombol dan modal harga outlet dijaga oleh `['products:outlet_prices', 'master:write']`.
   - Tombol dan modal harga grup pelanggan & member per outlet dijaga oleh `['products:customer_group_prices', 'master:write']`.
   - Tombol import produk massal dijaga oleh `['products:import', 'master:write']`.
3. **CustomersPage.vue**:
   - Penambahan, pengubahan, penonaktifan pelanggan dan grup member dijaga oleh `['customers:manage', 'master:write']`.
4. **VouchersPage.vue**:
   - Penambahan, pengubahan, penonaktifan kupon voucher diskon dijaga oleh `['vouchers:manage', 'master:write']`.
5. **AccountingPage.vue**:
   - Tombol dan dialog tutup buku periode dijaga oleh `['accounting:close_period', 'accounting:write']`.
   - Tombol buka kembali buku periode (reopen) dijaga oleh `['accounting:reopen_period', 'accounting:write']`.
   - Jurnal penyesuaian manual dijaga oleh `accounting:write`.
6. **InventoryPage.vue**:
   - Tombol simpan opname fisik dijaga oleh `inventory:adjust`.
   - Tombol buat transfer & terima transfer barang dijaga oleh `['inventory:transfer', 'inventory:write']`.
   - Tombol batalkan transfer barang dijaga oleh `['inventory:transfer_cancel', 'inventory:write']`.
