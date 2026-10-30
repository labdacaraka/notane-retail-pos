# ATURAN WAJIB STRUKTUR DATABASE (DATABASE SCHEMA RULES)

Dokumen ini mengikat seluruh AI Agent dan developer saat membaca, menulis kueri SQL, membuat migrasi, atau merefaktor kode di repositori `notane-retail-pos` (baik `notane-server` maupun `notane-manager`).

---

## 1. PROTOKOL & ATURAN EMAS INTEGRITAS DATABASE (GOLDEN RULES)

1. **Dilarang Berasumsi Nama Tabel dan Kolom:**
   - Sebelum menulis kueri `SELECT`, `INSERT`, `UPDATE`, atau `JOIN`, **WAJIB** mengecek dokumen ini untuk memastikan nama tabel dan kolom persis sesuai skema riil.
   - **Daftar Gotchas yang Sering Salah:**
     - `sales`: Kasir disimpan di kolom `created_by` (BUKAN `cashier_id`). Metode bayar ada di tabel `sale_payments` (BUKAN di `sales`). Status retur ada di `return_status` dan `total_refund_amount`.
     - `sale_items`: Harga jual disimpan di `unit_selling_price` (BUKAN `unit_price`). Satuan di `unit_id` (BUKAN `unit_code`).
     - `products`: Satuan dasar disimpan di `base_unit_id` (BUKAN `unit_code`).
     - Mutasi Stok: Tabel resmi adalah `stock_mutations` (BUKAN `stock_movements`). Kolom kuantitas adalah `qty_in`, `qty_out`, dan `balance`.
     - Jurnal Akuntansi: Tabel header adalah `journal_entries` (BUKAN `general_journals`). Tabel baris detail adalah `journal_lines` (BUKAN `journal_entry_lines`). Wajib seimbang (`total_debit == total_credit`) sesuai check constraint `chk_journal_balanced`.
2. **Wajib Memperbarui Dokumen Ini Saat Ada Migrasi Baru:**
   - Setiap kali membuat file migrasi SQL baru di `notane-server/src-tauri/migrations/` atau menambah tabel/kolom di `notane-manager/src-tauri/src/db/mod.rs`, **kamus struktur di dokumen ini WAJIB langsung di-update**.
3. **Registrasi Migrasi Baru di `notane-server/src-tauri/src/db/mod.rs`:**
   - Setiap file SQL baru di `migrations/` WAJIB didaftarkan ke array `migrations: &[(&str, &str)]` di `notane-server/src-tauri/src/db/mod.rs` agar otomatis tereksekusi pada saat server berjalan.
4. **Larangan Keras Mengabaikan Error di Dalam Transaksi PostgreSQL:**
   - Dilarang keras menggunakan `let _ = sqlx::query(...)` di dalam blok transaksi (`&mut *tx`). Di PostgreSQL, satu kueri gagal langsung membuat transaksi masuk ke kondisi *aborted state* (`current transaction is aborted, commands ignored until end of transaction block`).
   - Semua eksekusi kueri di dalam transaksi wajib menggunakan error handling ketat (`?` atau `.map_err()`).

---

## 2. KAMUS SKEMA DATABASE NOTANE-SERVER (POSTGRESQL `notane_pos`)

### 1. Auth, Pengguna & Organisasi

#### Tabel `users`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `role_id` | `integer` | NO | `-` | roles.id |
| `username` | `character varying` | NO | `-` | - |
| `password_hash` | `character varying` | NO | `-` | - |
| `pin_hash` | `character varying` | NO | `-` | - |
| `full_name` | `character varying` | NO | `-` | - |
| `email` | `character varying` | YES | `-` | - |
| `phone` | `character varying` | YES | `-` | - |
| `is_active` | `boolean` | NO | `true` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `biometric_credential_id` | `character varying` | YES | `-` | - |
| `biometric_public_key` | `text` | YES | `-` | - |
| `face_descriptor` | `text` | YES | `-` | - |
| `face_photo_preview` | `text` | YES | `-` | - |
| `has_biometric` | `boolean` | NO | `false` | - |
| `has_face_recognition` | `boolean` | NO | `false` | - |
| `current_outlet_id` | `uuid` | YES | `-` | outlets.id |

#### Tabel `roles`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `integer` | NO | `nextval('roles_id_seq'::reg...` | - |
| `name` | `character varying` | NO | `-` | - |
| `description` | `text` | YES | `-` | - |
| `is_active` | `boolean` | NO | `true` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `permissions`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `integer` | NO | `nextval('permissions_id_seq...` | - |
| `code` | `character varying` | NO | `-` | - |
| `description` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

#### Tabel `role_permissions`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `role_id` | `integer` | NO | `-` | roles.id |
| `permission_id` | `integer` | NO | `-` | permissions.id |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

#### Tabel `outlets`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `code` | `character varying` | NO | `-` | - |
| `name` | `character varying` | NO | `-` | - |
| `address` | `text` | YES | `-` | - |
| `phone` | `character varying` | YES | `-` | - |
| `is_active` | `boolean` | NO | `true` | - |
| `is_warehouse` | `boolean` | NO | `false` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `manager_id` | `uuid` | YES | `-` | users.id |
| `inventory_account_id` | `uuid` | YES | `-` | chart_of_accounts.id |
| `rak_account_id` | `uuid` | YES | `-` | chart_of_accounts.id |

#### Tabel `user_outlets`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `user_id` | `uuid` | NO | `-` | users.id |
| `outlet_id` | `uuid` | NO | `-` | outlets.id |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

#### Tabel `outlet_supervisors`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `outlet_id` | `uuid` | NO | `-` | outlets.id |
| `user_id` | `uuid` | NO | `-` | users.id |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |

### 2. Master Data Produk & Katalog

#### Tabel `categories`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `parent_id` | `uuid` | YES | `-` | categories.id |
| `code` | `character varying` | NO | `-` | - |
| `name` | `character varying` | NO | `-` | - |
| `is_active` | `boolean` | NO | `true` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `brands`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `code` | `character varying` | NO | `-` | - |
| `name` | `character varying` | NO | `-` | - |
| `description` | `text` | YES | `-` | - |
| `is_active` | `boolean` | NO | `true` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `units`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `code` | `character varying` | NO | `-` | - |
| `name` | `character varying` | NO | `-` | - |
| `is_fractional` | `boolean` | NO | `false` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `products`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `category_id` | `uuid` | NO | `-` | categories.id |
| `base_unit_id` | `uuid` | NO | `-` | units.id |
| `sku` | `character varying` | NO | `-` | - |
| `barcode` | `character varying` | NO | `-` | - |
| `name` | `character varying` | NO | `-` | - |
| `brand_id` | `uuid` | YES | `-` | brands.id |
| `costing_method` | `character varying` | NO | `'GLOBAL'::character varying` | - |
| `cost_price` | `numeric` | NO | `0.0000` | - |
| `base_selling_price` | `numeric` | NO | `-` | - |
| `min_stock` | `numeric` | NO | `0.0000` | - |
| `max_stock` | `numeric` | NO | `0.0000` | - |
| `tax_percent` | `numeric` | NO | `11.00` | - |
| `is_weight_embedded` | `boolean` | NO | `false` | - |
| `is_active` | `boolean` | NO | `true` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `image_url` | `text` | YES | `-` | - |
| `auto_adjust_selling_price` | `boolean` | NO | `false` | - |
| `markup_percent` | `numeric` | YES | `NULL::numeric` | - |
| `primary_supplier_id` | `uuid` | YES | `-` | suppliers.id |

#### Tabel `product_units`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `product_id` | `uuid` | NO | `-` | products.id |
| `unit_id` | `uuid` | NO | `-` | units.id |
| `conversion_factor` | `numeric` | NO | `-` | - |
| `selling_price` | `numeric` | NO | `-` | - |
| `barcode` | `character varying` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `product_barcodes`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `product_id` | `uuid` | NO | `-` | products.id |
| `barcode` | `character varying` | NO | `-` | - |
| `description` | `character varying` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `product_price_tiers`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `product_id` | `uuid` | NO | `-` | products.id |
| `min_quantity` | `numeric` | NO | `-` | - |
| `tier_price` | `numeric` | NO | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `product_outlet_prices`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `outlet_id` | `uuid` | NO | `-` | outlets.id |
| `product_id` | `uuid` | NO | `-` | products.id |
| `cost_price` | `numeric` | NO | `0.0000` | - |
| `base_selling_price` | `numeric` | NO | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `last_purchase_price` | `numeric` | YES | `NULL::numeric` | - |

### 3. Manajemen Stok & Inventori

#### Tabel `warehouse_stocks`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `warehouse_id` | `uuid` | NO | `-` | outlets.id |
| `product_id` | `uuid` | NO | `-` | products.id |
| `quantity` | `numeric` | NO | `0.0000` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `stock_mutations`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `bigint` | NO | `nextval('stock_mutations_id...` | - |
| `warehouse_id` | `uuid` | NO | `-` | outlets.id |
| `product_id` | `uuid` | NO | `-` | products.id |
| `reference_type` | `character varying` | NO | `-` | - |
| `reference_id` | `uuid` | NO | `-` | - |
| `qty_in` | `numeric` | NO | `0.0000` | - |
| `qty_out` | `numeric` | NO | `0.0000` | - |
| `balance` | `numeric` | NO | `-` | - |
| `cost_price` | `numeric` | NO | `-` | - |
| `notes` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `stock_batches`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `warehouse_id` | `uuid` | NO | `-` | outlets.id |
| `product_id` | `uuid` | NO | `-` | products.id |
| `batch_number` | `character varying` | NO | `-` | - |
| `received_date` | `date` | NO | `-` | - |
| `expiry_date` | `date` | YES | `-` | - |
| `original_qty` | `numeric` | NO | `-` | - |
| `remaining_qty` | `numeric` | NO | `-` | - |
| `unit_cost` | `numeric` | NO | `-` | - |
| `status` | `character varying` | NO | `'ACTIVE'::character varying` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `product_stock_batches`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `product_id` | `uuid` | NO | `-` | products.id |
| `outlet_id` | `uuid` | NO | `-` | outlets.id |
| `batch_number` | `character varying` | NO | `-` | - |
| `initial_qty` | `numeric` | NO | `-` | - |
| `remaining_qty` | `numeric` | NO | `-` | - |
| `cost_price` | `numeric` | NO | `-` | - |
| `reference_type` | `character varying` | NO | `-` | - |
| `reference_id` | `uuid` | YES | `-` | - |
| `received_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `expiry_date` | `date` | YES | `-` | - |
| `source_document_number` | `character varying` | YES | `-` | - |

#### Tabel `stock_transfers`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `transfer_number` | `character varying` | NO | `-` | - |
| `from_outlet_id` | `uuid` | NO | `-` | outlets.id |
| `to_outlet_id` | `uuid` | NO | `-` | outlets.id |
| `transfer_date` | `date` | NO | `-` | - |
| `status` | `character varying` | NO | `'COMPLETED'::character varying` | - |
| `notes` | `text` | YES | `-` | - |
| `total_amount` | `numeric` | NO | `0.0000` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `received_date` | `date` | YES | `-` | - |
| `received_by` | `uuid` | YES | `-` | users.id |
| `received_at` | `timestamp with time zone` | YES | `-` | - |
| `shipped_at` | `timestamp with time zone` | YES | `-` | - |
| `cancellation_reason` | `text` | YES | `-` | - |
| `cancelled_by` | `uuid` | YES | `-` | users.id |
| `cancelled_at` | `timestamp with time zone` | YES | `-` | - |

#### Tabel `stock_transfer_items`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `transfer_id` | `uuid` | NO | `-` | stock_transfers.id |
| `product_id` | `uuid` | NO | `-` | products.id |
| `quantity` | `numeric` | NO | `-` | - |
| `cost_price` | `numeric` | NO | `-` | - |
| `subtotal` | `numeric` | NO | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `received_quantity` | `numeric` | YES | `-` | - |
| `notes` | `text` | YES | `-` | - |

#### Tabel `stock_adjustments`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `adjustment_number` | `character varying` | NO | `-` | - |
| `warehouse_id` | `uuid` | NO | `-` | outlets.id |
| `adjustment_date` | `date` | NO | `-` | - |
| `reason_id` | `uuid` | NO | `-` | reason_codes.id |
| `status` | `character varying` | NO | `'DRAFT'::character varying` | - |
| `approved_by` | `uuid` | YES | `-` | users.id |
| `notes` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `stock_adjustment_items`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `adjustment_id` | `uuid` | NO | `-` | stock_adjustments.id |
| `product_id` | `uuid` | NO | `-` | products.id |
| `system_qty` | `numeric` | NO | `-` | - |
| `actual_qty` | `numeric` | NO | `-` | - |
| `discrepancy_qty` | `numeric` | NO | `-` | - |
| `cost_price` | `numeric` | NO | `-` | - |
| `total_cost_discrepancy` | `numeric` | NO | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `inventory_costing_settings`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `default_costing_method` | `character varying` | NO | `'WMA'::character varying` | - |
| `default_auto_adjust_selling_price` | `boolean` | NO | `false` | - |
| `default_markup_percent` | `numeric` | YES | `0` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

### 4. Pengadaan & Pembelian (Procurement)

#### Tabel `suppliers`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `code` | `character varying` | NO | `-` | - |
| `name` | `character varying` | NO | `-` | - |
| `contact_person` | `character varying` | YES | `-` | - |
| `phone` | `character varying` | YES | `-` | - |
| `address` | `text` | YES | `-` | - |
| `term_of_payment_days` | `integer` | NO | `0` | - |
| `is_active` | `boolean` | NO | `true` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `purchase_orders`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `po_number` | `character varying` | NO | `-` | - |
| `supplier_id` | `uuid` | NO | `-` | suppliers.id |
| `warehouse_id` | `uuid` | YES | `-` | outlets.id |
| `order_date` | `date` | NO | `-` | - |
| `expected_delivery` | `date` | YES | `-` | - |
| `total_amount` | `numeric` | NO | `0.0000` | - |
| `status` | `character varying` | NO | `'PENDING'::character varying` | - |
| `notes` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `outlet_id` | `uuid` | YES | `-` | outlets.id |

#### Tabel `purchase_order_items`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `po_id` | `uuid` | NO | `-` | purchase_orders.id |
| `product_id` | `uuid` | NO | `-` | products.id |
| `ordered_qty` | `numeric` | NO | `-` | - |
| `unit_price` | `numeric` | NO | `-` | - |
| `subtotal` | `numeric` | NO | `-` | - |
| `received_qty` | `numeric` | NO | `0.0000` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `goods_receipts`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `grn_number` | `character varying` | NO | `-` | - |
| `po_id` | `uuid` | NO | `-` | purchase_orders.id |
| `supplier_id` | `uuid` | NO | `-` | suppliers.id |
| `outlet_id` | `uuid` | YES | `-` | outlets.id |
| `delivery_order_number` | `character varying` | NO | `-` | - |
| `receive_date` | `date` | NO | `-` | - |
| `status` | `character varying` | NO | `'UNINVOICED'::character var...` | - |
| `notes` | `text` | YES | `-` | - |
| `total_received_qty` | `numeric` | NO | `0.0000` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `goods_receipt_items`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `goods_receipt_id` | `uuid` | NO | `-` | goods_receipts.id |
| `po_item_id` | `uuid` | YES | `-` | purchase_order_items.id |
| `product_id` | `uuid` | NO | `-` | products.id |
| `ordered_qty` | `numeric` | NO | `0.0000` | - |
| `received_qty` | `numeric` | NO | `-` | - |
| `unit_price` | `numeric` | NO | `0.0000` | - |
| `invoiced_qty` | `numeric` | NO | `0.0000` | - |
| `subtotal` | `numeric` | NO | `0.0000` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `expiry_date` | `date` | YES | `-` | - |
| `batch_number` | `character varying` | YES | `-` | - |

#### Tabel `purchase_invoices`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `invoice_number` | `character varying` | NO | `-` | - |
| `supplier_invoice_number` | `character varying` | NO | `-` | - |
| `po_id` | `uuid` | YES | `-` | purchase_orders.id |
| `supplier_id` | `uuid` | NO | `-` | suppliers.id |
| `warehouse_id` | `uuid` | YES | `-` | outlets.id |
| `invoice_date` | `date` | NO | `-` | - |
| `due_date` | `date` | NO | `-` | - |
| `subtotal` | `numeric` | NO | `-` | - |
| `tax_amount` | `numeric` | NO | `0.0000` | - |
| `discount_amount` | `numeric` | NO | `0.0000` | - |
| `total_amount` | `numeric` | NO | `-` | - |
| `paid_amount` | `numeric` | NO | `0.0000` | - |
| `payment_status` | `character varying` | NO | `'UNPAID'::character varying` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `outlet_id` | `uuid` | YES | `-` | outlets.id |
| `goods_receipt_id` | `uuid` | YES | `-` | goods_receipts.id |

#### Tabel `purchase_invoice_items`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `invoice_id` | `uuid` | NO | `-` | purchase_invoices.id |
| `product_id` | `uuid` | NO | `-` | products.id |
| `received_qty` | `numeric` | NO | `-` | - |
| `unit_price` | `numeric` | NO | `-` | - |
| `discount_amount` | `numeric` | NO | `0.0000` | - |
| `subtotal` | `numeric` | NO | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `ordered_qty` | `numeric` | NO | `0.0000` | - |
| `goods_receipt_item_id` | `uuid` | YES | `-` | goods_receipt_items.id |

#### Tabel `purchase_returns`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `return_number` | `character varying` | NO | `-` | - |
| `supplier_id` | `uuid` | NO | `-` | suppliers.id |
| `outlet_id` | `uuid` | YES | `-` | outlets.id |
| `warehouse_id` | `uuid` | YES | `-` | outlets.id |
| `invoice_id` | `uuid` | YES | `-` | purchase_invoices.id |
| `goods_receipt_id` | `uuid` | YES | `-` | goods_receipts.id |
| `return_date` | `date` | NO | `-` | - |
| `status` | `character varying` | NO | `'COMPLETED'::character varying` | - |
| `refund_method` | `character varying` | NO | `'DEBT_REDUCTION'::character...` | - |
| `reason` | `text` | NO | `-` | - |
| `subtotal` | `numeric` | NO | `0.0000` | - |
| `tax_amount` | `numeric` | NO | `0.0000` | - |
| `total_amount` | `numeric` | NO | `0.0000` | - |
| `notes` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `purchase_return_items`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `purchase_return_id` | `uuid` | NO | `-` | purchase_returns.id |
| `product_id` | `uuid` | NO | `-` | products.id |
| `returned_qty` | `numeric` | NO | `-` | - |
| `unit_price` | `numeric` | NO | `0.0000` | - |
| `subtotal` | `numeric` | NO | `0.0000` | - |
| `notes` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

### 5. Penjualan & Kasir (POS)

#### Tabel `sales`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `invoice_number` | `character varying` | NO | `-` | - |
| `shift_id` | `uuid` | NO | `-` | cashier_shifts.id |
| `customer_id` | `uuid` | YES | `-` | customers.id |
| `warehouse_id` | `uuid` | YES | `-` | outlets.id |
| `voucher_id` | `uuid` | YES | `-` | vouchers.id |
| `subtotal` | `numeric` | NO | `-` | - |
| `discount_amount` | `numeric` | NO | `0.0000` | - |
| `tax_amount` | `numeric` | NO | `0.0000` | - |
| `total_amount` | `numeric` | NO | `-` | - |
| `paid_amount` | `numeric` | NO | `-` | - |
| `change_amount` | `numeric` | NO | `-` | - |
| `total_cost_amount` | `numeric` | NO | `-` | - |
| `status` | `character varying` | NO | `'COMPLETED'::character varying` | - |
| `notes` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `outlet_id` | `uuid` | NO | `-` | outlets.id |
| `total_flash_sale_discount` | `numeric` | NO | `0.0000` | - |
| `total_member_discount` | `numeric` | NO | `0.0000` | - |
| `total_savings_amount` | `numeric` | NO | `0.0000` | - |
| `rounding_amount` | `numeric` | NO | `0.0000` | - |
| `return_status` | `character varying` | NO | `'NONE'::character varying` | - |
| `total_refund_amount` | `numeric` | NO | `0.0000` | - |

#### Tabel `sale_items`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `sale_id` | `uuid` | NO | `-` | sales.id |
| `product_id` | `uuid` | NO | `-` | products.id |
| `unit_id` | `uuid` | NO | `-` | units.id |
| `quantity` | `numeric` | NO | `-` | - |
| `unit_cost_price` | `numeric` | NO | `-` | - |
| `unit_selling_price` | `numeric` | NO | `-` | - |
| `discount_percent` | `numeric` | NO | `0.00` | - |
| `discount_amount` | `numeric` | NO | `0.0000` | - |
| `tax_amount` | `numeric` | NO | `0.0000` | - |
| `subtotal` | `numeric` | NO | `-` | - |
| `is_voided` | `boolean` | NO | `false` | - |
| `void_reason_id` | `uuid` | YES | `-` | reason_codes.id |
| `void_authorized_by` | `uuid` | YES | `-` | users.id |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `flash_sale_id` | `uuid` | YES | `-` | flash_sales.id |
| `original_price` | `numeric` | YES | `-` | - |
| `flash_sale_discount` | `numeric` | NO | `0.0000` | - |
| `member_discount` | `numeric` | NO | `0.0000` | - |

#### Tabel `sale_payments`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `sale_id` | `uuid` | NO | `-` | sales.id |
| `payment_method_id` | `uuid` | NO | `-` | payment_methods.id |
| `amount` | `numeric` | NO | `-` | - |
| `card_number_masked` | `character varying` | YES | `-` | - |
| `reference_number` | `character varying` | YES | `-` | - |
| `payment_proof_url` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `cashier_shifts`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `terminal_id` | `character varying` | NO | `-` | - |
| `user_id` | `uuid` | NO | `-` | users.id |
| `opened_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `closed_at` | `timestamp with time zone` | YES | `-` | - |
| `opening_cash` | `numeric` | NO | `-` | - |
| `expected_cash` | `numeric` | YES | `-` | - |
| `actual_cash` | `numeric` | YES | `-` | - |
| `discrepancy` | `numeric` | YES | `-` | - |
| `status` | `character varying` | NO | `'OPEN'::character varying` | - |
| `supervisor_approval_id` | `uuid` | YES | `-` | users.id |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `outlet_id` | `uuid` | YES | `-` | outlets.id |

#### Tabel `cash_drops`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `drop_number` | `character varying` | NO | `-` | - |
| `shift_id` | `uuid` | NO | `-` | cashier_shifts.id |
| `outlet_id` | `uuid` | YES | `-` | outlets.id |
| `cashier_id` | `uuid` | NO | `-` | users.id |
| `supervisor_id` | `uuid` | NO | `-` | users.id |
| `amount` | `numeric` | NO | `-` | - |
| `notes` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

#### Tabel `sales_returns`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `return_number` | `character varying` | NO | `-` | - |
| `sale_id` | `uuid` | NO | `-` | sales.id |
| `outlet_id` | `uuid` | YES | `-` | outlets.id |
| `shift_id` | `uuid` | YES | `-` | cashier_shifts.id |
| `cashier_id` | `uuid` | NO | `-` | users.id |
| `supervisor_id` | `uuid` | NO | `-` | users.id |
| `return_type` | `character varying` | NO | `'REFUND'::character varying` | - |
| `total_refund_amount` | `numeric` | NO | `0.0000` | - |
| `reason_id` | `uuid` | YES | `-` | reason_codes.id |
| `notes` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `refund_method` | `character varying` | NO | `'CASH'::character varying` | - |
| `refund_amount` | `numeric` | NO | `0.0000` | - |

#### Tabel `sales_return_items`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `sales_return_id` | `uuid` | NO | `-` | sales_returns.id |
| `sale_item_id` | `uuid` | NO | `-` | sale_items.id |
| `product_id` | `uuid` | NO | `-` | products.id |
| `quantity` | `numeric` | NO | `-` | - |
| `unit_price` | `numeric` | NO | `-` | - |
| `discount_amount` | `numeric` | NO | `0.0000` | - |
| `subtotal` | `numeric` | NO | `-` | - |
| `condition` | `character varying` | NO | `'GOOD'::character varying` | - |
| `restock` | `boolean` | NO | `true` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

#### Tabel `reason_codes`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `type` | `character varying` | NO | `-` | - |
| `code` | `character varying` | NO | `-` | - |
| `description` | `text` | NO | `-` | - |
| `is_active` | `boolean` | NO | `true` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `payment_methods`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `code` | `character varying` | NO | `-` | - |
| `name` | `character varying` | NO | `-` | - |
| `category` | `character varying` | NO | `-` | - |
| `is_active` | `boolean` | NO | `true` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `receipt_settings`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `outlet_id` | `uuid` | YES | `-` | outlets.id |
| `store_name` | `character varying` | NO | `'NOTANE RETAIL STORE'::char...` | - |
| `store_tagline` | `character varying` | YES | `'Pusat Belanja Retail Moder...` | - |
| `store_address` | `text` | NO | `'Jl. Sudirman No. 123, Jaka...` | - |
| `store_phone` | `character varying` | NO | `'021-5551234 / 0812-3456-78...` | - |
| `store_website` | `character varying` | YES | `'www.notane.id'::character ...` | - |
| `tax_id` | `character varying` | YES | `'NPWP: 01.234.567.8-901.000...` | - |
| `header_note` | `text` | YES | `'Selamat Datang di Notane R...` | - |
| `footer_note_1` | `text` | YES | `'Terima Kasih Atas Kunjunga...` | - |
| `footer_note_2` | `text` | YES | `'Barang yang sudah dibeli t...` | - |
| `social_media` | `character varying` | YES | `'IG: @notaneretail • CS: 08...` | - |
| `show_cashier_name` | `boolean` | NO | `true` | - |
| `show_customer_info` | `boolean` | NO | `true` | - |
| `show_loyalty_points` | `boolean` | NO | `true` | - |
| `show_tax_info` | `boolean` | NO | `true` | - |
| `paper_width` | `character varying` | NO | `'58mm'::character varying` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `show_phone` | `boolean` | NO | `true` | - |
| `show_address` | `boolean` | NO | `true` | - |
| `show_barcode` | `boolean` | NO | `true` | - |
| `show_flash_sale_discount` | `boolean` | NO | `true` | - |
| `show_member_discount` | `boolean` | NO | `true` | - |
| `show_tax_summary` | `boolean` | NO | `true` | - |
| `show_savings_summary` | `boolean` | NO | `true` | - |
| `cash_rounding_mode` | `character varying` | NO | `'NONE'::character varying` | - |
| `max_cash_drawer_limit` | `numeric` | NO | `2000000.0000` | - |
| `store_logo_url` | `text` | YES | `-` | - |
| `pairing_code` | `character varying` | YES | `'NOTANE-8899'::character va...` | - |
| `supervisor_biometric_mode` | `character varying` | NO | `'OFF'::character varying` | - |
| `supervisor_biometric_policy` | `character varying` | NO | `'OPTIONAL'::character varying` | - |
| `face_match_threshold` | `numeric` | NO | `75.00` | - |
| `audit_log_retention_days` | `integer` | NO | `30` | - |
| `is_initial_setup_completed` | `boolean` | NO | `false` | - |
| `initial_setup_completed_at` | `timestamp with time zone` | YES | `-` | - |

#### Tabel `sale_item_stock_batches`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `sale_id` | `uuid` | NO | `-` | sales.id |
| `sale_item_id` | `uuid` | NO | `-` | sale_items.id |
| `product_id` | `uuid` | NO | `-` | products.id |
| `outlet_id` | `uuid` | NO | `-` | outlets.id |
| `batch_id` | `uuid` | YES | `-` | product_stock_batches.id |
| `batch_number` | `character varying` | NO | `-` | - |
| `quantity` | `numeric` | NO | `-` | - |
| `cost_price` | `numeric` | NO | `-` | - |
| `subtotal_cost` | `numeric` | NO | `-` | - |
| `source_reference_type` | `character varying` | NO | `-` | - |
| `source_reference_id` | `uuid` | YES | `-` | - |
| `source_document_number` | `character varying` | YES | `-` | - |
| `expiry_date` | `date` | YES | `-` | - |
| `allocation_method` | `character varying` | NO | `'FIFO'::character varying` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

### 6. Pelanggan, Loyalitas & Promosi

#### Tabel `customers`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `customer_group_id` | `integer` | YES | `-` | customer_groups.id |
| `code` | `character varying` | NO | `-` | - |
| `name` | `character varying` | NO | `-` | - |
| `phone` | `character varying` | YES | `-` | - |
| `email` | `character varying` | YES | `-` | - |
| `credit_limit` | `numeric` | NO | `0.0000` | - |
| `reward_points` | `integer` | NO | `0` | - |
| `is_active` | `boolean` | NO | `true` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `customer_groups`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `integer` | NO | `nextval('customer_groups_id...` | - |
| `name` | `character varying` | NO | `-` | - |
| `description` | `text` | YES | `-` | - |
| `is_active` | `boolean` | NO | `true` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `customer_group_prices`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `customer_group_id` | `integer` | NO | `-` | customer_groups.id |
| `product_id` | `uuid` | NO | `-` | products.id |
| `special_price` | `numeric` | NO | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `outlet_customer_group_prices`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `outlet_id` | `uuid` | NO | `-` | outlets.id |
| `customer_group_id` | `integer` | NO | `-` | customer_groups.id |
| `product_id` | `uuid` | NO | `-` | products.id |
| `special_price` | `numeric` | NO | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `customer_point_ledgers`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `bigint` | NO | `nextval('customer_point_led...` | - |
| `customer_id` | `uuid` | NO | `-` | customers.id |
| `transaction_type` | `character varying` | NO | `-` | - |
| `reference_id` | `uuid` | YES | `-` | - |
| `points_in` | `integer` | NO | `0` | - |
| `points_out` | `integer` | NO | `0` | - |
| `balance` | `integer` | NO | `-` | - |
| `notes` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |

#### Tabel `loyalty_settings`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `is_enabled` | `boolean` | NO | `true` | - |
| `point_value_rupiah` | `numeric` | NO | `100.00` | - |
| `min_spend_per_point` | `numeric` | NO | `10000.00` | - |
| `max_point_usage_percent` | `numeric` | NO | `50.00` | - |
| `min_redeem_points` | `integer` | NO | `1` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

#### Tabel `vouchers`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `code` | `character varying` | NO | `-` | - |
| `description` | `text` | YES | `-` | - |
| `discount_type` | `character varying` | NO | `-` | - |
| `discount_value` | `numeric` | NO | `-` | - |
| `min_purchase_amount` | `numeric` | NO | `0.0000` | - |
| `max_discount_amount` | `numeric` | YES | `-` | - |
| `valid_from` | `timestamp with time zone` | NO | `-` | - |
| `valid_until` | `timestamp with time zone` | NO | `-` | - |
| `status` | `character varying` | NO | `'ACTIVE'::character varying` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `quota` | `integer` | NO | `0` | - |
| `used_count` | `integer` | NO | `0` | - |

#### Tabel `promotions`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `code` | `character varying` | NO | `-` | - |
| `name` | `character varying` | NO | `-` | - |
| `promo_type` | `character varying` | NO | `-` | - |
| `start_date` | `timestamp with time zone` | NO | `-` | - |
| `end_date` | `timestamp with time zone` | NO | `-` | - |
| `is_active` | `boolean` | NO | `true` | - |
| `min_purchase_amount` | `numeric` | NO | `0.0000` | - |
| `outlet_id` | `uuid` | YES | `-` | outlets.id |
| `priority` | `integer` | NO | `1` | - |
| `description` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `promotion_rules`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `promotion_id` | `uuid` | NO | `-` | promotions.id |
| `buy_product_id` | `uuid` | YES | `-` | products.id |
| `buy_quantity` | `numeric` | NO | `1.0000` | - |
| `reward_product_id` | `uuid` | YES | `-` | products.id |
| `reward_quantity` | `numeric` | NO | `1.0000` | - |
| `reward_discount_percent` | `numeric` | NO | `100.00` | - |
| `reward_special_price` | `numeric` | YES | `-` | - |
| `bundle_discount_amount` | `numeric` | NO | `0.0000` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

#### Tabel `flash_sales`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `name` | `character varying` | NO | `-` | - |
| `description` | `text` | YES | `-` | - |
| `start_date` | `date` | NO | `-` | - |
| `end_date` | `date` | NO | `-` | - |
| `is_all_day` | `boolean` | NO | `true` | - |
| `start_time` | `time without time zone` | YES | `-` | - |
| `end_time` | `time without time zone` | YES | `-` | - |
| `is_active` | `boolean` | NO | `true` | - |
| `is_stackable_with_member` | `boolean` | NO | `false` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |

#### Tabel `flash_sale_items`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `flash_sale_id` | `uuid` | NO | `-` | flash_sales.id |
| `product_id` | `uuid` | NO | `-` | products.id |
| `discount_type` | `character varying` | NO | `'FIXED_PRICE'::character va...` | - |
| `discount_value` | `numeric` | NO | `-` | - |
| `max_qty_per_transaction` | `numeric` | YES | `-` | - |
| `promo_quota` | `numeric` | YES | `-` | - |
| `sold_qty` | `numeric` | NO | `0` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

#### Tabel `flash_sale_outlets`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `flash_sale_id` | `uuid` | NO | `-` | flash_sales.id |
| `outlet_id` | `uuid` | NO | `-` | outlets.id |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

### 7. Akuntansi & Keuangan

#### Tabel `account_categories`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `integer` | NO | `nextval('account_categories...` | - |
| `name` | `character varying` | NO | `-` | - |
| `normal_balance` | `character varying` | NO | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `chart_of_accounts`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `account_category_id` | `integer` | NO | `-` | account_categories.id |
| `parent_id` | `uuid` | YES | `-` | chart_of_accounts.id |
| `code` | `character varying` | NO | `-` | - |
| `name` | `character varying` | NO | `-` | - |
| `description` | `text` | YES | `-` | - |
| `is_header` | `boolean` | NO | `false` | - |
| `is_active` | `boolean` | NO | `true` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `is_system` | `boolean` | NO | `false` | - |

#### Tabel `journal_entries`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `journal_number` | `character varying` | NO | `-` | - |
| `journal_date` | `date` | NO | `-` | - |
| `reference_type` | `character varying` | NO | `-` | - |
| `reference_id` | `uuid` | YES | `-` | - |
| `description` | `text` | NO | `-` | - |
| `total_debit` | `numeric` | NO | `-` | - |
| `total_credit` | `numeric` | NO | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `is_closing_entry` | `boolean` | NO | `false` | - |

#### Tabel `journal_lines`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `journal_id` | `uuid` | NO | `-` | journal_entries.id |
| `account_id` | `uuid` | NO | `-` | chart_of_accounts.id |
| `debit` | `numeric` | NO | `0.0000` | - |
| `credit` | `numeric` | NO | `0.0000` | - |
| `memo` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `outlet_id` | `uuid` | YES | `-` | outlets.id |

#### Tabel `accounting_periods`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `name` | `character varying` | NO | `-` | - |
| `start_date` | `date` | NO | `-` | - |
| `end_date` | `date` | NO | `-` | - |
| `is_closed` | `boolean` | NO | `false` | - |
| `closed_at` | `timestamp with time zone` | YES | `-` | - |
| `closed_by` | `uuid` | YES | `-` | users.id |
| `reopened_at` | `timestamp with time zone` | YES | `-` | - |
| `reopened_by` | `uuid` | YES | `-` | users.id |
| `reopen_reason` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

#### Tabel `accounting_closings`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `closing_number` | `character varying` | NO | `-` | - |
| `period_start` | `date` | NO | `-` | - |
| `period_end` | `date` | NO | `-` | - |
| `closing_date` | `date` | NO | `-` | - |
| `outlet_id` | `uuid` | YES | `-` | outlets.id |
| `total_revenue` | `numeric` | NO | `0` | - |
| `total_expense` | `numeric` | NO | `0` | - |
| `net_income` | `numeric` | NO | `0` | - |
| `journal_entry_id` | `uuid` | YES | `-` | journal_entries.id |
| `status` | `character varying` | NO | `'CLOSED'::character varying` | - |
| `notes` | `text` | YES | `-` | - |
| `reopened_at` | `timestamp with time zone` | YES | `-` | - |
| `reopened_by` | `uuid` | YES | `-` | users.id |
| `reopen_reason` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |

#### Tabel `bank_accounts`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `bank_name` | `character varying` | NO | `-` | - |
| `account_number` | `character varying` | NO | `-` | - |
| `account_holder` | `character varying` | NO | `-` | - |
| `branch` | `character varying` | YES | `-` | - |
| `outlet_id` | `uuid` | YES | `-` | outlets.id |
| `coa_id` | `uuid` | YES | `-` | chart_of_accounts.id |
| `is_active` | `boolean` | NO | `true` | - |
| `is_default` | `boolean` | NO | `false` | - |
| `notes` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

#### Tabel `capital_deposits`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `deposit_no` | `character varying` | NO | `-` | - |
| `outlet_id` | `uuid` | YES | `-` | outlets.id |
| `deposit_date` | `date` | NO | `CURRENT_DATE` | - |
| `investor_name` | `character varying` | NO | `-` | - |
| `amount` | `numeric` | NO | `-` | - |
| `payment_account_id` | `uuid` | NO | `-` | chart_of_accounts.id |
| `equity_account_id` | `uuid` | NO | `-` | chart_of_accounts.id |
| `reference_no` | `character varying` | YES | `-` | - |
| `notes` | `text` | YES | `-` | - |
| `attachment_url` | `text` | YES | `-` | - |
| `journal_entry_id` | `uuid` | YES | `-` | journal_entries.id |
| `created_by` | `uuid` | YES | `-` | users.id |
| `cancelled_by` | `uuid` | YES | `-` | users.id |
| `cancelled_at` | `timestamp with time zone` | YES | `-` | - |
| `cancellation_reason` | `text` | YES | `-` | - |
| `status` | `character varying` | NO | `'COMPLETED'::character varying` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

#### Tabel `dividend_distributions`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `distribution_number` | `character varying` | NO | `-` | - |
| `closing_id` | `uuid` | NO | `-` | accounting_closings.id |
| `distribution_date` | `date` | NO | `-` | - |
| `total_amount` | `numeric` | NO | `-` | - |
| `retained_earnings_account_id` | `uuid` | NO | `-` | chart_of_accounts.id |
| `payment_account_id` | `uuid` | NO | `-` | chart_of_accounts.id |
| `journal_entry_id` | `uuid` | YES | `-` | journal_entries.id |
| `recipient_name` | `character varying` | NO | `-` | - |
| `notes` | `text` | YES | `-` | - |
| `recipients_data` | `jsonb` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |

#### Tabel `expense_categories`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `code` | `character varying` | NO | `-` | - |
| `name` | `character varying` | NO | `-` | - |
| `account_id` | `uuid` | YES | `-` | chart_of_accounts.id |
| `description` | `text` | YES | `-` | - |
| `is_active` | `boolean` | NO | `true` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

#### Tabel `expense_transactions`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `transaction_no` | `character varying` | NO | `-` | - |
| `outlet_id` | `uuid` | NO | `-` | outlets.id |
| `category_id` | `uuid` | YES | `-` | expense_categories.id |
| `expense_account_id` | `uuid` | NO | `-` | chart_of_accounts.id |
| `payment_account_id` | `uuid` | NO | `-` | chart_of_accounts.id |
| `amount` | `numeric` | NO | `-` | - |
| `transaction_date` | `date` | NO | `CURRENT_DATE` | - |
| `recipient_name` | `character varying` | YES | `-` | - |
| `notes` | `text` | YES | `-` | - |
| `reference_no` | `character varying` | YES | `-` | - |
| `attachment_url` | `text` | YES | `-` | - |
| `shift_id` | `uuid` | YES | `-` | cashier_shifts.id |
| `journal_entry_id` | `uuid` | YES | `-` | journal_entries.id |
| `created_by` | `uuid` | YES | `-` | users.id |
| `cancelled_by` | `uuid` | YES | `-` | users.id |
| `cancelled_at` | `timestamp with time zone` | YES | `-` | - |
| `cancellation_reason` | `text` | YES | `-` | - |
| `status` | `character varying` | NO | `'COMPLETED'::character varying` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

### 8. Sistem, Lisensi & Job Queue

#### Tabel `app_update_history`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `integer` | NO | `nextval('app_update_history...` | - |
| `from_version` | `character varying` | NO | `-` | - |
| `to_version` | `character varying` | NO | `-` | - |
| `release_tag` | `character varying` | YES | `-` | - |
| `checksum_sha256` | `character varying` | YES | `-` | - |
| `applied_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `status` | `character varying` | NO | `'SUCCESS'::character varying` | - |
| `notes` | `text` | YES | `-` | - |

#### Tabel `audit_logs`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `bigint` | NO | `nextval('audit_logs_id_seq'...` | - |
| `user_id` | `uuid` | YES | `-` | users.id |
| `authorized_by` | `uuid` | YES | `-` | users.id |
| `terminal_id` | `character varying` | YES | `'SYSTEM'::character varying` | - |
| `action` | `character varying` | NO | `-` | - |
| `entity_name` | `character varying` | NO | `-` | - |
| `entity_id` | `character varying` | YES | `-` | - |
| `old_values` | `jsonb` | YES | `-` | - |
| `new_values` | `jsonb` | YES | `-` | - |
| `ip_address` | `character varying` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `created_by` | `uuid` | YES | `-` | users.id |
| `updated_by` | `uuid` | YES | `-` | users.id |
| `http_method` | `character varying` | YES | `-` | - |
| `endpoint` | `text` | YES | `-` | - |
| `status_code` | `integer` | YES | `200` | - |
| `execution_time_ms` | `integer` | YES | `0` | - |
| `user_agent` | `text` | YES | `-` | - |
| `user_name` | `character varying` | YES | `-` | - |
| `role_name` | `character varying` | YES | `-` | - |

#### Tabel `export_jobs`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `uuid` | NO | `gen_random_uuid()` | - |
| `user_id` | `uuid` | YES | `-` | users.id |
| `outlet_id` | `uuid` | YES | `-` | outlets.id |
| `module` | `character varying` | NO | `-` | - |
| `title` | `character varying` | NO | `-` | - |
| `filter_params` | `jsonb` | NO | `'{}'::jsonb` | - |
| `filename` | `character varying` | NO | `-` | - |
| `file_path` | `text` | YES | `-` | - |
| `file_size_bytes` | `bigint` | YES | `0` | - |
| `total_rows` | `integer` | YES | `0` | - |
| `processed_rows` | `integer` | YES | `0` | - |
| `status` | `character varying` | NO | `'QUEUED'::character varying` | - |
| `error_message` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `started_at` | `timestamp with time zone` | YES | `-` | - |
| `completed_at` | `timestamp with time zone` | YES | `-` | - |

#### Tabel `license_encrypted_cache`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `integer` | NO | `1` | - |
| `encrypted_vault` | `bytea` | NO | `-` | - |
| `hardware_id` | `character varying` | NO | `-` | - |
| `license_key` | `character varying` | YES | `-` | - |
| `last_monotonic_time` | `bigint` | NO | `0` | - |
| `activated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `last_sync_at` | `timestamp with time zone` | YES | `-` | - |
| `last_sync_status` | `character varying` | NO | `'INITIALIZED'::character va...` | - |
| `consecutive_failures` | `integer` | NO | `0` | - |
| `last_failure_reason` | `text` | YES | `-` | - |
| `last_failed_sync_at` | `timestamp with time zone` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |
| `updated_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

#### Tabel `license_sync_logs`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `bigint` | NO | `nextval('license_sync_logs_...` | - |
| `sync_type` | `character varying` | NO | `-` | - |
| `status` | `character varying` | NO | `-` | - |
| `http_status` | `integer` | YES | `-` | - |
| `latency_ms` | `integer` | YES | `-` | - |
| `provider_url` | `text` | NO | `-` | - |
| `message` | `text` | YES | `-` | - |
| `created_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

#### Tabel `_migrations`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `integer` | NO | `nextval('_migrations_id_seq...` | - |
| `migration` | `character varying` | NO | `-` | - |
| `batch` | `integer` | NO | `-` | - |
| `applied_at` | `timestamp with time zone` | NO | `CURRENT_TIMESTAMP` | - |

---

## 3. KAMUS SKEMA DATABASE NOTANE-MANAGER (SQLITE / PROVIDER PG)

Database `notane-manager` digunakan untuk manajemen lisensi, aktivasi perangkat tenant, rilis pembaruan biner, dan multi-tenant RBAC.

#### Tabel `provider_users`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `TEXT` | NO | `PK` | - |
| `username` | `TEXT` | NO | `UNIQUE` | Username login manager |
| `full_name` | `TEXT` | NO | `-` | Nama lengkap admin |
| `password_hash` | `TEXT` | NO | `-` | Argon2 / SHA hash |
| `quick_pin` | `TEXT` | NO | `-` | PIN otorisasi cepat |
| `role` | `TEXT` | NO | `'super_admin'` | Role manager |
| `last_login_at` | `TEXT` | YES | `-` | Waktu login terakhir |
| `created_at` | `TEXT` | NO | `-` | Timestamp ISO8601 |
| `updated_at` | `TEXT` | NO | `-` | Timestamp ISO8601 |

#### Tabel `roles`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `TEXT` | NO | `PK` | - |
| `name` | `TEXT` | NO | `UNIQUE` | Nama role (super_admin, operator) |
| `description` | `TEXT` | YES | `-` | Deskripsi fungsi role |
| `created_at` | `TEXT` | NO | `-` | Timestamp ISO8601 |

#### Tabel `permissions`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `TEXT` | NO | `PK` | - |
| `slug` | `TEXT` | NO | `UNIQUE` | Format: <module>:<action> (misal: license:create) |
| `name` | `TEXT` | NO | `-` | Nama tampilan permission |
| `module` | `TEXT` | NO | `-` | Modul (client, license, release) |
| `description` | `TEXT` | YES | `-` | Keterangan izin |
| `created_at` | `TEXT` | NO | `-` | Timestamp ISO8601 |

#### Tabel `role_permissions`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `role_id` | `TEXT` | NO | `PK` | roles.id (ON DELETE CASCADE) |
| `permission_id` | `TEXT` | NO | `PK` | permissions.id (ON DELETE CASCADE) |

#### Tabel `provider_clients`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `TEXT` | NO | `PK` | - |
| `client_name` | `TEXT` | NO | `-` | Nama penanggung jawab klien |
| `company_name` | `TEXT` | NO | `-` | Nama badan usaha / toko ritel tenant |
| `email` | `TEXT` | NO | `UNIQUE` | Email resmi tenant |
| `phone` | `TEXT` | YES | `-` | Nomor telepon / WhatsApp |
| `address` | `TEXT` | YES | `-` | Alamat toko / kantor |
| `created_at` | `TEXT` | NO | `-` | Timestamp ISO8601 |
| `updated_at` | `TEXT` | NO | `-` | Timestamp ISO8601 |

#### Tabel `provider_licenses`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `TEXT` | NO | `PK` | - |
| `license_key` | `TEXT` | NO | `UNIQUE` | Format NOTANE-XXXX-XXXX-XXXX |
| `client_id` | `TEXT` | NO | `-` | provider_clients.id (ON DELETE CASCADE) |
| `tier` | `TEXT` | NO | `'ENTERPRISE'` | Tingkat lisensi: BASIC, PRO, ENTERPRISE |
| `max_terminals` | `INTEGER` | NO | `5` | Batas jumlah terminal POS terhubung |
| `status` | `TEXT` | NO | `'ACTIVE'` | ACTIVE, SUSPENDED, EXPIRED, REVOKED |
| `is_claimed` | `INTEGER` | NO | `0` | 1 jika sudah di-binding ke server tertentu |
| `claimed_hardware_id` | `TEXT` | YES | `-` | Hardware UUID server yang mengklaim |
| `claimed_device_name` | `TEXT` | YES | `-` | Nama hostname mesin server |
| `claimed_at` | `TEXT` | YES | `-` | Waktu aktivasi klaim |
| `issued_at` | `TEXT` | NO | `-` | Tanggal penerbitan lisensi |
| `expires_at` | `TEXT` | NO | `-` | Tanggal kedaluwarsa lisensi |
| `grace_period_days` | `INTEGER` | NO | `7` | Masa tenggang offline (hari) |
| `features` | `TEXT` | NO | `-` | JSON array fitur aktif (e.g. ['pos','inventory']) |
| `notes` | `TEXT` | YES | `-` | Catatan operasional |
| `created_at` | `TEXT` | NO | `-` | Timestamp ISO8601 |
| `updated_at` | `TEXT` | NO | `-` | Timestamp ISO8601 |

#### Tabel `provider_device_activations`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `TEXT` | NO | `PK` | - |
| `license_id` | `TEXT` | NO | `-` | provider_licenses.id (ON DELETE CASCADE) |
| `hardware_id` | `TEXT` | NO | `-` | Hardware UUID terminal/server |
| `device_name` | `TEXT` | YES | `-` | Hostname / nama komputer |
| `ip_address` | `TEXT` | YES | `-` | Alamat IP perangkat |
| `os_info` | `TEXT` | YES | `-` | Info sistem operasi |
| `app_version` | `TEXT` | YES | `-` | Versi aplikasi saat aktivasi |
| `activated_at` | `TEXT` | NO | `-` | Waktu aktivasi awal |
| `last_ping_at` | `TEXT` | NO | `-` | Waktu heartbeat/ping terakhir |
| `is_active` | `INTEGER` | NO | `1` | 1 = aktif, 0 = nonaktif/revoked |

#### Tabel `provider_app_releases`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `TEXT` | NO | `PK` | - |
| `version` | `TEXT` | NO | `UNIQUE` | Semver format (e.g. 1.5.9) |
| `release_name` | `TEXT` | NO | `-` | Judul rilis pembaruan |
| `changelog` | `TEXT` | NO | `-` | Catatan perubahan / rilis |
| `release_date` | `TEXT` | NO | `-` | Tanggal rilis publikasi |
| `download_url` | `TEXT` | NO | `-` | URL unduhan installer / package |
| `checksum_sha256` | `TEXT` | NO | `-` | Checksum integritas SHA-256 |
| `signature_ed25519` | `TEXT` | NO | `-` | Tanda tangan kriptografi Ed25519 |
| `is_mandatory` | `INTEGER` | NO | `0` | 1 jika pembaruan wajib |
| `min_license_tier` | `TEXT` | NO | `'BASIC'` | Tier minimum yang berhak update |
| `has_db_migration` | `INTEGER` | NO | `1` | 1 jika rilis memuat migrasi skema |
| `is_published` | `INTEGER` | NO | `1` | 1 jika sudah tayang di publik |
| `component` | `TEXT` | NO | `'server'` | 'server' atau 'client' |
| `min_server_version` | `TEXT` | YES | `-` | Versi server minimum (jika client) |
| `created_at` | `TEXT` | NO | `-` | Timestamp ISO8601 |

#### Tabel `provider_license_audit_logs`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `id` | `TEXT` | NO | `PK` | - |
| `license_key` | `TEXT` | YES | `-` | Kunci lisensi terkait |
| `hardware_id` | `TEXT` | YES | `-` | Hardware ID terkait |
| `action` | `TEXT` | NO | `-` | CLAIM, VALIDATE, REVOKE, HEARTBEAT |
| `ip_address` | `TEXT` | YES | `-` | IP sumber permintaan |
| `status` | `TEXT` | NO | `-` | SUCCESS, REJECTED, EXPIRED |
| `details` | `TEXT` | YES | `-` | JSON rincian respons |
| `created_at` | `TEXT` | NO | `-` | Timestamp ISO8601 |

#### Tabel `provider_settings`
| Kolom | Tipe Data | Nullable | Default | Relasi / Keterangan |
|---|---|---|---|---|
| `key` | `TEXT` | NO | `PK` | Kunci konfigurasi (misal: ed25519_public_key) |
| `value` | `TEXT` | NO | `-` | Nilai konfigurasi |
