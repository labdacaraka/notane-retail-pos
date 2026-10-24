# 24 - Harga Member per Outlet (Outlet Customer Group Prices)

**Tanggal**: 2026-09-25  
**Status**: ✅ SELESAI  
**Caveman Mode**: full

---

## Problem

Harga member (`customer_group_prices`) bersifat global — sama di semua outlet. Padahal setiap outlet bisa memiliki harga member yang berbeda.

## Solusi

Tambah tabel `outlet_customer_group_prices` dengan prioritas lookup:

1. **Harga Member+Outlet** (`outlet_customer_group_prices`) ← prioritas tertinggi
2. **Harga Member Global** (`customer_group_prices`) ← fallback
3. **Harga Dasar Outlet** (`product_outlet_prices`) ← jika bukan member
4. **Harga Dasar Produk** (`products.base_selling_price`) ← fallback terakhir

## Files Changed

| File | Perubahan |
|------|-----------|
| `migrations/011_outlet_customer_group_prices.sql` | **[BARU]** Tabel baru + index |
| `src/models/product.rs` | Tambah struct `OutletCustomerGroupPriceDetail` + field di `ProductDetail` |
| `src/services/pos_service.rs` | Import model baru, query `outlet_customer_group_prices` di `lookup_product`, return di `ProductDetail` |
| `src/api/master.rs` | 3 handler baru: GET/POST/DELETE `/products/{id}/outlet-customer-group-prices` |
| `src/types/index.ts` | Tambah `OutletCustomerGroupPriceDetail` interface + field di `ProductDetail` |
| `src/stores/cartStore.ts` | Update `getBasePriceForCustomer()` — cek outlet+member price sebelum global member price |
| `src/pages/ProductsPage.vue` | Tambah state, functions, tombol "Member/Outlet", dan modal UI |

## API Endpoints Baru

- `GET /api/v1/products/{id}/outlet-customer-group-prices?outlet_id={uuid}` — list harga
- `POST /api/v1/products/{id}/outlet-customer-group-prices` — upsert (ON CONFLICT DO UPDATE)
- `DELETE /api/v1/products/outlet-customer-group-prices/{id}` — hapus override

## Build Status

- ✅ Frontend: `built in 5.34s` (0 error)
- ✅ Backend: `Finished dev profile` (0 error, 0 warning)
